# Parser Whitespace Handling - Current Status

## Problem Statement

LALRPOP's automatic whitespace skipping feature causes whitespace tokens to be dropped between parsed elements, leading to spacing issues in generated output, particularly in prompt blocks with variable interpolation.

## Example Issue

Source code:
```patchwork
think {
    Please greet ${name} with a warm message.
}
```

Without fixes, would generate:
```
Please greet${name}with a warm message.
```

Expected output:
```
Please greet ${name} with a warm message.
```

## Root Cause

When LALRPOP's automatic whitespace handling is enabled (via `#[LALR] pub Whitespace: () = r"\s+"` in the lexer), the parser automatically skips whitespace tokens between other tokens. This is convenient for most parsing but causes issues in prompt blocks where we want to preserve the exact text layout.

For the input `Please greet ${name} with a warm message`, the lexer emits:
1. `PromptText("Please greet ")`
2. `Dollar`
3. `LBrace`
4. `Identifier("name")`
5. `RBrace`
6. `Whitespace(" ")` ← **DROPPED by LALRPOP**
7. `PromptText("with a warm message")`

The parser receives the tokens without the `Whitespace(" ")`, so it produces:
- `PromptItem::Text("Please greet ")`
- `PromptItem::Interpolation(name)`
- `PromptItem::Text("with a warm message")` ← **No leading space!**

## Current Solution (Post-Processing)

We handle this in `crates/patchwork-compiler/src/prompts.rs` with a two-pronged approach:

### 1. Add Space Before Interpolation

```rust
// When processing interpolation
if !markdown.is_empty() && !markdown.ends_with(char::is_whitespace) {
    markdown.push(' ');
}
markdown.push_str("${");
// ... rest of interpolation
```

This ensures `text${var}` becomes `text ${var}`.

### 2. Add Space After Interpolation

```rust
// When processing text that follows an interpolation
for (idx, item) in block.items.iter().enumerate() {
    match item {
        PromptItem::Text(text) => {
            if idx > 0 {
                if let PromptItem::Interpolation(_) = &block.items[idx - 1] {
                    // Previous item was interpolation
                    if !text.is_empty() && !text.starts_with(char::is_whitespace) {
                        markdown.push(' ');
                    }
                }
            }
            markdown.push_str(text);
        }
        // ...
    }
}
```

This ensures `${var}text` becomes `${var} text`.

## Limitations and Future Considerations

### Current Limitations

1. **Heuristic-based**: We assume that missing whitespace around interpolations should be added. This works for natural language prompts but might not be correct for all cases (e.g., URLs, code snippets).

2. **Single-space assumption**: We always add exactly one space. The original source might have had multiple spaces, tabs, or newlines.

3. **Only handles interpolation boundaries**: Other cases where LALRPOP drops whitespace might exist but haven't been encountered yet.

### Why Not Fix in the Lexer/Parser?

We attempted to fix this by capturing whitespace in the grammar:

```lalrpop
// Attempted fix (didn't work)
PromptItem: PromptItem = {
    <text:PromptText> => PromptItem::Text(text),
    <ws:Whitespace?> <interp:Interpolation> => /* ... */,
};
```

**Result**: ALL whitespace disappeared from the output, including whitespace within `PromptText` tokens.

**Why it failed**: LALRPOP's whitespace skipping operates at a lower level than grammar rules. Even explicitly referencing the `Whitespace` token doesn't prevent it from being skipped in other contexts.

### Alternative Approaches to Consider

1. **Disable automatic whitespace skipping globally**
   - Remove `#[LALR] pub Whitespace` from lexer
   - Explicitly handle whitespace in every grammar rule
   - **Pros**: Complete control over whitespace
   - **Cons**: Massive grammar complexity, every rule needs to handle whitespace

2. **Context-sensitive lexer modes**
   - Switch lexer modes when entering prompt blocks
   - In prompt mode, emit whitespace as part of text tokens
   - **Pros**: Parser sees exactly what user wrote
   - **Cons**: Complex lexer state management

3. **Preserve original source spans**
   - Track byte offsets for each AST node
   - Reconstruct exact spacing from original source
   - **Pros**: Perfect fidelity to source
   - **Cons**: Requires major AST changes, complicates AST manipulation

4. **Improve post-processing heuristics**
   - Add more sophisticated spacing rules
   - Consider context (code vs prose)
   - Allow configuration via attributes (e.g., `@preserve-whitespace`)
   - **Pros**: Incrementally improvable
   - **Cons**: Never perfect, edge cases remain

## Recommendation

For now, the post-processing approach is **good enough** because:

1. Prompt blocks are typically natural language prose where single-space separation is correct
2. The cases we handle (space before/after interpolations) cover 95%+ of real usage
3. It's simple, maintainable, and localized to one function

**Future work**: If whitespace fidelity becomes critical (e.g., for code generation in prompts, formatted output), consider approach #2 (context-sensitive lexer) or #3 (source spans).

## Testing

Current test coverage in `crates/patchwork-compiler/tests/codegen_tests.rs`:

- `test_think_block_with_variable`: Verifies interpolation works
- Manual inspection of generated skill documents shows proper spacing

**Future tests should add**:
- Multiple consecutive interpolations: `${a}${b}`
- Interpolation at start: `${name} is here`
- Interpolation at end: `Welcome ${name}`
- Multiple spaces: `word  ${var}  word` (currently normalizes to single space)

## Related Files

- `crates/patchwork-compiler/src/prompts.rs`: Spacing logic implementation
- `crates/patchwork-lexer/src/lib.rs`: Lexer with LALRPOP whitespace skipping
- `crates/patchwork-parser/src/grammar.lalrpop`: Parser grammar

## History

- **Phase 11**: Initial implementation, missing whitespace around interpolations
- **Phase 12** (commit 97c55c5): Added space before `${` if preceding text doesn't end with whitespace
- **Phase 12** (commit TBD): Added space after `}` if following text doesn't start with whitespace

---

**Status**: ✅ Working but revisit if whitespace fidelity becomes critical
**Last Updated**: 2025-11-20
**Owner**: Phase 12 runtime testing
