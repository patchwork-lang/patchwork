# Patchwork Zed Language Extension Research

## Zed Language Extension Requirements

- Zed extensions register languages under `languages/<name>/config.toml` with `name`, `grammar`, `path_suffixes`, optional `line_comments`, `tab_size`, `hard_tabs`, `first_line_pattern`, `debuggers` (Source: [Zed docs – Language Extensions](https://zed.dev/docs/extensions/languages)).
- Tree-sitter grammars are listed in `extension.toml` under `[grammars.<id>]` with `repository` (URL or `file://` path) and `rev` (Git SHA); multiple grammars per extension are allowed.
- Features such as highlighting, folding, outline, indentation, injections, syntax overrides, text redactions, runnable code detection, bracket matching, etc., are all implemented through Tree-sitter query files (`highlights.scm`, `folds.scm`, `outline.scm`, `indents.scm`, `brackets.scm`, `injections.scm`, `redactions.scm`, `runnables.scm`).
- Zed themes recognize a fixed set of captures (e.g., `@keyword`, `@string`, `@property`, `@comment`, `@function`, `@attribute`, etc.), so our query files must map Patchwork nodes to these captures to get meaningful styling.
- Language servers are optional and configured via `[language_servers.<id>]` with the list of language names; the Rust `zed::Extension` trait implements `language_server_command` to spawn the server, with optional overrides for completions, docs, etc. Multi-language servers can use `language_ids` to remap to LSP identifiers.

## Patchwork Language Inputs for Tree-sitter

- Lexer design introduces dual contexts (code vs. prompt) with transitions on `think { … }`, `ask { … }`, and `do { … }`, plus shell mode tokens for `$` commands, redirections, and prompt text blobs (`docs/lexer-design.md:1`).
- Parser grammar (`crates/patchwork-parser/src/patchwork.lalrpop:1`) enumerates all tokens (keywords, operators, punctuation, prompt tokens, shell tokens) and outlines helper constructs like `ObjectKey`, providing the canonical token inventory for the Tree-sitter grammar.
- AST design (`docs/parser-design.md:1`) lays out the structural nodes (Program, Item, SkillDecl, TaskDecl, FunctionDecl, statements, expressions) we should mirror in Tree-sitter node names/fields to keep downstream tooling coherent.

## Existing Editor Assets

- VS Code extension metadata (`package.json:1`) already defines `.pw` associations, `source.patchwork` TextMate grammar, and `#` line comments; `language-configuration.json:1` captures brackets/auto-closing pairs that we can replicate in Zed’s language config.
- The TextMate grammar in `syntaxes/patchwork.tmLanguage.json` provides a useful reference for highlighting scope decisions and prompt handling, even though Tree-sitter will supersede it.

## Example Corpus

- Example programs under `examples/` (e.g., `examples/simple-test.pw:1`, `examples/prompt-demo.pw`, `examples/historian/`) showcase workers, traits, prompt blocks, shell commands, annotations, and nested contexts; these files are ideal fixtures for `tree-sitter test` and Zed QA.
- Additional cases likely live in `test/` and `crates/patchwork-parser/src/bin/` for parser validation—worth mining for Tree-sitter corpus coverage.

## Open Questions / Follow-ups

- Whether we need an external scanner to manage prompt→code transitions (likely yes, to treat `think/ask` prompts as single nodes with embedded markdown, while allowing `do { … }` re-entry to code).
- How much of the existing compiler/parser infrastructure can be reused for an eventual language server, or whether we should defer LSP integration until after grammar + queries ship.
- Any Zed-specific features beyond the standard query set (e.g., debugger integration, runnable detection) that would benefit Patchwork immediately.
