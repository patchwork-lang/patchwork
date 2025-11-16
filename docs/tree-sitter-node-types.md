# Patchwork Tree-sitter Node Types

The generated node type schema lives at:

- `tree-sitter/src/node-types.json`

Regenerate it after grammar changes:

```sh
cd tree-sitter
npx tree-sitter generate
```

Use this file as the reference for query authors (captures, fields, and node names) and for editor integrations that need the structural surface of the grammar.
