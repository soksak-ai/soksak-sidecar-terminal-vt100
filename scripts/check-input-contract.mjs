#!/usr/bin/env node
import { readFileSync } from "node:fs";

const cargo = readFileSync(new URL("../Cargo.toml", import.meta.url), "utf8");
const engine = readFileSync(new URL("../src/engine.rs", import.meta.url), "utf8");

const exactDependencies = [
  'vt100 = { git = "https://github.com/min-median-max/vt100-rust.git", rev = "5580fbb6dd389d18afbbd430fe3942867b02ae12" }',
  'soksak-kit-sidecar-terminal = { git = "https://github.com/soksak-ai/soksak-kit-sidecar-terminal", rev = "d806c04bdd8ac26983d38a438b75438b15d57c26", features = ["integration-tests"] }',
];
for (const dependency of exactDependencies) {
  if (!cargo.includes(dependency)) {
    throw new Error(`terminal input dependency is not pinned exactly: ${dependency}`);
  }
}
if (/\b(?:vt100|soksak-kit-sidecar-terminal)\s*=\s*\{[^}]*\bpath\s*=/.test(cargo)) {
  throw new Error("terminal input dependencies must not use workspace path injection");
}

for (const fact of [
  "mouse_x10: matches!(mouse, MouseProtocolMode::X10)",
  "mouse_click: matches!(mouse, MouseProtocolMode::PressRelease)",
  "mouse_highlight: matches!(mouse, MouseProtocolMode::Highlight)",
  "let mouse_reporting = modes.mouse_reporting();",
  "modes.reports_pointer(input.phase, input.button)",
]) {
  if (!engine.includes(fact)) {
    throw new Error(`VT100 tracking-mode contract is missing: ${fact}`);
  }
}

process.stdout.write("VT100 input tracking contract: passed\n");
