# Patchwork Zed Language Extension Plan

## Status

- [x] Research Zed language extension requirements
- [x] Capture Patchwork lexer/parser design references
- [ ] Implement Tree-sitter grammar and Zed extension artifacts

## Goals

- Deliver first-class Patchwork authoring support inside Zed, matching or exceeding the existing VS Code experience.
- Leverage Tree-sitter for accurate parsing of Patchwork code, prompt blocks, and shell segments.
- Provide Tree-sitter queries for highlighting, folding, outline, indentation, injections, and future redaction/runnable features.
- Package the grammar and metadata as a Zed extension with clear documentation and, optionally, a language server hookup.

## Phase 1: Tree-sitter Grammar Foundation

**Goal:** Stand up a Tree-sitter grammar that mirrors the existing lalrpop grammar and lexer behavior, including prompt-mode transitions.

 - [x] Inventory tokens, precedence, and structural nodes from `crates/patchwork-parser` and `docs/lexer-design.md` to define the grammar surface area.
 - [x] Scaffold `grammar.js` with modules/items/statements/expressions that reflect the AST in `docs/parser-design.md`.
 - [x] Implement prompt sections so `think { … }`/`ask { … }` switch into a prompt node that captures raw markdown, and `do { … }` transitions back to code (likely via an external scanner/state machine).
 - [x] Model shell statements/expressions (`$ …`, `$(…)`, redirections) so they parse as dedicated nodes for later highlighting/injections.
- [x] Create fixtures from `examples/` and `test/` plus targeted edge cases; run `tree-sitter test` until they pass.
- [x] Export `node-types.json` and document node/field names for downstream query authors.

## Phase 2: Query Suite & Embedded Languages

**Goal:** Provide the Tree-sitter query files Zed needs for editor features and embedded-language handoffs.

 - [x] Write `queries/highlights.scm` covering Patchwork keywords, prompt delimiters, shell constructs, annotations, and literals (aligned with TextMate scopes).
 - [x] Author `queries/injections.scm` so prompt nodes embed Markdown and shell nodes embed Bash/command syntax; ensure `do { … }` inside prompt re-enters Patchwork.
 - [x] Add `folds.scm`, `brackets.scm`, and `indents.scm` to keep structure-aware folding/indentation on par with the compiler’s block semantics.
 - [x] Implement `outline.scm` (and optional `textobjects.scm`) so workers, traits, tasks, skills, and functions show up in Zed’s outline/navigation UI.
- [x] Add `redactions.scm`/`runnables.scm` placeholders or initial rules if we identify sensitive sections or runnable scripts worth exposing.

## Phase 3: Zed Extension Packaging

**Goal:** Register the grammar and language metadata so Zed recognizes `.pw` files and loads our queries.

- [x] Create `extension.toml` with `[grammars.patchwork]` pointing to the Tree-sitter repository/revision (use `file://` during development, Git URL for release).
- [x] Add `languages/patchwork/config.toml` defining `name`, `grammar`, `path_suffixes = ["pw"]`, `line_comments = ["# "]`, and indentation defaults consistent with `language-configuration.json`.
- [x] Bundle the compiled queries under `queries/patchwork/*.scm` inside the extension directory structure.
- [x] Document build/test instructions (e.g., `zed --extension-dev`) so contributors can iterate locally.

## Phase 4: Language Server Integration (Optional)

**Goal:** Evaluate and, if feasible, ship LSP-backed features powered by the existing Patchwork compiler/runtime.

- [x] Decide whether to adapt the current compiler into an LSP (diagnostics, hover, completion) or defer.
- [x] If pursuing, implement a language server binary (Rust or JS) that reuses `patchwork-parser`/`patchwork-compiler` crates for analysis.
- [x] Register `[language_servers.patchwork]` in `extension.toml` with `languages = ["Patchwork"]` and implement `language_server_command` in the Rust extension harness.
- [x] Map any custom `language_ids` and test completions/diagnostics formatting within Zed.

## Phase 5: QA & Release

**Goal:** Validate the full experience and publish the extension.

- [ ] Manually verify highlighting, folding, outline, prompt/markdown embedding, and shell injections against the sample programs in `examples/` and `test/`.
- [x] Add automated regression coverage (Tree-sitter corpus tests, optional screenshot/highlight tests) to guard against grammar drift.
- [ ] Package the extension, update repo documentation (README + docs/), and share install instructions for contributors.
- [ ] Track follow-up improvements (debuggers, runnables, redactions, language server enhancements) in this plan or GitHub issues.
