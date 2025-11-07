# Code Fence Issue Analysis (Issue #5) - ✅ RESOLVED

**Status:** FIXED with balanced braces + escape syntax
**Date Resolved:** 2025-11-05
**Solution:** Option G - Balanced Braces with Recursive Grammar

---

# Original Analysis

## Problem Statement

When writing prompts with code examples (using markdown code fences), the braces `{` `}` inside the code are tokenized as patchwork syntax tokens instead of being treated as prompt text.

## Example from analyst.pw

```patchwork
var commit_plan = think {
    Create a detailed plan as an array of commit objects:
    ```javascript
    [
        {num: 1, description: "Add user authentication models"},
        {num: 2, description: "Implement OAuth token validation"},
        // ... feature commits ...
        {num: N, description: "Add tests and update documentation"}
    ]
    ```
}
```

## Error

```
UnrecognizedToken {
    token: (3249, LBrace, 3250),
    expected: ["newline", "dollar", "\"do\"", "\"}\"", "prompt_text"]
}
```

At byte offset 3249, which is the `{` in `{num: 1, description: ...}` on line 91.

## Root Cause Analysis

### Lexer Behavior

**Current PromptText pattern:**
```alex
PromptText: <Prompt> [^{}\s\$]+
```

This pattern explicitly **excludes** `{` and `}` characters from prompt text.

**Why?** The lexer needs to detect:
- `do {` - embedded code blocks in prompts
- `${expr}` - interpolation expressions
- `}` - end of the prompt block

### Parser Expectations in Prompt Mode

```lalrpop
PromptItem: PromptItem<'input> = {
    <text:prompt_text> => PromptItem::Text(text),
    dollar <id:identifier> => PromptItem::Interpolation(Expr::Identifier(id)),
    dollar "{" <e:Expr> "}" => PromptItem::Interpolation(e),
    "do" "{" <statements:StatementList> "}" => PromptItem::Code(Block { statements }),
};
```

