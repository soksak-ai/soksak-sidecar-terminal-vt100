# soksak-sidecar-terminal-vt100

The terminal-domain restore sidecar built on the **vt100** VT engine. It is an
engine unit implementing the contract `soksak-spec-sidecar-terminal` — the same
contract the other engine units implement on their own engines. One contract, many engine units, one at a time
behind a terminal plugin's manifest declaration (NAMING §8: the unit name carries the
engine, exactly as `soksak-sidecar-browser-chromium` carries Chromium).

## The contract lives elsewhere — this repo does not copy it

The normative wire (server face, consumer/daemon peering, checkpoint policy, failure
semantics, acceptance) is owned by **one** repo, and it is not this one:
`soksak-contract-terminal` (`~/.soksak-dev/contracts/soksak-contract-terminal`). It owns
`SPEC.md`, the corpus, the declared goldens, and the assertions this unit is graded by.
This unit implements that contract; it does not restate it.

## Engine seat vs shared machinery

The restore domain is engine-agnostic: the tee consumer `daemon.rs`, the checkpoint
policy `checkpoint.rs`, the mirror + ANSI serializer `mirror.rs`, the daemon wire
`proto.rs`, and the service runtime `service.rs`/`main.rs` never name an engine. The
engine lives behind one face in `engine.rs`, implemented here on `vt100`, exposing
`feed`/`resize`/grid·mode·cursor reads. A different engine unit swaps that one file;
the restore domain logic stays put.

## Graded against a declared golden, not against another engine

The contract declares the screen each corpus stream must produce, and this unit is graded
against that declaration: its mirror's screen must equal the golden, and the screen its own
restore paint rebuilds must equal the same golden. Nothing renders the paint on this unit's
behalf, and no engine's behaviour defines correctness — the standard is external to every
implementation, this one included.

## Engine specifics

vt100 never writes a reply to the PTY, so query suppression is inherent to it — the
mirror does not need to intercept a reply path. Swallowed queries (DA1/DSR/OSC) are
counted through vt100's `Callbacks` trait (`unhandled_csi`/`unhandled_osc`), which is
the observability the contract asks for (`suppressedReplies`). vt100 exposes native
getters for bracketed-paste, application-cursor/keypad, mouse mode/encoding, and
show-cursor; the private modes without a getter (focus tracking, alternate-scroll,
auto-wrap, insert) are reconstructed by observing the same `unhandled_csi` stream. The
grid stores a wide character as a body cell plus a continuation cell, aligned with the
contract's canonical two-cell layout.

## The gate

**This unit passes when `scripts/gate.sh` passes, and by no other means.** One command, all of
it blocking: the seven fixtures against the contract's declared goldens, the unit tests, the
real-daemon integration, and the performance budgets (SPEC.md §14.2). The benchmark is ignored
in the ordinary test run — it would slow the development loop — so the gate is what makes the
budget binding rather than decorative. The contract repo's own `scripts/gate.sh` runs this one
alongside the other units and adds the guard that only shows when they stand side by side.

## Acceptance

The contract's acceptance suite belongs to the kit, not to this repo. The seven engine-neutral
restore fixtures live in `soksak-kit-terminal-conformance`, and this unit stands its mirror up
against them in one line (`tests/conformance.rs`). GREEN on that shared suite is the unit's
gate — and with no copy here, there is nothing to drift. Real-daemon integration
(`tests/ptyd_integration.rs`, driven by `scripts/e2e/ptyd-integration.sh`) exercises the
tee→mirror→checkpoint round trip against an isolated `soksak-ptyd` binary.

## Qualification verdict

### Initial qualification — crates.io vt100 0.16.2

Conformance result against `soksak-spec-sidecar-terminal`: **6 of 7 fixtures pass**.

Fixture ⑦ `dec_line_drawing_box_restores_glyphs` was **RED**. The published vt100
0.16.2 does not implement the DEC Special Graphics character set — it ignores
`ESC ( 0` and treats SI/SO as no-ops — so a line-drawing TUI border was mirrored as
literal ASCII (`lqk`) instead of box glyphs (`┌─┐`), and the restored screen diverged
from the original. This was an engine capability gap, not a defect in the restore
domain logic (fixtures ①–⑥ all pass). Against
that engine the unit was not eligible for release, and the `dec_line_drawing` fixture
stayed active (RED, not `#[ignore]`) as the honest 6/7 record.

### Re-qualification — local vt100 fork

The gap was closed at its owner. A local vt100 fork (`../../vendor/vt100`, consumed by
Cargo path dependency) adds DEC Special Graphics support: `ESC ( <final>` / `ESC )
<final>` designation, SO/SI to invoke a G-set, glyph translation on print, DECSC/DECRC
of the charset state, and persistence across the alternate screen. Against that engine
the **unchanged** seven-fixture suite is **7 of 7** — ⑦ went RED→GREEN entirely from
the engine, with no change to the fixture or the restore logic. The lib unit tests,
`service_down`, and the real-ptyd integration remain GREEN, and the fixtures pass 20/20
repeats without flaking.

Release eligibility is now gated only on the charset support reaching a published
crate — the fork's change landing upstream (a pull request is prepared) or the fork
being published — a user decision. Until then the unit consumes the fork by path.

## Licensing is per-unit

This unit ships the vt100 engine (MIT) and carries its `LICENSE` +
`THIRD-PARTY-NOTICES`. No license crosses between units. The
conformance judge is a dev-dependency and ships nowhere, so its Apache-2.0 does not reach this
unit either.
