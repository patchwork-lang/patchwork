# Patchwork Zed Extension – Dev Guide

## Prereqs
- `npm` (for the Tree-sitter grammar under `tree-sitter/`)
- `tree-sitter` CLI (`npm install -g tree-sitter-cli` if needed)
- Zed installed locally

## Build & Test the Grammar
1. `cd tree-sitter`
2. `npm install`
3. `npx tree-sitter test`

Corpus fixtures live under `tree-sitter/test/corpus/`. Add new cases there and rerun the tests to lock behavior.

## Run the Extension in Zed
1. From repo root: `zed --extension-dev editors/zed`
2. Open a `.pw` file and confirm highlighting, prompts, injections, folds, and outline entries.

The extension files are rooted at `editors/zed/extension.toml`, `editors/zed/languages/patchwork/config.toml`, and `editors/zed/queries/patchwork/*.scm` (copied from `tree-sitter/queries/`).

## Packaging Notes
- `extension.toml` currently points to the grammar on `main` at `tree-sitter/`.
- Queries are copied into `editors/zed/queries/patchwork/`. Regenerate them after query edits with `cp tree-sitter/queries/*.scm editors/zed/queries/patchwork/`.
- Node types are generated at `tree-sitter/src/node-types.json` for downstream reference.