When the parser is inside a `think { ... }` block in Prompt mode, it expects:
1. `prompt_text` tokens (which can't contain `{` or `}`)
2. `dollar` for interpolation
3. `"do"` for embedded code
4. `"}"` to close the prompt block
5. `newline` tokens

### What Happens with Code Fences

Inside the `think { ... }` block at line 74-97:

1. Line 88: `Create a detailed plan...` → **PromptText** ✅
2. Line 89: ` ```javascript` → **PromptText** ✅
3. Line 90: `[` → **PromptText** ✅
4. Line 91: `{num: 1, ...}` → **LBrace** token ❌

The lexer sees the `{` and emits an **LBrace** token, not PromptText.

The parser expects one of:
- `prompt_text` (can't be `{`)
- `dollar` (for interpolation)
- `"do"` (for embedded code block)
- `"}"` (to close the think block)
- `newline`

But it gets `LBrace`, which doesn't match any valid PromptItem production.

## Why the Design Excludes Braces

The current design tries to use the lexer to distinguish between:

1. **Prompt text**: `foo bar baz` → PromptText tokens
2. **Interpolation**: `${var}` → Dollar, LBrace, identifier, RBrace
3. **Embedded code**: `do { x = 1 }` → Do, LBrace, Code tokens, RBrace
4. **Block end**: `}` → RBrace

The problem is that `{` and `}` are **overloaded**:
- In code/interpolation context: structural delimiters
- In prose/examples: just regular characters

## Potential Solutions

### Option A: Escape Braces in Prose

**Approach**: Require users to escape braces in prompt text
```patchwork
think {
    Example: \{num: 1, description: "..."\}
}
```

**Pros**:
- Simple, no lexer changes needed
- Clear distinction between structural and literal braces

**Cons**:
- User-unfriendly (breaks copy-paste of code examples)
- Non-obvious requirement
- Doesn't match markdown/other documentation conventions

### Option B: Code Fence Recognition

**Approach**: Make lexer recognize ` ``` ` fences and treat everything inside as text

**Pros**:
- Natural for documentation
- Matches markdown conventions
- Users can paste code examples verbatim

**Cons**:
- Complex lexer state machine (need FenceText mode)
- Need to track fence start/end (` ``` ` vs ` ``` ` with language)
- Nested fences become ambiguous

### Option C: Allow Braces in PromptText with Lookahead

**Approach**: Change PromptText pattern to allow `{` `}` when not followed by syntax

**Problem**: Alex lexer doesn't support lookahead patterns effectively

**Pattern would need to match**:
- `{` when NOT followed by identifier (for `${var}`)
- `}` when NOT closing a `do {` block or `${...}` interpolation
- But also handle `do {` which starts with text "do" then `{`

This is very complex and error-prone in a regex-based lexer.

### Option D: Parser-Based Disambiguation (Context-Sensitive)

**Approach**: Allow LBrace/RBrace in Prompt mode, use parser to determine meaning

Change PromptItem to:
```lalrpop
PromptItem: PromptItem<'input> = {
    <text:prompt_text> => PromptItem::Text(text),
    "{" => PromptItem::Text("{"),  // literal brace in text
    "}" => PromptItem::Text("}"),  // literal brace in text
    dollar <id:identifier> => PromptItem::Interpolation(...),
    dollar "{" <e:Expr> "}" => PromptItem::Interpolation(...),
    "do" "{" <statements:StatementList> "}" => PromptItem::Code(...),
};
```

**Problem**: Now `}` is ambiguous:
- Does it close the think/ask block?
- Or is it literal text?

The parser would need to count brace depth, which gets complex with interpolation and embedded code.

### Option E: String Literal Syntax for Code Examples

**Approach**: Require code examples to be in string literals

```patchwork
think {
    Create a detailed plan as an array of commit objects:
    '```javascript
    [
        {num: 1, description: "Add user authentication models"},
    ]
    ```'
}
```

**Pros**:
- Clear distinction (strings are for verbatim content)
- No lexer changes needed
- Already supported by current syntax

**Cons**:
- Slightly awkward (mixing quote styles)
- Escaping quotes inside becomes an issue
- Not as natural as plain text

### Option F: Indented Code Blocks

**Approach**: Use indentation to signal code blocks (like Python/Markdown)

```patchwork
think {
    Create a detailed plan:

        [
            {num: 1, description: "..."},
        ]
}
```

**Pros**:
- Natural indentation already present
- No special syntax needed
- Matches markdown code blocks

**Cons**:
- Whitespace-significant (complex in lexer)
- Need to track indentation levels
- Less explicit than fences

## Recommended Solution

**Hybrid Approach: Option E (Short-term) + Option B (Long-term)**

### Short-term (Quick Fix)
Use single-quoted strings for code examples:
```patchwork
think {
    Create a plan:
    '```javascript
    [{num: 1, description: "..."}]
    ```'
}
```

Changes needed:
- Update analyst.pw to use string syntax
- Document pattern for code examples

### Long-term (Proper Fix)
Implement code fence recognition in lexer:
1. Add FenceText mode
2. Recognize ` ``` ` as fence delimiter
3. Switch to FenceText mode inside fences
4. Emit all content as PromptText until closing ` ``` `

Changes needed:
- Add FenceText mode to lexer.alex
- Add mode transitions on ` ``` `
- Handle fence language tags (` ```javascript `, etc.)
- Update parser to handle fence tokens

## Decision Factors

| Solution | Effort | User-Friendliness | Robustness | Breaking Change |
|----------|--------|-------------------|------------|----------------|
| A: Escape | Low | Low | High | No |
| B: Fences | High | High | Medium | No |
| C: Lookahead | Very High | High | Low | No |
| D: Parser-based | Medium | Medium | Low | No |
| E: Strings | Very Low | Medium | High | No |
| F: Indentation | High | Medium | Medium | No |

## Recommendation

Start with **Option E** (string syntax) to unblock analyst.pw immediately, then implement **Option B** (fence recognition) for a better long-term solution.

This gives us:
1. Immediate progress (update 1 file)
2. Clear path forward (fence mode implementation)
3. No breaking changes to existing code
4. Better user experience eventually

---

# ✅ IMPLEMENTED SOLUTION (2025-11-05)

## Option G: Balanced Braces with Recursive Grammar

Instead of the recommended hybrid approach, we implemented a better solution that allows balanced braces naturally in prompts through recursive parsing.

### Implementation

**Lexer Changes:**
- Added `PromptEscape: <Prompt> \$'(.)'` for `$'<char>'` escape syntax
- Kept `PromptText: <Prompt> [^{}\s\$]+` (excludes braces as before)

**Parser Changes:**
```lalrpop
PromptItem: PromptItem<'input> = {
    <text:prompt_text> => PromptItem::Text(text),

    // NEW: Escaped characters
    <escaped:prompt_escape> => PromptItem::Text(escaped),

    // NEW: Balanced braces (recursive)
    "{" <inner:PromptBlock> "}" => {
        // Flatten inner items to text representation
        let mut text = String::from("{");
        for item in &inner.items {
            // Append each item's text
        }
        text.push('}');
        PromptItem::Text(text.leak())
    },

    // Existing: Interpolation (dollar prefix disambiguates)
    dollar <id:identifier> => PromptItem::Interpolation(...),
    dollar "{" <e:Expr> "}" => PromptItem::Interpolation(...),

    // Existing: Embedded code (do prefix disambiguates)
    "do" "{" <statements:StatementList> "}" => PromptItem::Code(...),
};
```

### How It Works

1. **Balanced braces are recursive**: `{...}` parses as a PromptBlock, then flattens to text
2. **Prefixes disambiguate**:
   - `${expr}` - dollar prefix means interpolation
   - `do { code }` - do keyword means embedded code
   - `{text}` - no prefix means balanced text braces
3. **Escape for edge cases**: `$'{'` and `$'}'` for imbalanced braces

### Examples

**Balanced braces work naturally:**
```patchwork
think {
    Return an object: {name: "test", value: 42}
}
```

**Nested braces:**
```patchwork
think {
    Complex: {outer: {inner: 123}}
}
```

**With interpolation:**
```patchwork
think {
    Object: {name: $userName, id: ${userId + 100}}
}
```

**Escape syntax for imbalanced:**
```patchwork
think {
    Syntax example: x = $'{' expr
}
```

### Test Results

- ✅ `test_balanced_braces_in_prompt` - passes
- ✅ `test_nested_balanced_braces_in_prompt` - passes
- ✅ `test_balanced_braces_with_interpolation` - passes
- ✅ `test_prompt_escape_syntax` - passes
- ✅ `test_parse_historian_analyst` - **NOW PASSING!**

### Benefits

1. **Natural syntax** - Balanced braces (99% of cases) just work
2. **No string quoting** - Code examples don't need wrapping in strings
3. **Proper fix** - Not a workaround, properly integrated into grammar
4. **No breaking changes** - Existing code continues to work
5. **Future-proof** - Escape syntax handles edge cases

### Impact

- analyst.pw now parses successfully
- Test status improved from 93/96 to 99/100 passing
- Files passing: 3/4 historian examples (only scribe.pw still failing)

## Comparison to Original Recommendations

| Aspect | Recommended (Option E+B) | Implemented (Option G) |
|--------|-------------------------|------------------------|
| Short-term fix | String syntax | Balanced braces |
| User experience | Medium (strings awkward) | Excellent (natural) |
| Effort | Very Low | Low-Medium |
| Robustness | High | High |
| Long-term plan | Fence mode | Escape syntax |
| Breaking changes | No | No |

**Verdict:** Option G (balanced braces) proved superior to the recommended approach. It provides better UX without requiring future fence mode implementation.
