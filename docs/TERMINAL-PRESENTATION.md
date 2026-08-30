# Terminal presentation

The pinned terminal model owns parsed cursor state. `Screen::cursor_style()` exposes block,
underline, or bar plus blink state. The sidecar maps that API to `TerminalCursorStyle` and does not
parse CSI again.

DECSCUSR selects shape and blink. DECSET/DECRST 12 changes only blink, and DECTCEM visibility remains
separate in `TerminalModes.show_cursor`. The provider declares a 500 ms renderer animation policy.

`tests/conformance.rs::cursor_style` runs the contract-owned DECSCUSR, mode 12, DECTCEM, and warm
rehydrate cases. `make verify TARGET=aarch64-apple-darwin` verifies this provider only.

The pinned terminal model also owns OSC 4/10/11/12 parsing and raw override state on `Screen`.
The Sidecar maps `Screen::theme_overrides()` to the common `TerminalThemeOverrides`; it does not
observe unhandled OSC setters or parse color syntax again. OSC 104/110/111/112 and RIS clear the
engine state so the common renderer reveals the current host base palette.

The same provider boundary owns live mouse tracking. `MouseProtocolMode::X10` maps only to
`TerminalModes.mouse_x10`, `PressRelease` maps only to `mouse_click`, and `Highlight` maps only to
`mouse_highlight`; drag and any-motion remain separate. The Kit's public `mouse_reporting()` helper
selects mouse-report wheel routing for all five tracking modes, while `reports_pointer(...)` admits
an X10 press but no X10 release and admits highlight press/release but no ordinary motion.

Pointer bytes come from `Screen::encode_mouse_event()`. The provider suppresses modifier bits in
X10 and preserves the normal legacy modifier rules for highlight. The vt100 public pointer type has
no wheel buttons, so this adapter applies buttons 4–7 to the provider's live mode and encoding; it
uses the same X10 modifier rule and introduces no fallback encoding. Live mouse reporting outranks
DEC 1007 alternate scroll, and both routes are refused if their mode facts changed after Kit
routing. The normally executed `red_x10_and_highlight_*` owner tests retain the original RED
criteria after the implementation turns them GREEN.
