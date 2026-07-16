//! 엔진 격리 좌석 — vt100 을 만지는 유일한 모듈. 미러(복원 직렬화기)는 여기가 내놓는
//! 엔진-중립 뷰(스칼라 상태 + [`GridCell`] 행 읽기)만 쓴다. 판정자는 별도 엔진 위에 있고
//! (계약 kit) 이 크레이트에 없다 — 그래서 교차 엔진 오라클이 성립한다: vt100 이 만든 페인트를
//! 독립 렌더러가 채점한다(양쪽 잠복 버그가 서로 뒤에 못 숨는다).
//!
//! 엔진-중립 타입([`ColorSnap`]·[`ModeSnap`]·[`GridCell`])은 직렬화기가 그리드를 읽는 창이다.
//! [`Engine`] 만 vt100 이다.
//!
//! vt100 이 흡수한 엔진 차이(엔진-중립 면의 시그니처는 계약이 고정한다):
//!   - 응답 포획: vt100 은 PTY 에 응답 바이트를 절대 만들지 않는다(무응답이 설계). 그래서
//!     삼킴은 공짜다 — 응답 경로를 가로챌 필요가 없다. 대신 삼킨 질의(DA1/DA2/DSR/OSC)는
//!     [`Callbacks`] 의 unhandled_csi/unhandled_osc 로 흘러오므로 [`VtCallbacks`] 가 계수한다.
//!   - private mode 읽기: vt100 은 일부 private mode(bracketed/app-cursor/app-keypad/mouse/
//!     show-cursor)에 public getter 가 있고, focus(1004)·alternate-scroll(1007)·auto-wrap(7)·
//!     insert(4)는 없다(decset 이 그것들을 unhandled 로 흘린다). getter 있는 것은 screen 에서
//!     직접 읽고, 없는 것은 같은 unhandled_csi 스트림을 관찰해 [`VtCallbacks`] 로 재구성한다.
//!   - 스크롤백 읽기: vt100 은 랜덤 접근 대신 `set_scrollback(offset)` 로 뷰를 위로 민다 —
//!     스크롤백 행은 오프셋을 옮겨 위치 0 에서 읽고, 읽고 나서 오프셋을 0 으로 되돌린다(읽기
//!     후 관측 상태 불변이라 grid 읽기는 논리적으로 const → [`Engine`] 은 [`std::cell::RefCell`]
//!     로 오프셋을 transient 읽기 커서로 다룬다).
//!   - 그리드 폭: vt100 은 wide 문자를 본체 셀 + 연속 셀 2칸으로 담는다(계약 정규형의
//!     본체+스페이서와 동형). 연속 셀은 spacer 로, 마지막 칸의 `row_wrapped` 는 wrapline 로.
//!   - strikethrough/hidden: vt100 셀은 이 두 속성을 노출하지 않는다 — 항상 false 로 둔다
//!     (엔진이 추적하지 않는 값을 지어내지 않는다).

use std::cell::RefCell;

use vt100::{Callbacks, Color, MouseProtocolEncoding, MouseProtocolMode, Parser, Screen as VtScreen};

/// 엔진이 유지하는 스크롤백 행 수. 바이트 충실 복원의 바닥 — 전체 의미 이력은
/// command_blocks(app.data)가 소유하고, 이 수치는 화면 재현용 창이다.
pub const MIRROR_SCROLLBACK_LINES: usize = 1000;

// ── 엔진-중립 스냅샷 타입(계약의 비교 통화 — 두 엔진 유닛 공용) ──────────────

/// 색 스냅샷 — 엔진 타입을 밖으로 새지 않게 자체 표현으로 고정한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorSnap {
    Default,
    Named(u8),
    Indexed(u8),
    Rgb(u8, u8, u8),
}

/// 복원 대상 private mode 집합의 스냅샷(rehydrate 가 재현해야 하는 전부).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModeSnap {
    pub bracketed_paste: bool,
    pub app_cursor: bool,
    pub app_keypad: bool,
    pub mouse_click: bool,
    pub mouse_drag: bool,
    pub mouse_motion: bool,
    pub sgr_mouse: bool,
    pub utf8_mouse: bool,
    pub focus_in_out: bool,
    pub alternate_scroll: bool,
    pub show_cursor: bool,
    pub line_wrap: bool,
    pub insert: bool,
}

