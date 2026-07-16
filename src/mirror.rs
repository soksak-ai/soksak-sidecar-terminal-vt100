//! 헤드리스 PTY 미러 + 복원 직렬화기 — 세션 출력 바이트를 소비해 화면 상태(스크롤백·
//! alt-screen·private mode)를 유지하고, 재부착/체크포인트가 재생할 수 있는 페인트
//! 시퀀스를 만들어 낸다.
//!
//! 직렬화기·alt-freeze·재생 가드는 엔진-불가지다 — 엔진은 [`crate::engine::Engine`] 좌석 뒤에
//! 있고, 이 파일은 그 좌석이 내놓는 엔진-중립 뷰([`GridCell`] 행 읽기 + 스칼라 상태)만 읽는다.
//!
//! [`Mirror`] 는 복원 경로의 단위다 — 출력 스트림을 먹고(`feed`) 복원 시퀀스를 낸다. warm
//! 재부착은 [`Mirror::rehydrate`](화면 상태 재현), cold 체크포인트는 [`Mirror::cold_paint`]
//! (비활성 텍스트 평면화)를 쓴다. 이 면이 계약의 합격시험이 만지는 전부이며, 채점은 미러의
//! 자기 보고가 아니라 계약이 선언한 골든이 한다 — 픽스처도 골든도 이 크레이트에 사본으로 두지
//! 않는다.
//!
//! 불변식(재생 가드): 미러는 절대 응답하지 않는다 — 질의(DA1/DSR/OSC)의 단일 응답자는
//! 프론트 터미널 하나다. vt100 은 애초에 응답 바이트를 만들지 않으므로 삼킴은 공짜이고,
//! 삼킨 응답 요구는 [`Mirror::suppressed_replies`] 로 관찰만 된다. 복원 시퀀스에는 질의
//! 바이트가 실리지 않는다(이중응답 원천 차단).

use crate::engine::{ColorSnap, Engine, GridCell, ModeSnap};

// ── Mirror — 복원 경로의 단위(vt100 엔진) ────────────────────────────────────

/// 세션 출력 전량을 헤드리스로 렌더해 화면 상태를 유지하고, 복원 시퀀스를 그리드에서
/// 합성한다. 재생 바이트는 전부 합성물이라 질의가 실릴 수 없다(이중응답 원천 차단).
///
/// alt-screen 뒤에 얼어 있는 프라임 화면: 엔진은 비활성 그리드를 공개하지 않으므로,
/// alt 진입 시퀀스(`CSI ? …47/1047/1049… h`)를 피드 경계에서 감지해 진입 직전의
/// 프라임 페인트를 얼려 둔다(alt 활성 중 프라임은 불변이라 staleness 0).
pub struct Mirror {
    engine: Engine,
    // alt 진입 직전에 얼린 프라임 페인트 + 커서. alt 이탈 시 해제.
    frozen_primary: Option<FrozenPrimary>,
    // 청크 경계에 걸린 alt-진입 후보 시퀀스의 보류 버퍼(ESC 부터).
    held: Vec<u8>,
}

struct FrozenPrimary {
    paint: Vec<u8>,
    cursor: (usize, usize),
}

enum Candidate {
    // 청크가 후보 중간에서 끝났다 — 나머지가 와야 판정 가능.
    NeedMore,
    // alt 진입 DECSET(길이 = 시퀀스 전체 바이트 수).
    AltEnter(usize),
    // 후보 아님.
    No,
}

// b[0]==ESC 전제. `CSI ? <params> h` 이고 params 에 47|1047|1049 가 있으면 alt 진입.
fn classify_alt_enter(b: &[u8]) -> Candidate {
    if b.len() < 2 {
        return Candidate::NeedMore;
    }
    if b[1] != b'[' {
        return Candidate::No;
    }
    if b.len() < 3 {
        return Candidate::NeedMore;
    }
    if b[2] != b'?' {
        return Candidate::No;
    }
    let mut j = 3;
    while j < b.len() && (b[j].is_ascii_digit() || b[j] == b';') {
        j += 1;
        if j - 3 > 32 {
            return Candidate::No; // 비정상 파라미터 길이 — 보류 상한
        }
    }
    if j >= b.len() {
        return Candidate::NeedMore;
    }
    if b[j] != b'h' {
        return Candidate::No;
    }
    let hit = b[3..j]
        .split(|c| *c == b';')
        .any(|p| p == b"47" || p == b"1047" || p == b"1049");
    if hit {
        Candidate::AltEnter(j + 1)
    } else {
        Candidate::No
    }
}

