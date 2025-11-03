# Patchwork Lexer Implementation Plan

**Goal:** Implement a context-aware lexer for patchwork that can successfully tokenize the historian examples.

**Approach:** Build incrementally with unit tests, starting from infrastructure through to full example validation.

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

## Milestone 4: Full Example Validation

**Goal:** Successfully lex all historian examples and handle real-world patterns

### Bash substitution
- [ ] Implement `$(...)` token pattern for bash command substitution
- [ ] Write unit tests for bash substitution

### Example file testing
- [ ] Test lexer on `examples/historian/main.pw`
- [ ] Test lexer on `examples/historian/analyst.pw`
- [ ] Test lexer on `examples/historian/narrator.pw`
- [ ] Test lexer on `examples/historian/scribe.pw`

### Edge case refinement
- [ ] Identify and fix any tokenization errors from example files
- [ ] Refine token set based on actual usage patterns
- [ ] Document any modifications made to example files
- [ ] Add unit tests for discovered edge cases

### Final validation
- [ ] Verify all examples tokenize without errors
- [ ] Review token streams for parser readiness
- [ ] Document any known limitations or future work
