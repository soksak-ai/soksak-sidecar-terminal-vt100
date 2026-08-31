# Change log

This file records completed changes. Current behavior is defined by the terminal contract and the
documents in this directory.

## 2026-08-31

### 0.0.41

- Use the exact SDK 0.0.20 release closure for both local and public owner proofs.

### 0.0.40

- Assign the rewritten source commit a new immutable release identity; 0.0.39 remains bound to its
  original source commit and bytes.

## 2026-08-30

### 0.0.39

- Pinned vt100-rust to `5580fbb6dd389d18afbbd430fe3942867b02ae12` and the common terminal Kit
  to final `v0.0.34` commit `20fb2d73d13e5bcde592380d3052c5d2204a592f`.
- Exposed DEC 9 X10 and DEC 1001 highlight as distinct live facts without aliasing VT200 click,
  drag, or any-motion tracking.
- Routed wheel and pointer admission through the Kit's public helpers; X10 suppresses modifiers
  and reports presses only, while highlight preserves modifiers and reports press/release.

### 0.0.38

- Wheel reports are encoded at the VT100 engine boundary from the live mouse mode and encoding.
- SGR, default legacy, and UTF-8 legacy reports preserve direction, position, modifiers, and steps.
- Alternate-screen plus alternate-scroll emits application cursor keys on both axes.
- The common terminal Kit continues to own device-unit normalization and normal scrollback.

## 2026-08-29

- Pointer press, motion, and release use the live vt100 Screen mouse encoder.

## 2026-08-28

- Cursor shape and blink state now come from the versioned vt100-rust parser API.
- The renderer receives a 500 ms cursor animation policy.
- Contract cursor acceptance and the arm64 owner gate passed.
