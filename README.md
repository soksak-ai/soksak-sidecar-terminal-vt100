# soksak-sidecar-terminal-vt100

The terminal-domain restore sidecar built on the **vt100** VT engine. It is an
engine unit implementing the contract `soksak-spec-sidecar-terminal` — the same
contract the other engine units implement on their own engines. One contract, many engine units, one at a time
behind a terminal plugin's manifest declaration (NAMING §8: the unit name carries the
engine, exactly as `[redacted]` carries Chromium).

## The contract lives elsewhere — this repo does not copy it

The normative wire (server face, consumer/daemon peering, checkpoint policy, failure
semantics, acceptance) is owned by **one** repo, and it is not this one:
`soksak-contract-terminal`. It owns
`SPEC.md`, the corpus, the declared reference states, and the assertions this unit is graded by.
This unit implements that contract; it does not restate it.

## Engine seat vs shared machinery

The restore domain is engine-agnostic: the tee consumer `daemon.rs`, the checkpoint
policy `checkpoint.rs`, the mirror + ANSI serializer `mirror.rs`, the daemon wire
`proto.rs`, and the service runtime `service.rs`/`main.rs` never name an engine. The
engine lives behind one face in `engine.rs`, implemented here on `vt100`, exposing
`feed`/`resize`/grid·mode·cursor reads. A different engine unit swaps that one file;
the restore domain logic stays put.

## Graded against the declared reference state

The contract declares the screen each corpus stream must produce, and this unit is graded
against that declaration: its mirror's screen must equal the reference state, and the screen its own
restore paint rebuilds must equal the same reference state. Nothing renders the paint on this unit's
behalf. The declared reference state is the sole correctness criterion for this implementation.

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

```sh
make lock TARGET=aarch64-apple-darwin
make verify TARGET=aarch64-apple-darwin
```

`make lock` is the only owner operation that projects changed Cargo declarations into
`Cargo.lock`. Normal build and verification remain `--locked`.

**This unit passes when `scripts/gate.sh` passes, and by no other means.** One command, all of
it blocking: the seven fixtures against the contract's declared reference states, the unit tests, and
the performance budgets (SPEC.md §14.2). The benchmark is ignored
in the ordinary test run — it would slow the development loop — so the gate is what makes the
budget binding rather than decorative. The contract repo's own `scripts/gate.sh` runs this one
alongside the other units and adds the guard that only shows when they stand side by side.

## Acceptance

The contract's acceptance suite belongs to the kit, not to this repo. The seven engine-neutral
restore fixtures live in `soksak-kit-terminal-conformance`, and this unit stands its mirror up
against them in one line (`tests/conformance.rs`). GREEN on that shared suite is the unit's
gate — and with no copy here, there is nothing to drift. Installed PTY and recovery-sidecar
composition belongs to the terminal acceptance repository, which installs both products through
Core and verifies warm and archived restore across every terminal plugin.

## Qualification verdict

The owner pins `soksak-ai/vt100-rust` commit
`01778784e11f9e073d24559c792546ba40ac20ad`. That engine includes DEC Special
Graphics support, and the unchanged seven-fixture conformance suite passes 7 of 7.

## Licensing is per-unit

This unit ships the vt100 engine (MIT) and carries its `LICENSE` +
`THIRD-PARTY-NOTICES`. No license crosses between units. The
conformance judge is a dev-dependency and ships nowhere, so its Apache-2.0 does not reach this
unit either.
