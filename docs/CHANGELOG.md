# Change log

This file records completed changes. Current behavior is defined by the terminal contract and the
documents in this directory.

## 2026-08-28

- Cursor shape and blink state now come from the versioned vt100-rust parser API.
- The renderer receives a 500 ms cursor animation policy.
- Contract cursor acceptance and the arm64 owner gate passed.
