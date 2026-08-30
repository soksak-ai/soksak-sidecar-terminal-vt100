use soksak_kit_sidecar_terminal::mirror::{
    EngineWheelInput, EngineWheelRoute, SelectionModifiers, TerminalEngine,
};
use soksak_sidecar_terminal_vt100::engine::Engine;

fn wheel(horizontal: i32, vertical: i32, route: EngineWheelRoute) -> EngineWheelInput {
    EngineWheelInput {
        row: 2,
        col: 1,
        horizontal,
        vertical,
        modifiers: SelectionModifiers::default(),
        route,
    }
}

#[test]
fn vt100_engine_owns_sgr_mouse_wheel_direction_position_and_repetition() {
    let mut engine = Engine::new(120, 40);
    engine.feed(b"\x1b[?1000h\x1b[?1006h");

    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, wheel(0, -2, EngineWheelRoute::MouseReport),)
            .unwrap(),
        b"\x1b[<64;2;3M\x1b[<64;2;3M",
    );
    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, wheel(-1, 1, EngineWheelRoute::MouseReport),)
            .unwrap(),
        b"\x1b[<65;2;3M\x1b[<66;2;3M",
    );
    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, wheel(1, 0, EngineWheelRoute::MouseReport),)
            .unwrap(),
        b"\x1b[<67;2;3M",
    );
}

#[test]
fn vt100_engine_owns_legacy_and_utf8_mouse_wheel_encodings() {
    let mut engine = Engine::new(240, 120);
    engine.feed(b"\x1b[?1000h");
    let mut legacy = wheel(0, -1, EngineWheelRoute::MouseReport);
    legacy.modifiers = SelectionModifiers {
        shift: true,
        alt: true,
        control: true,
        meta: false,
    };
    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, legacy).unwrap(),
        [0x1b, b'[', b'M', 124, 34, 35],
    );

    engine.feed(b"\x1b[?1005h");
    let mut extended = wheel(0, -1, EngineWheelRoute::MouseReport);
    extended.col = 100;
    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, extended).unwrap(),
        [0x1b, b'[', b'M', 96, 0xc2, 0x85, 35],
    );
}

#[test]
fn vt100_engine_owns_alternate_screen_alternate_scroll_on_both_axes() {
    let mut engine = Engine::new(80, 24);
    engine.feed(b"\x1b[?1049h\x1b[?1007h");

    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, wheel(1, -1, EngineWheelRoute::AlternateScroll),)
            .unwrap(),
        b"\x1bOA\x1bOC",
    );
}

#[test]
fn vt100_engine_rejects_stale_wheel_routes_after_modes_change() {
    let mut engine = Engine::new(80, 24);
    let mouse_error =
        TerminalEngine::wheel_input(&mut engine, wheel(0, -1, EngineWheelRoute::MouseReport))
            .unwrap_err();
    assert!(
        mouse_error.starts_with("WHEEL_MODE_CHANGED:"),
        "{mouse_error}"
    );

    engine.feed(b"\x1b[?1049h\x1b[?1007h\x1b[?1007l");
    let alternate_error =
        TerminalEngine::wheel_input(&mut engine, wheel(0, -1, EngineWheelRoute::AlternateScroll))
            .unwrap_err();
    assert!(
        alternate_error.starts_with("WHEEL_MODE_CHANGED:"),
        "{alternate_error}"
    );
}

#[test]
fn red_x10_and_highlight_are_distinct_mouse_report_routes() {
    let mut engine = Engine::new(80, 24);
    let modified = SelectionModifiers {
        shift: true,
        alt: true,
        control: true,
        meta: false,
    };

    engine.feed(b"\x1b[?9h");
    let x10 = TerminalEngine::modes(&engine);
    assert!(x10.mouse_x10);
    assert!(!x10.mouse_click && !x10.mouse_highlight && !x10.mouse_drag && !x10.mouse_motion);
    assert!(x10.mouse_reporting());
    let mut x10_wheel = wheel(0, -1, EngineWheelRoute::MouseReport);
    x10_wheel.modifiers = modified;
    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, x10_wheel).unwrap(),
        [0x1b, b'[', b'M', 96, 34, 35],
    );

    engine.feed(b"\x1b[?9l\x1b[?1001h");
    let highlight = TerminalEngine::modes(&engine);
    assert!(highlight.mouse_highlight);
    assert!(
        !highlight.mouse_x10
            && !highlight.mouse_click
            && !highlight.mouse_drag
            && !highlight.mouse_motion
    );
    assert!(highlight.mouse_reporting());
    let mut highlight_wheel = wheel(0, -1, EngineWheelRoute::MouseReport);
    highlight_wheel.modifiers = modified;
    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, highlight_wheel).unwrap(),
        [0x1b, b'[', b'M', 124, 34, 35],
    );

    engine.feed(b"\x1b[?1049h\x1b[?1007h");
    let precedence =
        TerminalEngine::wheel_input(&mut engine, wheel(0, -1, EngineWheelRoute::AlternateScroll))
            .unwrap_err();
    assert!(
        precedence.starts_with("WHEEL_MODE_CHANGED:"),
        "live highlight reporting must outrank alternate scroll: {precedence}"
    );
}
