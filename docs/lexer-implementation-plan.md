# Patchwork Lexer Implementation Plan

**Goal:** Implement a context-aware lexer for patchwork that can successfully tokenize the historian examples.

**Approach:** Build incrementally with unit tests, starting from infrastructure through to full example validation.

---

## Milestone 1: Infrastructure & Build Setup

**Goal:** Get parlex-gen integrated and compiling

### Build infrastructure
- [ ] Create `crates/patchwork-lexer/lexer.x` with minimal ALEX specification
- [ ] Set up `build.rs` to invoke parlex-gen during cargo build
- [ ] Add parlex-gen dependency to Cargo.toml
- [ ] Verify `cargo build` successfully generates lexer code

### Basic project structure
- [ ] Create `src/lib.rs` with public lexer interface
- [ ] Create `tests/` directory for unit tests
- [ ] Write initial smoke test that creates a lexer and tokenizes empty input

---

## Milestone 2: Code State Tokens

**Goal:** Handle basic tokenization in Code state only (no state transitions yet)

### Keywords and identifiers
- [ ] Implement keyword tokens (import, from, var, const, if, else, while, for, async, await, return)
- [ ] Implement identifier token pattern
- [ ] Write unit tests for keywords vs identifiers

### Literals
- [ ] Implement string literals (basic double-quoted strings first)
- [ ] Implement number literals (integers and floats)
- [ ] Implement boolean literals (true, false)
- [ ] Write unit tests for all literal types

### Operators and punctuation
- [ ] Implement comparison operators (==, ===, !=, !==, <, >, <=, >=)
- [ ] Implement arithmetic operators (+, -, *, /, %)
- [ ] Implement logical operators (&&, ||, !)
- [ ] Implement assignment operators (=, +=, -=, etc.)
- [ ] Implement punctuation ({, }, (, ), [, ], ;, ,, ., :)
- [ ] Write unit tests for operators and punctuation

### Whitespace and comments
- [ ] Implement whitespace handling
- [ ] Implement single-line comments (//)
- [ ] Implement multi-line comments (/* */)
- [ ] Write unit tests for comments (including edge cases)

### String interpolation
- [ ] Implement `${}` detection within strings
- [ ] Create tokens for string parts and interpolation boundaries
- [ ] Write unit tests for string interpolation

### Integration test
- [ ] Write test that tokenizes a simple code snippet with variables, expressions, and comments

---

## Milestone 3: State Transitions & Prompt Handling

**Goal:** Implement context-aware lexing with state stack for think/ask/do operators

### State infrastructure
- [ ] Define Code and Prompt lexer states in ALEX spec
- [ ] Implement state stack mechanism for tracking nested contexts
- [ ] Implement brace depth tracking

### Prompt operator transitions
- [ ] Implement `think` keyword that transitions Code → Prompt
- [ ] Implement `ask` keyword that transitions Code → Prompt
- [ ] Write unit tests for simple think/ask blocks

### Code operator transitions
- [ ] Implement `do` keyword with lookahead for `{` that transitions Prompt → Code
- [ ] Ensure `do` as identifier works in Code state
- [ ] Write unit tests for do transitions

### Prompt text handling
- [ ] Implement PromptText token (captures text in Prompt state)
- [ ] Handle braces within prompt text
- [ ] Handle `do` without `{` as regular prompt text
- [ ] Write unit tests for prompt text edge cases

### Nested context handling
- [ ] Test simple nesting: `think { ... do { ... } }`
- [ ] Test complex nesting: `think { ... do { ... think { ... } } }`
- [ ] Write unit tests for multiple levels of nesting

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