/// 직렬화기가 읽는 엔진-중립 셀 — 직렬화에 필요한 것을 다 담는다(spacer·wrapline·zerowidth
/// 포함). 이 타입 하나가 직렬화기의 그리드 읽기 단일 창이다 — 엔진 세부는 이 파일 밖으로
/// 나가지 않는다.
pub struct GridCell {
    pub ch: char,
    pub fg: ColorSnap,
    pub bg: ColorSnap,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
    pub strikeout: bool,
    pub hidden: bool,
    /// wide 문자 본체(2칸 점유의 첫 칸).
    pub wide: bool,
    /// wide 문자 스페이서(본체 뒤 칸) — 직렬화기가 건너뛴다.
    pub spacer: bool,
    /// WRAPLINE — 마지막 칸에서만 의미: 이 행이 자연 개행(wrap)으로 이어진다.
    pub wrapline: bool,
    /// 결합 문자(zero-width) 후속.
    pub zerowidth: Vec<char>,
}

fn blank_cell() -> GridCell {
    GridCell {
        ch: ' ',
        fg: ColorSnap::Default,
        bg: ColorSnap::Default,
        bold: false,
        dim: false,
        italic: false,
        underline: false,
        inverse: false,
        strikeout: false,
        hidden: false,
        wide: false,
        spacer: false,
        wrapline: false,
        zerowidth: Vec::new(),
    }
}

// ── Callbacks — 삼킨 질의 계수 + getter 없는 private mode 관찰 ─────────────────
// vt100 은 응답을 만들지 않으므로 질의(DA1/DA2/DSR/OSC)는 unhandled 로 흘러온다. 여기서
// 질의는 계수만 하고(관찰 전용), getter 없는 private mode 는 상태로 재구성한다. getter 있는
// 모드(bracketed/app-cursor/app-keypad/mouse/show-cursor)는 screen 이 직접 답하므로 관찰하지
// 않는다 — 이 콜백엔 오지 않는다.

struct VtCallbacks {
    // 삼킨 응답 요구 수(DA/DSR CSI, OSC 질의). 관찰 전용 — 응답 경로는 존재하지 않는다.
    suppressed: u64,
    // getter 없는 private mode 의 관찰 상태(신선한 터미널의 초기값에서 시작).
    focus_in_out: bool,
    alternate_scroll: bool,
    line_wrap: bool,
    insert: bool,
}

impl VtCallbacks {
    fn new() -> Self {
        // 계약이 선언한 출생 상태(SPEC.md §11.I): 자동 줄바꿈(DECAWM)은 켜짐, alternate
        // scroll(1007)·focus·insert 는 꺼짐. 엔진의 기본값이 아니라 계약의 선언이 기준이다.
        VtCallbacks { suppressed: 0, focus_in_out: false, alternate_scroll: false, line_wrap: true, insert: false }
    }
}

impl Callbacks for VtCallbacks {
    fn unhandled_csi(
        &mut self,
        _: &mut VtScreen,
        i1: Option<u8>,
        _i2: Option<u8>,
        params: &[&[u16]],
        c: char,
    ) {
        // private mode set/reset(`CSI ? … h|l`) — getter 없는 모드만 상태로 잡는다.
        if i1 == Some(b'?') && (c == 'h' || c == 'l') {
            let set = c == 'h';
            for p in params {
                match p.first().copied() {
                    Some(7) => self.line_wrap = set,
                    Some(1004) => self.focus_in_out = set,
                    Some(1007) => self.alternate_scroll = set,
                    _ => {}
                }
            }
            return;
        }
        // IRM insert mode(비-private `CSI 4 h|l`).
        if i1.is_none() && (c == 'h' || c == 'l') {
            let set = c == 'h';
            for p in params {
                if p.first().copied() == Some(4) {
                    self.insert = set;
                }
            }
            return;
        }
        // 응답을 유발하는 질의 — DA(`c`)·DSR(`n`). 실제 터미널이라면 PTY 에 답을 되쓴다.
        if c == 'c' || c == 'n' {
            self.suppressed += 1;
        }
    }

    fn unhandled_osc(&mut self, _: &mut VtScreen, params: &[&[u8]]) {
        // 색·클립보드 질의는 마지막 파라미터가 `?` 다(예: `OSC 11 ; ? BEL`).
        if params.last().map_or(false, |p| *p == b"?") {
            self.suppressed += 1;
        }
    }
}

// ── Engine — 유일한 vt100 좌석 ───────────────────────────────────────────────

