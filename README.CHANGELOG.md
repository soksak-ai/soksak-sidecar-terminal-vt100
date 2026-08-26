# VT100 qualification history

## DEC Special Graphics

The line-drawing contract requires DEC Special Graphics designation and invocation; the fixture and restore logic remain unchanged.

This owner pins revision `01778784e11f9e073d24559c792546ba40ac20ad` declared in `Cargo.toml`. That revision implements the required charset behavior. The seven-fixture conformance suite passes 7 of 7.

## Frame wire v2 (0.0.14, interface 0.0.2)

`terminal.frame` reads the viewport in one pass: the scrollback view is shifted once per request and the rows are read consecutively, then the view is put back. The engine reports no hyperlinks (`capabilities.hyperlinks` is false) because its cells do not track OSC 8. `tests/conformance.rs::frame_delta_reproduces_reference_states` folds this unit's delta series against the declared reference states, and `tests/bench.rs` holds `frame_at(0)` at 80×24 under 2 ms in release builds.
