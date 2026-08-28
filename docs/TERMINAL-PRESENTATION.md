# Terminal presentation

The pinned terminal model owns parsed cursor state. `Screen::cursor_style()` exposes block,
underline, or bar plus blink state. The sidecar maps that API to `TerminalCursorStyle` and does not
parse CSI again.

DECSCUSR selects shape and blink. DECSET/DECRST 12 changes only blink, and DECTCEM visibility remains
separate in `TerminalModes.show_cursor`. The provider declares a 500 ms renderer animation policy.

`tests/conformance.rs::cursor_style` runs the contract-owned DECSCUSR, mode 12, DECTCEM, and warm
rehydrate cases. `make verify TARGET=aarch64-apple-darwin` verifies this provider only.
