use soksak_kit_sidecar_terminal::mirror::{
    EnginePointerInput, PointerButton, PointerPhase, SelectionModifiers, TerminalEngine,
};
use soksak_sidecar_terminal_vt100::engine::Engine;

fn pointer(phase: PointerPhase, button: PointerButton) -> EnginePointerInput {
    EnginePointerInput {
        row: 2,
        col: 1,
        phase,
        button,
        click_count: if phase == PointerPhase::Move { 0 } else { 1 },
        modifiers: SelectionModifiers::default(),
    }
}

#[test]
fn vt100_encoder_owns_sgr_press_drag_release_and_free_motion() {
    let mut engine = Engine::new(120, 40);
    engine.feed(b"\x1b[?1002h\x1b[?1006h");
    assert_eq!(
        TerminalEngine::pointer_input(
            &mut engine,
            pointer(PointerPhase::Down, PointerButton::Left),
        )
        .unwrap(),
        b"\x1b[<0;2;3M",
    );
    assert_eq!(
        TerminalEngine::pointer_input(
            &mut engine,
            pointer(PointerPhase::Move, PointerButton::Left),
        )
        .unwrap(),
        b"\x1b[<32;2;3M",
    );
    assert_eq!(
        TerminalEngine::pointer_input(&mut engine, pointer(PointerPhase::Up, PointerButton::Left),)
            .unwrap(),
        b"\x1b[<0;2;3m",
    );

    engine.feed(b"\x1b[?1002l\x1b[?1003h");
    assert_eq!(
        TerminalEngine::pointer_input(
            &mut engine,
            pointer(PointerPhase::Move, PointerButton::None),
        )
        .unwrap(),
        b"\x1b[<35;2;3M",
    );
}

#[test]
fn red_x10_and_highlight_admit_only_their_owned_pointer_phases() {
    let mut engine = Engine::new(80, 24);
    let modified = SelectionModifiers {
        shift: true,
        alt: true,
        control: true,
        meta: true,
    };

    engine.feed(b"\x1b[?9h");
    let x10 = TerminalEngine::modes(&engine);
    assert!(x10.mouse_x10);
    assert!(!x10.mouse_click && !x10.mouse_highlight && !x10.mouse_drag && !x10.mouse_motion);
    assert!(x10.reports_pointer(PointerPhase::Down, PointerButton::Left));
    assert!(!x10.reports_pointer(PointerPhase::Up, PointerButton::Left));

    let mut x10_press = pointer(PointerPhase::Down, PointerButton::Left);
    x10_press.modifiers = modified;
    assert_eq!(
        TerminalEngine::pointer_input(&mut engine, x10_press).unwrap(),
        [0x1b, b'[', b'M', 32, 34, 35],
    );
    let x10_release =
        TerminalEngine::pointer_input(&mut engine, pointer(PointerPhase::Up, PointerButton::Left))
            .unwrap_err();
    assert!(
        x10_release.starts_with("POINTER_MODE_CHANGED:"),
        "X10 release must be rejected by the public admission rule: {x10_release}"
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
    assert!(highlight.reports_pointer(PointerPhase::Down, PointerButton::Left));
    assert!(highlight.reports_pointer(PointerPhase::Up, PointerButton::Left));
    assert!(!highlight.reports_pointer(PointerPhase::Move, PointerButton::Left));

    let mut highlight_press = pointer(PointerPhase::Down, PointerButton::Left);
    highlight_press.modifiers = modified;
    assert_eq!(
        TerminalEngine::pointer_input(&mut engine, highlight_press).unwrap(),
        [0x1b, b'[', b'M', 60, 34, 35],
    );
    let mut highlight_release = pointer(PointerPhase::Up, PointerButton::Left);
    highlight_release.modifiers = modified;
    assert_eq!(
        TerminalEngine::pointer_input(&mut engine, highlight_release).unwrap(),
        [0x1b, b'[', b'M', 63, 34, 35],
    );
    let highlight_motion = TerminalEngine::pointer_input(
        &mut engine,
        pointer(PointerPhase::Move, PointerButton::Left),
    )
    .unwrap_err();
    assert!(
        highlight_motion.starts_with("POINTER_MODE_CHANGED:"),
        "highlight motion must be rejected by the public admission rule: {highlight_motion}"
    );
}
