# Patchwork Parser Implementation Plan

## Overview

This document breaks down the implementation of the patchwork parser into concrete, testable milestones. Each milestone builds incrementally toward parsing the complete historian examples.

**Architecture:** lalrpop parser generator with external lexer adapter (see [parser-design.md](parser-design.md))

**Validation target:** `examples/historian/*.pw` - four files demonstrating all language features

## Milestones

### Milestone 1: Infrastructure & Token Adapter ✅

**Goal:** Set up the parser crate, lalrpop integration, and token adapter layer.

**Status:** COMPLETE

**Tasks:**

1. **Create parser crate structure**
   - [x] Create `crates/patchwork-parser/` directory
   - [x] Add `Cargo.toml` with dependencies: lalrpop, patchwork-lexer
   - [x] Add lalrpop to build-dependencies
   - [x] Create `build.rs` to run lalrpop generator

2. **Define AST types (initial minimal set)**
   - [x] Deferred to Milestone 2 - started with minimal grammar first
   - Note: Created ParserToken enum instead, AST will come in next milestone

3. **Create token adapter**
   - [x] Create `src/token.rs` with `ParserToken<'input>` enum
     - Maps all lexer `Rule` variants (88 total tokens)
     - Lifetime parameter for string references
     - Variants: Identifier(&'input str), StringText(&'input str), Number(&'input str), etc.
   - [x] Create `src/adapter.rs` with `LexerAdapter<'input>` struct
     - Implements `Iterator<Item = Result<(usize, ParserToken, usize), ParseError>>`
     - Converts lexer tokens to ParserTokens
     - Extracts string slices from input for text-carrying tokens
     - Converts line/column positions to byte offsets for spans
   - [x] Define `ParseError` type in `adapter.rs`
     - Variants: LexerError, UnexpectedToken

4. **Create minimal lalrpop grammar**
   - [x] Create `patchwork.lalrpop` with:
     - `grammar<'input>(input: &'input str);` declaration
     - `extern` block mapping all 88 ParserTokens
     - Single rule: `pub Program: () = { end => () };`
   - [x] Verify lalrpop code generation works

5. **Set up public API**
   - [x] Create `src/lib.rs` with:
     - Module declarations (token, adapter)
     - Include generated parser: `mod patchwork { include!(...) }`
     - Public parse function: `pub fn parse(input: &str) -> Result<(), ParseError>`
   - [x] Re-export token and adapter types

6. **Write basic test**
   - [x] Test parsing empty input successfully
   - [x] Verify build chain works (lexer → adapter → parser)

**Implementation notes:**
- Used parlex 0.3.0 to match lexer dependencies
- Position type uses line/column, not byte offsets - added `position_to_offset` helper
- Handled `Rule::Empty` case by mapping to `ParserToken::End`
- Started minimal - AST types will be added incrementally in next milestones

**Success criteria:**
- ✅ Parser crate builds successfully
- ✅ lalrpop generates parser code
- ✅ Can parse empty input without errors
- ✅ Token adapter correctly converts lexer output

---

### Milestone 2: Top-Level Items & Block Structure ✅

**Goal:** Parse top-level declarations (import, skill, task, fun) and recognize block boundaries.

**Status:** COMPLETE

**Tasks:**

1. **Extend AST for top-level items**
   - [x] Created complete AST in `src/ast.rs`:
     - `Item` enum with Import/Skill/Task/Function variants
     - `ImportDecl` with `ImportPath` (Simple or RelativeMulti)
     - `SkillDecl`, `TaskDecl`, `FunctionDecl` with name, params, body
     - `Param` with name field
     - `Block` with statements vector
     - Placeholder types for Statement/Expr (will expand in Milestone 3+)

2. **Add grammar rules for imports**
   - [x] `ImportDecl`: `"import" ImportPath`
   - [x] `ImportPath`:
     - Simple: `import foo` → ImportPath::Simple(vec!["foo"])
     - RelativeMulti: `import ./{a, b, c}` → ImportPath::RelativeMulti(vec!["a", "b", "c"])
   - [x] Test: Simple imports parse correctly
   - [x] Test: `./{analyst, narrator, scribe}` parses correctly

3. **Add grammar rules for declarations**
   - [x] `SkillDecl`: `"skill" identifier "(" ParamList ")" Block`
   - [x] `TaskDecl`: `"task" identifier "(" ParamList ")" Block`
   - [x] `FunctionDecl`: `"fun" identifier "(" ParamList ")" Block`
   - [x] `ParamList`: Handles empty, single, and comma-separated identifiers
   - [x] `Block`: `"{"  "}"` (empty blocks only for now)

4. **Test top-level parsing**
   - [x] Test: `skill foo() {}` parses correctly
   - [x] Test: `task bar(a, b, c) {}` parses correctly
   - [x] Test: `fun baz(x) {}` parses correctly
   - [x] Test: Multiple items in sequence
   - [x] Test: Historian main.pw structure (import + skill declaration)

**Implementation notes:**
- Whitespace/newline/comment filtering added to LexerAdapter for clean token stream
- Used custom ParamList grammar (not generic Comma helper) to avoid lalrpop conflicts
- ImportPath simplified to Simple (single identifier) for now - can expand later
- Empty blocks only (statement parsing in Milestone 3)
- 9 tests passing, including historian structure validation

**Success criteria:**
- ✅ Can parse all top-level item types
- ✅ Recognizes skill/task/fun with parameter lists
- ✅ Recognizes block boundaries (empty braces)
- ✅ Can parse historian main.pw overall structure

---

### Milestone 3: Simple Statements ✅

**Goal:** Parse variable declarations, expression statements, and control flow.

**Status:** COMPLETE

**Tasks:**

1. **Extend Statement enum**
   - [x] Add variants:
     - `VarDecl { name: &'input str, type_ann: Option<TypeExpr>, init: Option<Expr> }`
     - `Expr(Expr)`
     - `If { condition: Expr, then_block: Block, else_block: Option<Block> }`
     - `For { var: &'input str, iter: Expr, body: Block }`
     - `While { condition: Expr, body: Block }`
     - `Return(Option<Expr>)`
     - `Succeed`
     - `Fail`
     - `Break`

2. **Add grammar rules for statements**
   - [x] `VarDecl`: `"var" identifier (":" TypeExpr)? ("=" Expr)?`
   - [x] `ExprStmt`: `Expr` (newlines/semicolons separate)
   - [x] `If`: `"if" Expr Block ("else" Block)?`
   - [x] `For`: `"for" "var" identifier "in" Expr Block`
   - [x] `While`: `"while" "(" Expr ")" Block`
   - [x] `Return`: `"return" Expr?`
   - [x] `Succeed`: `"succeed"`
   - [x] `Fail`: `"fail"`
   - [x] `Break`: `"break"`

3. **Add placeholder for TypeExpr**
   - [x] Define `TypeExpr<'input>` enum with minimal variants:
     - `Name(&'input str)` - for `string`, `int`, etc.
     - Details deferred to Milestone 8

4. **Add minimal Expr placeholder**
   - [x] Add `Expr::Identifier(&'input str)` variant
   - [x] Add `Expr::Number`, `Expr::True`, `Expr::False` variants
   - [x] Grammar rules for basic literals

5. **Implement Swift-style statement separators**
   - [x] Modified lexer adapter to keep newline tokens
   - [x] Created `Separator` rule (newline or semicolon)
   - [x] Updated `StatementList` to require separators between statements
   - [x] Key insight: newlines SEPARATE statements, enabling `return\nx` to mean return-nothing + x-statement

6. **Test statement parsing**
   - [x] Test: `var x`
   - [x] Test: `var x = y`
   - [x] Test: `var x: string = y`
   - [x] Test: `if cond { ... }`
   - [x] Test: `if cond { ... } else { ... }`
   - [x] Test: `for var i in items { ... }`
   - [x] Test: `while (true) { ... }`
   - [x] Test: `return` and `return value`
   - [x] Test: `succeed`, `fail`, `break`
   - [x] Test: Newline separation (`return\nx` → two statements)
   - [x] Test: Semicolon separation (`x; y; z` → three statements on one line)

**Implementation notes:**
- Adopted Swift's approach: newlines and semicolons act as statement separators
- This elegantly solves LR(1) ambiguity - no conflicts in lalrpop!
- `return\nx` parses as two statements (return with no value, then x)
- `return x` parses as one statement (return with value x)
- Semicolons allow multiple statements on one line
- 24 tests passing (9 from M1-2 + 15 new M3 tests)

**Success criteria:**
- ✅ All statement types parse correctly
- ✅ Blocks can contain statement sequences
- ✅ Can parse control flow with nested blocks
- ✅ Variable declarations with optional type annotations
- ✅ Swift-style optional semicolons working with LR(1) parser

---

### Milestone 4: Basic Expressions

**Goal:** Parse literals, binary operations, function calls, and member access.

**Tasks:**

1. **Extend Expr enum with literals**
   - [ ] Add variants:
     - `Identifier(&'input str)`
     - `Number(&'input str)`
     - `String(StringLiteral)` (simple version without interpolation)
     - `True`
     - `False`

2. **Add operators and complex expressions**
   - [ ] Add variants:
     - `Binary { op: BinOp, left: Box<Expr>, right: Box<Expr> }`
     - `Unary { op: UnOp, operand: Box<Expr> }`
     - `Call { callee: Box<Expr>, args: Vec<Expr> }`
     - `Member { object: Box<Expr>, field: &'input str }`
     - `Index { object: Box<Expr>, index: Box<Expr> }`

3. **Define operator enums**
   - [ ] `BinOp`: Add, Sub, Mul, Div, Eq, NotEq, Lt, Gt, And, Or, Assign, Pipe, Range
   - [ ] `UnOp`: Not, Neg

4. **Add expression grammar rules**
   - [ ] Literals: identifier, number, string (no interpolation yet), true, false
   - [ ] Binary operations with precedence:
     - Pipe: `||` (lowest)
     - Logical: `&&`, `||`
     - Comparison: `==`, `!=`, `<`, `>`
     - Arithmetic: `+`, `-`, `*`, `/`
     - Range: `...`
   - [ ] Unary: `!`, `-`
   - [ ] Call: `Expr "(" (Expr ("," Expr)*)? ")"`
   - [ ] Member: `Expr "." identifier`
   - [ ] Parenthesized: `"(" Expr ")"`

5. **Define StringLiteral (simple version)**
   - [ ] `StringLiteral<'input>` struct with single text field (no interpolation)
   - [ ] Grammar: Match StringStart, StringEnd (ignore StringText for now)

6. **Test expression parsing**
   - [ ] Test: Literals (`42`, `"hello"`, `true`, `foo`)
   - [ ] Test: Binary ops (`1 + 2`, `x == y`, `a && b`)
   - [ ] Test: Precedence (`1 + 2 * 3` → correct AST)
   - [ ] Test: Unary (`!x`, `-5`)
   - [ ] Test: Calls (`log(a, b, c)`)
   - [ ] Test: Member access (`commit.num`, `plan.length`)
   - [ ] Test: Combined (`self.receive(timeout)`)
   - [ ] Test: Range (`1...3`)

**Success criteria:**
- ✅ All basic expression types parse
- ✅ Operator precedence correct
- ✅ Can parse complex nested expressions
- ✅ Function calls and method calls work

---

### Milestone 5: Prompt Expressions (Think/Ask/Do)

**Goal:** Parse the unique patchwork prompt expressions.

**Tasks:**

1. **Define PromptBlock AST**
   - [ ] Add `PromptBlock<'input>` struct:
     - `items: Vec<PromptItem>`
   - [ ] Add `PromptItem<'input>` enum:
     - `Text(&'input str)` - raw prompt text
     - `Code(Block)` - embedded `do { ... }`

2. **Extend Expr with prompt variants**
   - [ ] Add to `Expr` enum:
     - `Think { content: PromptBlock, fallback: Option<Box<Expr>> }`
     - `Ask { content: PromptBlock }`
     - `Do(Block)`

3. **Add grammar rules for prompt expressions**
   - [ ] `ThinkExpr`: `"think" "{" PromptBlock "}" ("||" Expr)?`
     - Handle fallback pattern: `think { ... } || ask { ... }`
   - [ ] `AskExpr`: `"ask" "{" PromptBlock "}"`
   - [ ] `DoExpr`: `"do" "{" Statement* "}"`

4. **Parse PromptBlock content**
   - [ ] Collect PromptText tokens into Text items
   - [ ] Recognize `do {` within prompt and create Code item
   - [ ] Handle nested braces correctly (lexer already tracks depth)

5. **Test prompt expression parsing**
   - [ ] Test: Simple think block
     ```
     var x = think {
       What is the answer?
     }
     ```
   - [ ] Test: Simple ask block
     ```
     var approval = ask {
       Do you approve?
     }
     ```
   - [ ] Test: Think with fallback
     ```
     var cmd = think {
       Figure it out
     } || ask {
       What command?
     }
     ```
   - [ ] Test: Prompt with embedded do block
     ```
     think {
       First analyze the problem.
       do {
         var x = read_file()
       }
       Then explain the solution.
     }
     ```
   - [ ] Test: Parse analyst.pw's think and ask expressions

**Success criteria:**
- ✅ Think blocks parse correctly
- ✅ Ask blocks parse correctly
- ✅ Think || ask fallback pattern works
- ✅ Do blocks inside prompts are recognized
- ✅ Can parse analyst.pw prompt expressions

---

### Milestone 6: String Interpolation

**Goal:** Parse strings with `${...}` and `$(...)` interpolation.

**Tasks:**

1. **Update StringLiteral AST**
   - [ ] Change `StringLiteral<'input>` to:
     - `parts: Vec<StringPart>`
   - [ ] Add `StringPart<'input>` enum:
     - `Text(&'input str)`
     - `Interpolation(Expr)`

2. **Parse chunked string tokens**
   - [ ] Grammar rule for interpolated strings:
     - Match sequence: StringStart, (StringText | Expr)*, StringEnd
     - Build StringPart::Text from StringText tokens
     - Build StringPart::Interpolation from expressions between tokens

3. **Handle interpolation contexts**
   - [ ] After StringStart or StringText, check for:
     - Another StringText → add Text part
     - Dollar token → begin interpolation expression
     - StringEnd → finish string
   - [ ] Parse expression after Dollar:
     - Identifier → `$var` form
     - LBrace → `${expr}` form (parse until RBrace)
     - LParen → `$(cmd)` form (parse until RParen)

4. **Test string interpolation**
   - [ ] Test: Simple interpolation `"hello ${name}"`
   - [ ] Test: Multiple interpolations `"${a} and ${b}"`
   - [ ] Test: Bash substitution `"session-$(date)"`
   - [ ] Test: Nested interpolation `"outer ${inner + "nested"}"`
   - [ ] Test: All three forms: `"$id ${expr} $(cmd)"`
   - [ ] Test: historian examples with interpolation:
     - `"historian-${timestamp}"`
     - `"/tmp/${session_id}"`
     - `"${work_dir}/state.json"`

**Success criteria:**
- ✅ String interpolation parses correctly
- ✅ All three forms ($id, ${expr}, $(cmd)) work
- ✅ Nested interpolation handles correctly
- ✅ Historian string examples parse

---

### Milestone 7: Advanced Expressions

**Goal:** Parse arrays, objects, destructuring, await, and bash substitution.

**Tasks:**

1. **Add array and object literals**
   - [ ] Extend `Expr` with:
     - `Array(Vec<Expr>)`
     - `Object(Vec<ObjectField>)`
   - [ ] Define `ObjectField<'input>`:
     - `{ key: &'input str, value: Option<Expr> }` (None for shorthand)
   - [ ] Grammar rules:
     - Array: `"[" (Expr ("," Expr)*)? "]"`
     - Object: `"{" (ObjectField ("," ObjectField)*)? "}"`
     - ObjectField: `identifier (":" Expr)?` (shorthand or full)

2. **Add destructuring patterns**
   - [ ] Define `Pattern<'input>` enum:
     - `Identifier(&'input str)`
     - `Object(Vec<ObjectPatternField>)`
   - [ ] Define `ObjectPatternField<'input>`:
     - `{ key: &'input str, pattern: Pattern, type_ann: Option<TypeExpr> }`
   - [ ] Update VarDecl to use Pattern instead of simple name:
     - `VarDecl { pattern: Pattern, init: Option<Expr> }`
   - [ ] Grammar: `"var" Pattern ("=" Expr)?`

3. **Add await expressions**
   - [ ] Extend `Expr` with:
     - `Await(Box<Expr>)`
   - [ ] Grammar: `"await" Expr`
   - [ ] Handle multiple awaits: `await task a(), b(), c()`
     - Parse as `Await(Call(...))` where args are multiple calls

4. **Add bash substitution**
   - [ ] Extend `Expr` with:
     - `BashSubst(Vec<BashToken>)` (or simpler: `BashSubst(Expr)`)
   - [ ] For now, parse `$(...)` as BashSubst containing token sequence
   - [ ] Later milestone can parse bash syntax if needed

5. **Add output redirection**
   - [ ] Extend `Expr` with:
     - `Pipe { left: Box<Expr>, right: Box<Expr> }` (using `|` operator)
     - `Redirect { expr: Box<Expr>, target: Box<Expr> }` (using `>` operator)
   - [ ] Grammar: Binary operators for `|` and `>`

6. **Test advanced expressions**
   - [ ] Test: Arrays `[1, 2, 3]`, `[{num: 1}, {num: 2}]`
   - [ ] Test: Objects `{x: 1, y: 2}`, `{session_id, timestamp}`
   - [ ] Test: Destructuring `var {x, y} = obj`
   - [ ] Test: Destructuring with types `var {x: string, y: int} = msg`
   - [ ] Test: Nested destructuring `var {commits: [{num, description}]} = plan`
   - [ ] Test: Await `await task foo()`
   - [ ] Test: Multiple awaits `await task a(), b(), c()`
   - [ ] Test: Bash subst `$(date +%Y%m%d)`
   - [ ] Test: Output redirect `cat(obj) > "${file}"`
   - [ ] Test: Parse main.pw's complex expressions

**Success criteria:**
- ✅ Array and object literals parse
- ✅ Object shorthand syntax works
- ✅ Destructuring patterns parse
- ✅ Await expressions work
- ✅ Bash substitution recognized
- ✅ main.pw parses completely

---

### Milestone 8: Type System

**Goal:** Parse type annotations and type declarations.

**Tasks:**

1. **Define TypeExpr AST**
   - [ ] Expand `TypeExpr<'input>` enum:
     - `Name(&'input str)` - simple types like `string`, `int`
     - `Object(Vec<TypeField>)` - object type `{ x: string, y: int }`
     - `Array(Box<TypeExpr>)` - array type `[string]`
     - `Union(Vec<TypeExpr>)` - union type `"success" | "error"`
     - `Literal(&'input str)` - string literal type `"success"`

2. **Define TypeField AST**
   - [ ] `TypeField<'input>` struct:
     - `key: &'input str`
     - `type_expr: TypeExpr`
     - `optional: bool` (for future `key?:` syntax)

3. **Add type declaration**
   - [ ] Extend `Item` with:
     - `TypeDecl(TypeDeclItem)`
   - [ ] Define `TypeDeclItem<'input>`:
     - `name: &'input str`
     - `type_expr: TypeExpr`
   - [ ] Grammar: `"type" identifier "=" TypeExpr`

4. **Add grammar rules for types**
   - [ ] TypeExpr rules:
     - Simple name: `identifier`
     - Object type: `"{" (TypeField ("," TypeField)*)? "}"`
     - Array type: `"[" TypeExpr "]"`
     - Union type: `TypeExpr ("|" TypeExpr)+`
     - Literal type: `string` (for string literal types)
   - [ ] TypeField: `identifier ":" TypeExpr`

5. **Test type parsing**
   - [ ] Test: Simple type `var x: string`
   - [ ] Test: Object type in destructuring `var {x: string, y: int} = msg`
   - [ ] Test: Type declaration
     ```
     type scribe_result = {
       status: "success" | "error",
       commit_hash: string
     }
     ```
   - [ ] Test: Array type `var items: [string]`
   - [ ] Test: Union type `status: "success" | "error"`
   - [ ] Test: Parse narrator.pw's type declarations

**Success criteria:**
- ✅ Type annotations parse in variable declarations
- ✅ Type declarations parse
- ✅ Object types parse
- ✅ Union types parse
- ✅ narrator.pw parses completely with types

---

### Milestone 9: Comments & Annotations

**Goal:** Handle comments and decorator-style annotations.

**Tasks:**

1. **Handle comments in lexer/parser**
   - [ ] Decision: Ignore comments during parsing or preserve in AST?
   - [ ] For now: Lexer already emits Comment tokens, parser ignores them
   - [ ] Future: Add comment preservation for doc generation

2. **Parse decorator annotations**
   - [ ] Recognize patterns like:
     - `# @arg param_name description`
     - `# @color purple`
   - [ ] Option 1: Parse as structured Annotation nodes
   - [ ] Option 2: Keep as Comment strings, parse later
   - [ ] Decision: Start with Option 2 (keep as comments)

3. **Test comment handling**
   - [ ] Test: Comments don't break parsing
   - [ ] Test: Inline comments `var x = 1  # comment`
   - [ ] Test: Block comments before declarations
   - [ ] Test: Parse all historian files with comments preserved/ignored

**Success criteria:**
- ✅ Comments don't interfere with parsing
- ✅ All historian files parse with comments present

---

### Milestone 10: Full Historian Example Validation

**Goal:** Successfully parse all four historian example files and validate AST structure.

**Tasks:**

1. **Parse all example files**
   - [ ] Test: `examples/historian/main.pw` parses completely
   - [ ] Test: `examples/historian/analyst.pw` parses completely
   - [ ] Test: `examples/historian/narrator.pw` parses completely
   - [ ] Test: `examples/historian/scribe.pw` parses completely

2. **Validate AST structure**
   - [ ] Write test helper to dump AST as formatted output
   - [ ] Verify key structures parse correctly:
     - main.pw: skill declaration with await task calls
     - analyst.pw: think/ask expressions, variable destructuring
     - narrator.pw: for loop, type declarations, function definition
     - scribe.pw: while loop, nested do blocks in think

3. **Compare with lexer tests**
   - [ ] Ensure parser tests align with existing lexer tests
   - [ ] Both should successfully process the same examples
   - [ ] Parser produces meaningful AST, not just tokens

4. **Error reporting test**
   - [ ] Test intentionally malformed input
   - [ ] Verify error messages are helpful
   - [ ] Check error position tracking

5. **Performance check**
   - [ ] Parse all four files in reasonable time (<100ms combined)
   - [ ] No unnecessary allocations or copies

**Success criteria:**
- ✅ All four historian files parse without errors
- ✅ AST structure matches expected program semantics
- ✅ Error messages are clear and helpful
- ✅ Parser performance is acceptable

---

## Testing Strategy

### Per-Milestone Testing

Each milestone includes specific tests inline. General approach:

1. **Unit tests** - Test individual grammar rules
   - Example: Test that `"var x = 1"` produces correct VarDecl AST node

2. **Snippet tests** - Test small code fragments
   - Example: Test parsing a simple function declaration

3. **Integration tests** - Test parsing complete files
   - Example: Parse full historian examples

4. **Regression tests** - Previous milestones keep passing
   - Never remove tests from earlier milestones
   - Each new milestone runs all previous tests

### Test Organization

```
crates/patchwork-parser/
├─ src/
│  └─ lib.rs              # Inline unit tests
└─ tests/
   ├─ statements.rs       # Statement parsing tests
   ├─ expressions.rs      # Expression parsing tests
   ├─ prompts.rs          # Prompt expression tests
   ├─ strings.rs          # String interpolation tests
   └─ historian.rs        # Full example tests
```

### Test Helpers

```rust
// Helper to parse and assert success
fn parse_expr(input: &str) -> Expr {
    let result = parse(input).unwrap();
    // Extract expression from program...
}

// Helper to assert parse failure
fn assert_parse_error(input: &str) {
    assert!(parse(input).is_err());
}

// Helper to dump AST for debugging
fn dump_ast(ast: &Program) -> String {
    // Pretty-print AST structure
}
```

## Key Decisions & Patterns

### Decision 1: Precedence Strategy

Use lalrpop's precedence levels for binary operators:
```
Tier 1 (highest): Member access (.), Call (())
Tier 2: Unary (!, -)
Tier 3: Multiplicative (*, /)
Tier 4: Additive (+, -)
Tier 5: Range (...)
Tier 6: Comparison (==, !=, <, >)
Tier 7: Logical AND (&&)
Tier 8: Logical OR (||)
Tier 9: Pipe (|)
Tier 10: Assignment (=)
Tier 11 (lowest): Redirect (>)
```

### Decision 2: String Interpolation Parsing

The lexer emits chunked tokens (StringStart, StringText, StringEnd) with Dollar tokens for interpolation. The parser:
1. Matches StringStart
2. Loops collecting StringText or parsing interpolation expressions
3. Matches StringEnd
4. Builds StringLiteral with parts vector

### Decision 3: Prompt Block Parsing

PromptText tokens come from the lexer as large chunks. The parser:
1. Collects consecutive PromptText tokens into Text items
2. Recognizes Do token + LBrace as start of embedded code
3. Parses the code block
4. Continues collecting prompt text

### Decision 4: Optional Semicolons

Following JavaScript convention, semicolons are optional. The grammar:
- Allows `Expr ";"?` for expression statements
- Doesn't require semicolons after blocks

### Decision 5: Error Recovery

Initial implementation: minimal error recovery
- Let lalrpop's default error handling work
- Focus on clear error messages
- Future: Add error recovery for common mistakes

## Deferred to Future Milestones

These features are NOT in historian examples but may be needed later:

1. **Match expressions** - Pattern matching like Rust
2. **Generics** - Type parameters
3. **Macros** - Code generation
4. **Async/await semantics** - Beyond basic await syntax
5. **Module system** - Beyond basic imports
6. **Operator overloading**
7. **Traits/interfaces**

## Success Metrics

Overall success criteria for the parser implementation:

- ✅ All 10 milestones completed
- ✅ All four historian examples parse successfully
- ✅ AST accurately represents program structure
- ✅ Tests cover all language features
- ✅ Error messages are helpful
- ✅ Parser performance <100ms for historian examples
- ✅ Code is maintainable and well-documented

## Next Steps After Completion

Once the parser is complete:

1. **AST Visitor Pattern** - For traversing and transforming AST
2. **Pretty Printer** - Convert AST back to source code
3. **Semantic Analysis** - Type checking, scope resolution
4. **Interpreter** - Execute patchwork programs
5. **Code Generation** - Compile to target language

But first: let's build the parser! 🚀
