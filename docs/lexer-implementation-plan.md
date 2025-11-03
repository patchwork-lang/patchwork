# Patchwork Lexer Implementation Plan

**Goal:** Implement a context-aware lexer for patchwork that can successfully tokenize the historian examples.

**Approach:** Build incrementally with unit tests, starting from infrastructure through to full example validation.

## Current Status

**✓ Lexer Complete** - All 5 core milestones finished!

- **52 tests passing** including 4 historian example files
- Full support for context-aware tokenization (Code, Prompt, InString modes)
- Complete string interpolation with arbitrary nesting: `$id`, `${expr}`, `$(cmd)`
- Prompt interpolation in think/ask blocks
- Single and double-quoted strings with proper escaping
- Driver-based state management for complex nested contexts

**Next Steps:** Begin parser implementation or add optional enhancements (Milestone 6) as needed.

---

## Milestone 1: Infrastructure & Build Setup ✓

**Goal:** Get parlex-gen integrated and compiling

### Build infrastructure
- [x] Create `crates/patchwork-lexer/lexer.x` with minimal ALEX specification
- [x] Set up `build.rs` to invoke parlex-gen during cargo build
- [x] Add parlex-gen dependency to Cargo.toml
- [x] Verify `cargo build` successfully generates lexer code

### Basic project structure
- [x] Create `src/lib.rs` with public lexer interface
- [x] Create `tests/` directory for unit tests
- [x] Write initial smoke test that creates a lexer and tokenizes empty input

---

## Milestone 2: Code State Tokens ✓

**Goal:** Handle basic tokenization in Code state only (no state transitions yet)

### Keywords and identifiers
- [x] Implement keyword tokens (import, from, var, if, else, for, while, await, task, skill, fun, type, return, succeed, fail, break, self, in)
- [x] Implement identifier token pattern
- [x] Write unit tests for keywords vs identifiers

### Literals
- [x] Implement string literals (basic double-quoted strings with escape sequences)
- [x] Implement number literals (integers - floats handled as Number Dot Number)
- [x] Implement boolean literals (true, false)
- [x] Write unit tests for all literal types

### Operators and punctuation
- [x] Implement comparison operators (==, !=, <, >, <=, >=)
- [x] Implement arithmetic operators (+, -, *, /, %)
- [x] Implement logical operators (&&, ||, !)
- [x] Implement assignment and other operators (=, |, &, ->, ...)
- [x] Implement punctuation ({, }, (, ), [, ], ;, ,, ., :, @)
- [x] Write unit tests for operators and punctuation

