# Change log

This file records completed changes. Current behavior is defined by the terminal contract and the
documents in this directory.

## 2026-08-30

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