/// 바이트를 실제 렌더해 화면 상태를 유지하는 헤드리스 VT 엔진(vt100). 미러(복원 로직)가
/// 쓰는 유일한 엔진 면이며, "이 바이트를 먹은 터미널이 PTY 에 무엇을 되쓰려 했는가"의
/// 프로브(`suppressed_replies`)이기도 하다.
///
/// 그리드 읽기는 vt100 의 스크롤백 오프셋을 잠시 옮겼다가 0 으로 되돌린다(관측 상태 불변) —
/// [`RefCell`] 로 오프셋을 transient 읽기 커서로 다뤄, 미러 쪽 읽기 API 를 `&self` 로 유지한다.
pub struct Engine {
    parser: RefCell<Parser<VtCallbacks>>,
    cols: u16,
    rows: u16,
}

impl Engine {
    pub fn new(cols: u16, rows: u16) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);
        // vt100 Parser::new 는 (rows, cols, scrollback) 순서.
        let parser = Parser::new_with_callbacks(rows, cols, MIRROR_SCROLLBACK_LINES, VtCallbacks::new());
        Engine { parser: RefCell::new(parser), cols, rows }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.get_mut().process(bytes);
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols.max(1);
        self.rows = rows.max(1);
        // vt100 Screen::set_size 는 (rows, cols) 순서.
        self.parser.get_mut().screen_mut().set_size(self.rows, self.cols);
    }

    pub fn cols(&self) -> u16 {
        self.cols
    }

    pub fn rows(&self) -> u16 {
        self.rows
    }

    pub fn alt_active(&self) -> bool {
        self.parser.borrow().screen().alternate_screen()
    }

    /// 커서 위치(화면 기준 0-base row, col). vt100 커서는 오프셋과 무관한 드로잉 좌표다.
    pub fn cursor(&self) -> (usize, usize) {
        let (row, col) = self.parser.borrow().screen().cursor_position();
        (row as usize, col as usize)
    }

    /// 현재 스크롤백(화면 위로 밀려난) 행 수. vt100 은 히스토리 크기를 직접 노출하지 않으므로
    /// 오프셋을 최대로 밀면 clamp 된 값이 실제 스크롤백 행 수다. 읽고 나서 0 으로 되돌린다.
    pub fn history_size(&self) -> usize {
        let mut parser = self.parser.borrow_mut();
        let screen = parser.screen_mut();
        screen.set_scrollback(usize::MAX);
        let h = screen.scrollback();
        screen.set_scrollback(0);
        h
    }

    pub fn modes(&self) -> ModeSnap {
        let parser = self.parser.borrow();
        let screen = parser.screen();
        let cb = parser.callbacks();
        let mouse = screen.mouse_protocol_mode();
        let enc = screen.mouse_protocol_encoding();
        ModeSnap {
            bracketed_paste: screen.bracketed_paste(),
            app_cursor: screen.application_cursor(),
            app_keypad: screen.application_keypad(),
            mouse_click: matches!(mouse, MouseProtocolMode::Press | MouseProtocolMode::PressRelease),
            mouse_drag: matches!(mouse, MouseProtocolMode::ButtonMotion),
            mouse_motion: matches!(mouse, MouseProtocolMode::AnyMotion),
            sgr_mouse: matches!(enc, MouseProtocolEncoding::Sgr),
            utf8_mouse: matches!(enc, MouseProtocolEncoding::Utf8),
            focus_in_out: cb.focus_in_out,
            alternate_scroll: cb.alternate_scroll,
            show_cursor: !screen.hide_cursor(),
            line_wrap: cb.line_wrap,
            insert: cb.insert,
        }
    }

    /// 미러가 관찰한, 삼킨 응답 요구 수(DA1/DSR/OSC 질의). vt100 은 응답을 절대 만들지
    /// 않으므로 관찰 전용 계수다.
    pub fn suppressed_replies(&self) -> u64 {
        self.parser.borrow().callbacks().suppressed
    }

    /// 한 행(line index; 0..rows = 보이는 화면, 음수 = 스크롤백)을 엔진-중립 셀 벡터로
    /// 읽는다. 길이는 항상 `cols` — spacer 포함(직렬화기가 skip 판정을 소유한다). 스크롤백
    /// 행은 오프셋을 옮겨 위치 0 에서 읽고, 읽고 나서 0 으로 되돌린다(관측 상태 불변).
    pub fn line_cells(&self, line: i32) -> Vec<GridCell> {
        let cols = self.cols;
        let mut parser = self.parser.borrow_mut();
        let screen = parser.screen_mut();
        let (offset, view_row): (usize, u16) = if line >= 0 {
            // 보이는 화면 — 오프셋 0, 행 그대로.
            (0, line as u16)
        } else {
            // 스크롤백 — 오프셋 s 에서 위치 0 이 scrollback[H-s] 다. line -k 는 scrollback[H-k]
            // (line -1 = 최신, line -H = 최고참)이므로 s = k = -line, 위치 0 에서 읽는다.
            ((-line) as usize, 0)
        };
        screen.set_scrollback(offset);
        let out = materialize_row(screen, view_row, cols);
        screen.set_scrollback(0);
        out
    }
}