### Whitespace and comments
- [x] Implement whitespace handling
- [x] Implement single-line comments (# to end of line, not //)
- [x] Write unit tests for comments

### String interpolation
- [ ] Deferred to Milestone 3 - will handle with state transitions

### Integration test
- [x] Write test that tokenizes a simple code snippet with variables, expressions, and comments
- [x] Write test with historian example snippet

**Key learnings:**
- ALEX doesn't support `//` style comments - using `#` instead (matches design doc)
- `self` keyword generates `Self` enum variant which conflicts with Rust keyword - renamed to `SelfKw`
- Escaped backslashes in regex patterns (like `\-\>`) can trigger Unicode word boundary errors - use unescaped where possible
- Optional regex groups `()?` don't work reliably in ALEX - floats tokenize as `Number Dot Number` (parser will handle)
- Rule ordering matters: longer/more specific patterns must come before shorter ones

---

## Milestone 3: State Transitions & Prompt Handling ✓

**Goal:** Implement context-aware lexing with state stack for think/ask/do operators

### State infrastructure
- [x] Define Code and Prompt lexer states in ALEX spec
- [x] Implement state stack mechanism for tracking nested contexts
- [x] Implement brace depth tracking

### Prompt operator transitions
- [x] Implement `think` keyword that transitions Code → Prompt
- [x] Implement `ask` keyword that transitions Code → Prompt
- [x] Write unit tests for simple think/ask blocks

### Code operator transitions
- [x] Implement `do` keyword with lookahead for `{` that transitions Prompt → Code
- [x] Ensure `do` as identifier works in Code state
- [x] Write unit tests for do transitions

### Prompt text handling
- [x] Implement PromptText token (captures text in Prompt state)
- [x] Handle braces within prompt text
- [x] Handle `do` without `{` as regular prompt text (emits Do token, no transition)
- [x] Write unit tests for prompt text edge cases

### Nested context handling
- [x] Test simple nesting: `think { ... do { ... } }`
- [x] Test complex nesting: `think { ... do { ... think { ... } } }`
- [x] Write unit tests for multiple levels of nesting

**Key learnings:**
- ALEX uses longest-match semantics, not first-match
- PromptText pattern `[^{}]+` was too greedy - changed to `[^{}\s]+` to allow keyword recognition
- Whitespace tokens should not clear `last_token` state tracking
- State transitions must occur after yielding the token to ensure correct mode for next token
- PromptText now tokenizes word-by-word with separate Whitespace tokens

---

## Milestone 4: Full Example Validation ✓

**Goal:** Successfully lex all historian examples and handle real-world patterns

### Bash substitution
- [x] Implement `$(...)` token pattern for bash command substitution
- [x] Write unit tests for bash substitution

### Example file testing
- [x] Test lexer on `examples/historian/main.pw`
- [x] Test lexer on `examples/historian/analyst.pw`
- [x] Test lexer on `examples/historian/narrator.pw`
- [x] Test lexer on `examples/historian/scribe.pw`

### Edge case refinement
- [x] Identify and fix any tokenization errors from example files
- [x] Refine token set based on actual usage patterns
- [x] Add unit tests for discovered edge cases

### Final validation
- [x] Verify all examples tokenize without errors
- [x] Review token streams for parser readiness

**Key learnings:**
- Bash substitution pattern `\$\([^\)]*\)` works well for simple cases
- All historian examples tokenize successfully with existing token set
- No modifications to example files were needed
- Basic lexer complete, ready for string interpolation enhancement

---

## Milestone 5: String Interpolation & Escaping ✓

**Goal:** Implement proper string interpolation and escaping in both Code and Prompt contexts

### String Literal Types in Code Context

#### Double-quoted strings (interpolated)
- [x] Replace single `String` token with chunked tokenization
- [x] Emit `StringStart` token for opening `"`
- [x] Emit `StringText` tokens for literal text segments
- [x] Emit `StringEnd` token for closing `"`
- [x] Implement escape sequences (`\n`, `\t`, `\r`, `\\`, `\"`) - handled by ALEX pattern
- [x] Implement `\$` escape sequence (literal dollar sign)
- [x] Write unit tests for basic double-quoted strings

#### Interpolation patterns in double-quoted strings
- [x] Recognize `$identifier` pattern (e.g., `"Hello $name"`)
  - Emit `Dollar` token followed by `Identifier`
  - Driver switches back to InString mode after identifier
- [x] Recognize `${expression}` pattern (e.g., `"Total: ${x + y}"`)
  - Emit `Dollar`, `LBrace`, tokenize expression in Code context, `RBrace`
  - Track brace depth for nested expressions with delimiter stack
- [x] Recognize `$(command)` pattern inside strings (e.g., `"Date: $(date)"`)
  - Removed `BashSubst` token, now tokenizes as individual tokens
  - Emit `Dollar`, `LParen`, content, `RParen` with state tracking
- [x] Write unit tests for each interpolation pattern
- [x] Support arbitrary nesting depth of `${...}` and `$(...)` forms

#### Single-quoted strings (literal, no interpolation)
- [x] Implement `SingleQuoteString` token for `'...'` literals
- [x] No interpolation or escape sequences except `\'` and `\\`
- [x] Write unit tests for single-quoted strings

### Interpolation in Prompt Context

#### Direct interpolation (no quotes needed)
- [x] Recognize `$identifier` in Prompt context
  - Convert `PromptText` followed by `$identifier` to separate tokens
- [x] Recognize `${expression}` in Prompt context
  - Emit `Dollar`, `LBrace`, switch to Code context, tokenize expression, `RBrace`
  - Track brace depth and return to Prompt context
- [x] Recognize `$(command)` in Prompt context
  - Emit `Dollar`, `LParen`, tokenize bash command, `RParen`
- [x] Write unit tests for prompt interpolation

### State Management

#### String state tracking
- [x] Add `InString` mode to lexer states (alongside Code and Prompt)
- [x] Maintain state stack for nested interpolations: `String → Expression → String`
- [x] Track delimiter type (Brace vs Paren) to distinguish `${...}` from `$(...)`

#### Interpolation state transitions
- [x] Code string interpolation: `Code (String) → ${ → Code (Expression) → } → Code (String)`
- [x] Handle nested cases: `"Outer ${f("Inner ${x}")} text"`
- [x] Handle mixed nesting: `"Result: ${x + $(cmd)}"` and `"A: ${a + $(b + ${c})}"`
- [x] Prompt interpolation: `Prompt → ${ → Code (Expression) → } → Prompt`

### Token Set Updates

#### New token types
- [x] `StringStart` - opening `"` for interpolated string
- [x] `StringText` - literal text chunk within string
- [x] `StringEnd` - closing `"` for interpolated string
- [x] `Dollar` - interpolation prefix `$` (active in both Code and InString modes)
- [x] `SingleQuoteString` - complete single-quoted literal

#### Modified tokens
- [x] Removed `String` token entirely in favor of chunked tokenization
- [x] Removed `BashSubst` token - now tokenizes as `Dollar`, `LParen`, ..., `RParen`

### Example Test Cases

#### Code context examples
```patchwork
# Simple interpolation
"Hello $name"  # → StringStart, StringText("Hello "), Dollar, Identifier(name), StringEnd

# Expression interpolation
"Total: ${x + y}"  # → StringStart, StringText("Total: "), Dollar, LBrace, Identifier(x), Plus, Identifier(y), RBrace, StringEnd

# Command substitution
"Date: $(date)"  # → StringStart, StringText("Date: "), Dollar, LParen, Identifier(date), RParen, StringEnd

# Escaped dollar
"Price: \$100"  # → StringStart, StringText("Price: $100"), StringEnd

# Nested interpolation
"Outer ${f("inner")}"  # → StringStart, StringText("Outer "), Dollar, LBrace, Identifier(f), LParen, StringStart, StringText("inner"), StringEnd, RParen, RBrace, StringEnd

# Single-quoted (no interpolation)
'Literal $var text'  # → SingleQuoteString("Literal $var text")
```

#### Prompt context examples
```patchwork
think {
    Analyze $filename and check ${x + y} items
}
# Inside prompt:
# → PromptText("Analyze"), Whitespace, Dollar, Identifier(filename),
#   Whitespace, PromptText("and"), Whitespace, PromptText("check"), Whitespace,
#   Dollar, LBrace, Identifier(x), Plus, Identifier(y), RBrace,
#   Whitespace, PromptText("items")
```

### Integration & Testing

- [x] Test escape sequences in double-quoted strings
- [x] Test all three interpolation forms (`$id`, `${expr}`, `$(cmd)`)
- [x] Test nested interpolations (string in expression in string)
- [x] Test deeply nested mixed interpolations
- [x] Test edge cases (empty strings, only interpolation, multiple interpolations)
- [x] Test single-quoted strings (no interpolation)
- [x] Test prompt context interpolations
- [x] Verify all historian examples still tokenize correctly (52 tests pass)

**Key learnings:**
- Driver-based state switching is superior to ALEX patterns for complex interpolation
- Single-token patterns like `BashSubst` prevent proper state tracking for nested contexts
- `DelimiterType` enum is essential to distinguish `${...}` (waiting for `}`) from `$(...)` (waiting for `)`)
- After popping a delimiter, must check parent mode stack to determine if still nested
- For `${func(...)}`, the `)` is just part of the expression, not the end of interpolation
- All interpolation tests pass, including triple-level nesting: `"A: ${a + $(b + ${c})}"`
- `\$` escape in strings already worked via ALEX pattern `([^\"\$\\]|\\.)+`
- Single-quoted strings implemented as single token with pattern `'([^'\\]|\\.)*'`
- Prompt interpolation requires excluding `$` from PromptText pattern: `[^{}\s\$]+`
- Interpolation flags (`in_string_interpolation`, `in_prompt_interpolation`) must be cleared when returning to parent mode after closing `${...}` or `$(...)`
- Critical bug: When popping from interpolation back to parent Prompt/InString mode, must clear the interpolation flag to prevent subsequent `}` from incorrectly returning to Prompt/InString instead of Code
- All 52 tests passing including 4 historian example files

---

## Milestone 6: Optional Lexer Enhancements (Deferred)

**Goal:** Additional string and prompt features that may be useful but aren't required for current examples

### Prompt Escaping
- [ ] Implement `\$` escape in Prompt context (literal dollar sign)
- [ ] Handle other useful escapes (`\{`, `\}`, `\\`)
- [ ] Write unit tests for prompt escaping

### String Enhancements
- [ ] Track string delimiter type (double-quote vs single-quote) if needed for parser
- [ ] Raw strings (no escaping at all) - `r"literal\n"`?
- [ ] Multi-line strings with proper indentation handling?
- [ ] Format string mini-language (like Python f-strings)?

### Additional Features
- [ ] Explicit `BackslashEscape` token type if parser needs it
- [ ] Any other lexer features identified during parser development

**Status:** Deferred until needed. Current lexer is feature-complete for all historian examples.
