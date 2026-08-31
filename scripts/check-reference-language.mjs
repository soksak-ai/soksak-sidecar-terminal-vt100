import { readFileSync } from "node:fs";

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
const documents = new Map([
  ["Cargo.toml", read("Cargo.toml")],
  ["README.md", read("README.md")],
  ["README.CHANGELOG.md", read("README.CHANGELOG.md")],
  ["README.CHANGELOG.ko.md", read("README.CHANGELOG.ko.md")],
  ["docs/TERMINAL-PRESENTATION.md", read("docs/TERMINAL-PRESENTATION.md")],
]);

const forbidden = [
  /not against another engine/i,
  /original crates\.io/i,
  /기존 crates\.io/i,
  /the defect was fixed at the engine boundary/i,
  /결함은 엔진 경계에서 수정했다/,
  /immutable fork commit/i,
  /versioned vt100-rust fork/i,
  /soksak-ai\/vt100-rust/i,
];

for (const [path, text] of documents) {
  for (const pattern of forbidden) {
    if (pattern.test(text)) throw new Error(`${path} retains comparison/provenance prose: ${pattern}`);
  }
}

const cargo = documents.get("Cargo.toml");
for (const required of [
  "Immutable engine revision providing DEC Special Graphics and distinct DEC 9/1001 input state.",
  'vt100 = { git = "https://github.com/min-median-max/vt100-rust.git", rev = "5580fbb6dd389d18afbbd430fe3942867b02ae12" }',
]) {
  if (!cargo.includes(required)) throw new Error(`Cargo.toml is missing dependency contract: ${required}`);
}

for (const required of [
  "## Graded against the declared reference state",
  "The owner pins terminal-model revision `5580fbb6dd389d18afbbd430fe3942867b02ae12` declared in `Cargo.toml`",
]) {
  if (!documents.get("README.md").includes(required)) throw new Error(`README.md is missing self-description: ${required}`);
}

if (!documents.get("README.CHANGELOG.md").includes("This owner pins revision `5580fbb6dd389d18afbbd430fe3942867b02ae12` declared in `Cargo.toml`.")) {
  throw new Error("English qualification history does not name the declared revision");
}
if (!documents.get("README.CHANGELOG.ko.md").includes("이 owner는 `Cargo.toml`에 선언된 revision `5580fbb6dd389d18afbbd430fe3942867b02ae12`을 고정한다.")) {
  throw new Error("Korean qualification history does not name the declared revision");
}

const presentation = documents.get("docs/TERMINAL-PRESENTATION.md");
if (!presentation.includes("The pinned terminal model owns parsed cursor state.")) {
  throw new Error("presentation does not describe cursor-state ownership directly");
}
if (!presentation.includes("The pinned terminal model also owns OSC 4/10/11/12 parsing")) {
  throw new Error("presentation does not describe color-state ownership directly");
}