impl Mirror {
    pub fn new(cols: u16, rows: u16) -> Self {
        Mirror { engine: Engine::new(cols, rows), frozen_primary: None, held: Vec::new() }
    }

    /// 세션 출력 바이트 소비. 절대 응답하지 않는다 — 응답 요구는 관찰값으로만 남는다.
    pub fn feed(&mut self, bytes: &[u8]) {
        let mut data = std::mem::take(&mut self.held);
        data.extend_from_slice(bytes);
        let mut fed = 0; // data[..fed] 는 이미 엔진에 들어갔다
        let mut i = 0;
        while i < data.len() {
            if data[i] != 0x1b {
                i += 1;
                continue;
            }
            match classify_alt_enter(&data[i..]) {
                Candidate::NeedMore => {
                    // 후보가 청크 끝에 걸렸다 — 프리픽스만 먹이고 나머지는 보류.
                    self.engine.feed(&data[fed..i]);
                    self.held = data[i..].to_vec();
                    return;
                }
                Candidate::AltEnter(len) => {
                    self.engine.feed(&data[fed..i]);
                    if !self.engine.alt_active() {
                        self.frozen_primary = Some(FrozenPrimary {
                            paint: paint_primary(&self.engine),
                            cursor: self.engine.cursor(),
                        });
                    }
                    self.engine.feed(&data[i..i + len]);
                    fed = i + len;
                    i = fed;
                }
                Candidate::No => {
                    i += 1;
                }
            }
        }
        self.engine.feed(&data[fed..]);
        // alt 이탈 후에는 프라임이 다시 라이브다 — 냉동 해제.
        if !self.engine.alt_active() {
            self.frozen_primary = None;
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.engine.resize(cols, rows);
    }

    /// warm 재부착 재생 시퀀스 — 신선한 터미널에 먹이면 세션의 화면 상태(스크롤백·
    /// alt-screen·모드·커서)가 재현된다. 전부 그리드 합성물이라 질의 바이트가 없다.
    pub fn rehydrate(&self) -> Vec<u8> {
        let mut out = b"\x1b[0m".to_vec();
        if self.engine.alt_active() {
            if let Some(fp) = &self.frozen_primary {
                out.extend_from_slice(&fp.paint);
                out.extend(cup(fp.cursor));
            }
            out.extend_from_slice(b"\x1b[?1049h");
            out.extend(paint_alt(&self.engine));
            out.extend(cup(self.engine.cursor()));
        } else {
            out.extend(paint_primary(&self.engine));
            out.extend(cup(self.engine.cursor()));
        }
        out.extend(mode_sets(&self.engine.modes()));
        out
    }

    /// cold 체크포인트 페인트 — 화면 이력을 비활성 텍스트로 평면화한 시퀀스. alt-screen
    /// 이 활성이었다면 그 프레임 내용이 (모드 전환 없이) 텍스트 블록으로 이어진다.
    /// 죽은 세션의 잔상은 텍스트가 정직하다 — 프로세스 없는 alt-screen 은 만들지 않는다.
    pub fn cold_paint(&self) -> Vec<u8> {
        let mut out = b"\x1b[0m".to_vec();
        if self.engine.alt_active() {
            if let Some(fp) = &self.frozen_primary {
                out.extend_from_slice(&fp.paint);
            }
            out.extend_from_slice(b"\r\n");
            out.extend(paint_alt_flat(&self.engine));
        } else {
            out.extend(paint_primary(&self.engine));
        }
        out.extend_from_slice(b"\x1b[0m\r\n");
        out
    }

    /// alt-screen 활성 여부(체크포인트 메타·고지용).
    pub fn alt_active(&self) -> bool {
        self.engine.alt_active()
    }

    /// 미러가 삼킨 응답 요구 수(DA1/DSR 등). 관찰 전용 — 응답 경로는 존재하지 않는다.
    /// vt100 은 응답 바이트를 만들지 않으므로 삼킴은 공짜이고, 엔진이 unhandled 질의를
    /// 계수한 값을 그대로 노출한다.
    pub fn suppressed_replies(&self) -> u64 {
        self.engine.suppressed_replies()
    }

    // ── 화면 읽기 — 합격시험이 계약의 정규형(ScreenState)으로 옮겨 갈 면. 미러는 제가 들고 있는
    // 상태를 내놓을 뿐이고, 정규형으로의 변환은 유닛 좌석(tests/conformance.rs)이 한다 — 계약은
    // 엔진 표현을 알지 못한다.

    pub fn cols(&self) -> u16 {
        self.engine.cols()
    }

    pub fn rows(&self) -> u16 {
        self.engine.rows()
    }

    /// 커서 위치(화면 기준 0-base row, col).
    pub fn cursor(&self) -> (usize, usize) {
        self.engine.cursor()
    }

    /// 복원 대상 private mode 집합.
    pub fn modes(&self) -> ModeSnap {
        self.engine.modes()
    }

    /// 스크롤백(화면 위로 밀려난) 행 수.
    pub fn history_size(&self) -> usize {
        self.engine.history_size()
    }

    /// 한 행(0..rows = 보이는 화면, 음수 = 스크롤백; -1 이 최신).
    pub fn line_cells(&self, line: i32) -> Vec<GridCell> {
        self.engine.line_cells(line)
    }
}

// ── 직렬화기 — 그리드 → SGR 런(엔진-중립 GridCell 만 읽는다) ──────────────────
// 엔진([`Engine`])의 그리드를 읽는다 — 이 파일은 엔진-중립 셀만 본다.

fn cup((row, col): (usize, usize)) -> Vec<u8> {
    format!("\x1b[{};{}H", row + 1, col + 1).into_bytes()
}

// 직렬화기·판정자 공용 "빈 셀" 기준 — 꼬리 생략의 단일 진실.
fn cell_is_blank_default(cell: &GridCell) -> bool {
    cell.ch == ' '
        && cell.fg == ColorSnap::Default
        && cell.bg == ColorSnap::Default
        && !(cell.bold
            || cell.dim
            || cell.italic
            || cell.underline
            || cell.inverse
            || cell.strikeout
            || cell.hidden)
        && cell.zerowidth.is_empty()
}

#[derive(Default, PartialEq, Clone)]
struct SgrKey {
    fg: Option<String>,
    bg: Option<String>,
    attrs: Vec<&'static str>,
}

fn sgr_key(cell: &GridCell) -> SgrKey {
    let mut attrs = Vec::new();
    if cell.bold {
        attrs.push("1");
    }
    if cell.dim {
        attrs.push("2");
    }
    if cell.italic {
        attrs.push("3");
    }
    if cell.underline {
        attrs.push("4");
    }
    if cell.inverse {
        attrs.push("7");
    }
    if cell.hidden {
        attrs.push("8");
    }
    if cell.strikeout {
        attrs.push("9");
    }
    SgrKey { fg: color_code(&cell.fg, false), bg: color_code(&cell.bg, true), attrs }
}

// 셀 색 → SGR 코드 조각. 기본색은 None(리셋 상태 그대로).
fn color_code(color: &ColorSnap, is_bg: bool) -> Option<String> {
    let base = if is_bg { 40 } else { 30 };
    let bright = if is_bg { 100 } else { 90 };
    let ext = if is_bg { 48 } else { 38 };
    match color {
        ColorSnap::Default => None,
        ColorSnap::Named(n) => {
            let i = *n as usize;
            if i < 8 {
                Some(format!("{}", base + i))
            } else if i < 16 {
                Some(format!("{}", bright + (i - 8)))
            } else {
                None // 파서가 셀에 넣지 않는 특수 이름(커서 등) — 기본색으로
            }
        }
        ColorSnap::Indexed(i) => Some(format!("{ext};5;{i}")),
        ColorSnap::Rgb(r, g, b) => Some(format!("{ext};2;{r};{g};{b}")),
    }
}

fn emit_sgr(out: &mut Vec<u8>, key: &SgrKey) {
    let mut parts: Vec<String> = vec!["0".into()];
    parts.extend(key.attrs.iter().map(|s| s.to_string()));
    if let Some(fg) = &key.fg {
        parts.push(fg.clone());
    }
    if let Some(bg) = &key.bg {
        parts.push(bg.clone());
    }
    out.extend(format!("\x1b[{}m", parts.join(";")).into_bytes());
}

// 한 행을 SGR 런으로 페인트. 반환 = 이 행이 자연 개행(wrap)으로 이어지는가.
// wrap 행은 전체 폭을 그대로 내보내(재생 시 같은 지점에서 다시 감긴다), 아닌 행은
// 꼬리의 빈 셀을 생략한다(계약 정규형과 같은 기준).
fn paint_row(out: &mut Vec<u8>, engine: &Engine, line: i32, style: &mut SgrKey) -> bool {
    let cells = engine.line_cells(line);
    let cols = cells.len();
    let wrapped = cols > 0 && cells[cols - 1].wrapline;
    // 생략 가능한 꼬리 길이(wrap 행은 0).
    let mut last = cols;
    if !wrapped {
        while last > 0 && cell_is_blank_default(&cells[last - 1]) {
            last -= 1;
        }
    }
    for cell in cells.iter().take(last) {
        if cell.spacer {
            continue;
        }
        let key = sgr_key(cell);
        if key != *style {
            emit_sgr(out, &key);
            *style = key;
        }
        let mut buf = [0u8; 4];
        out.extend_from_slice(cell.ch.encode_utf8(&mut buf).as_bytes());
        for c in &cell.zerowidth {
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        }
    }
    wrapped
}

// 프라임 페인트: 스크롤백 전체 + 보이는 화면 전 행. 모든 행을 그려야(빈 행 포함)
// 원본과 같은 바닥 정렬로 끝난다 — 커서는 호출자가 CUP 으로 되돌린다.
fn paint_primary(engine: &Engine) -> Vec<u8> {
    let mut out = Vec::new();
    let mut style = SgrKey::default();
    let hist = engine.history_size() as i32;
    let rows = engine.rows() as i32;
    for l in -hist..rows {
        let wrapped = paint_row(&mut out, engine, l, &mut style);
        if !wrapped && l != rows - 1 {
            // 줄바꿈 전에 스타일을 끈다. 켜 둔 채 개행하면, 새로 드러난 줄을 그때의 펜으로 채우는
            // 터미널(배경색 소거)에서 그 줄의 안 쓰인 칸에 색이 배어난다 — 반전이 걸려 있으면
            // 눈에 보이는 블록이 된다. 원본 스트림도 개행 전에 SGR 을 끄고 넘어간다.
            out.extend_from_slice(b"\x1b[0m\r\n");
            style = SgrKey::default();
        }
    }
    out.extend_from_slice(b"\x1b[0m");
    out
}

// alt 화면 페인트(재수화용): 행마다 CUP 절대주소 — 스크롤이 일어나지 않는다.
fn paint_alt(engine: &Engine) -> Vec<u8> {
    let mut out = b"\x1b[2J".to_vec();
    let mut style = SgrKey::default();
    for l in 0..engine.rows() as i32 {
        let row_start = out.len();
        out.extend(format!("\x1b[{};1H", l + 1).into_bytes());
        let before = out.len();
        paint_row(&mut out, engine, l, &mut style);
        if out.len() == before {
            out.truncate(row_start); // 빈 행은 CUP 조차 생략
        }
    }
    out.extend_from_slice(b"\x1b[0m");
    out
}

// alt 화면 평면화(cold용): 내용 있는 행만 위→아래 텍스트 블록으로.
fn paint_alt_flat(engine: &Engine) -> Vec<u8> {
    let mut rows: Vec<Vec<u8>> = Vec::new();
    let mut style = SgrKey::default();
    for l in 0..engine.rows() as i32 {
        let mut row = Vec::new();
        paint_row(&mut row, engine, l, &mut style);
        rows.push(row);
    }
    while rows.last().map_or(false, |r| r.is_empty()) {
        rows.pop();
    }
    let mut out = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        out.extend_from_slice(row);
        if i != rows.len() - 1 {
            out.extend_from_slice(b"\x1b[0m\r\n");
        }
    }
    out
}

// private mode 재수화 — 신선한 터미널의 기본값과 다른 것만 내보낸다.
fn mode_sets(m: &ModeSnap) -> Vec<u8> {
    let mut out = Vec::new();
    let mut set = |cond: bool, seq: &str| {
        if cond {
            out.extend_from_slice(seq.as_bytes());
        }
    };
    set(m.bracketed_paste, "\x1b[?2004h");
    set(m.app_cursor, "\x1b[?1h");
    set(m.app_keypad, "\x1b=");
    set(m.mouse_click, "\x1b[?1000h");
    set(m.mouse_drag, "\x1b[?1002h");
    set(m.mouse_motion, "\x1b[?1003h");
    set(m.sgr_mouse, "\x1b[?1006h");
    set(m.utf8_mouse, "\x1b[?1005h");
    set(m.focus_in_out, "\x1b[?1004h");
    set(m.insert, "\x1b[4h");
    set(m.alternate_scroll, "\x1b[?1007h");
    // 계약의 출생 상태에서 켜진 채 태어나는 둘(DECAWM·DECTCEM)만 꺼짐을 내보낸다(SPEC.md §11.I).
    set(!m.line_wrap, "\x1b[?7l");
    set(!m.show_cursor, "\x1b[?25l");
    out
}
