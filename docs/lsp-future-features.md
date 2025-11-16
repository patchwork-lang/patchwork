# Patchwork LSP – Future Improvements

- Scope-aware hover/completions: derive symbols from the AST (per-scope identifiers, params, fields) instead of regex over the document.
- Rich hover content: include declaration kinds (worker/trait/task/fun/var), type info (if/when available), and docstring/annotation context.
- Smarter diagnostics: map precise spans from the lexer/parser for all error types; consider offering quick-fix hints where sensible.
- Formatting and document symbols: add outline/semantic tokens and format-on-save once the grammar/LSP surface stabilizes.
- Language IDs and multi-file/symbol indexing: handle imports and provide project-wide references/definitions when feasible.