// 한 뷰 행(현재 오프셋 기준 위치 view_row)을 계약 정규형과 동형인 GridCell 벡터(길이 = cols)로
// 정렬한다. wide 문자는 본체 칸에 wide, 연속 칸에 spacer. row_wrapped 는 마지막 칸 wrapline.
fn materialize_row(screen: &VtScreen, view_row: u16, cols: u16) -> Vec<GridCell> {
    let mut out: Vec<GridCell> = Vec::with_capacity(cols as usize);
    for col in 0..cols {
        out.push(match screen.cell(view_row, col) {
            Some(cell) => cell_of(cell),
            None => blank_cell(),
        });
    }
    if screen.row_wrapped(view_row) {
        if let Some(last) = out.last_mut() {
            last.wrapline = true;
        }
    }
    out
}

fn cell_of(cell: &vt100::Cell) -> GridCell {
    // wide 연속 칸은 본체 뒤의 점유 스페이서다 — 문자를 담지 않는다.
    if cell.is_wide_continuation() {
        return GridCell { spacer: true, ..blank_cell() };
    }
    let mut chars = cell.contents().chars();
    let ch = chars.next().unwrap_or(' ');
    let zerowidth: Vec<char> = chars.collect();
    GridCell {
        ch,
        fg: snap_color(cell.fgcolor()),
        bg: snap_color(cell.bgcolor()),
        bold: cell.bold(),
        dim: cell.dim(),
        italic: cell.italic(),
        underline: cell.underline(),
        inverse: cell.inverse(),
        // vt100 셀은 strikethrough/hidden 을 노출하지 않는다 — 추적 안 하는 값을 지어내지 않는다.
        strikeout: false,
        hidden: false,
        wide: cell.is_wide(),
        spacer: false,
        wrapline: false,
        zerowidth,
    }
}

// vt100 Color → 엔진-중립 ColorSnap. 팔레트 0..16 은 Named(기본/브라이트 SGR 로 왕복),
// 16..256 은 Indexed(38;5;N), truecolor 는 Rgb(판정자 계약의 팔레트/트루컬러 정규화와 동형).
fn snap_color(color: Color) -> ColorSnap {
    match color {
        Color::Default => ColorSnap::Default,
        Color::Idx(i) => {
            if i < 16 {
                ColorSnap::Named(i)
            } else {
                ColorSnap::Indexed(i)
            }
        }
        Color::Rgb(r, g, b) => ColorSnap::Rgb(r, g, b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // vt100 은 질의에 응답 바이트를 만들지 않는다 — DA/DSR/OSC 질의는 unhandled 로 흘러
    // 계수만 된다. 각 질의를 신선한 엔진에 먹여 계수가 오르는지 곧바로 단언한다.
    #[test]
    fn queries_are_counted_but_never_answered() {
        for q in [&b"\x1b[c"[..], b"\x1b[>c", b"\x1b[6n", b"\x1b]11;?\x07"] {
            let mut e = Engine::new(80, 24);
            e.feed(q);
            assert!(
                e.suppressed_replies() > 0,
                "feed must count the swallowed query {q:?} (vt100 answers none)"
            );
        }
    }

    // getter 없는 private mode(focus/alt-scroll/auto-wrap/insert)를 unhandled_csi 관찰로
    // 재구성한다 — 기본값과 다른 것을 세운 뒤 modes() 로 확인.
    #[test]
    fn observed_private_modes_track_the_unhandled_stream() {
        let mut e = Engine::new(80, 24);
        e.feed(b"\x1b[?1004h\x1b[?1007l\x1b[?7l\x1b[4h");
        let m = e.modes();
        assert!(m.focus_in_out, "focus(1004) set");
        assert!(!m.alternate_scroll, "alt-scroll(1007) cleared");
        assert!(!m.line_wrap, "auto-wrap(7) cleared");
        assert!(m.insert, "insert(4) set");
    }
}
