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

### Milestone 4: Basic Expressions ✅

**Goal:** Parse literals, binary operations, function calls, and member access.

**Status:** COMPLETE

**Tasks:**

1. **Extend Expr enum with literals**
   - [x] Add variants:
     - `Identifier(&'input str)`
     - `Number(&'input str)`
     - `String(StringLiteral)` (simple version without interpolation)
     - `True`
     - `False`

2. **Add operators and complex expressions**
   - [x] Add variants:
     - `Binary { op: BinOp, left: Box<Expr>, right: Box<Expr> }`
     - `Unary { op: UnOp, operand: Box<Expr> }`
     - `Call { callee: Box<Expr>, args: Vec<Expr> }`
     - `Member { object: Box<Expr>, field: &'input str }`
     - `Index { object: Box<Expr>, index: Box<Expr> }`

3. **Define operator enums**
   - [x] `BinOp`: Add, Sub, Mul, Div, Eq, NotEq, Lt, Gt, And, Or, Assign, Pipe, Range
   - [x] `UnOp`: Not, Neg

4. **Add expression grammar rules**
   - [x] Literals: identifier, number, string (no interpolation yet), true, false, self
   - [x] Binary operations with precedence (manual precedence climbing):
     - Assignment: `=` (right-associative, lowest)
     - Pipe: `|` (left-associative)
     - Logical OR: `||` (left-associative)
     - Logical AND: `&&` (left-associative)
     - Comparison: `==`, `!=`, `<`, `>` (left-associative)
     - Range: `...` (left-associative)
     - Arithmetic: `+`, `-` (left-associative)
     - Multiplicative: `*`, `/` (left-associative)
   - [x] Unary: `!`, `-` (right-associative)
   - [x] Call: `Expr "(" (Expr ("," Expr)*)? ")"`
   - [x] Member: `Expr "." identifier`
   - [x] Index: `Expr "[" Expr "]"`
   - [x] Parenthesized: `"(" Expr ")"`

5. **Define StringLiteral (simple version)**
   - [x] `StringLiteral<'input>` struct with single text field (no interpolation)
   - [x] Grammar: Match StringStart, StringText, StringEnd

6. **Test expression parsing**
   - [x] Test: Literals (`42`, `"hello"`, `true`, `foo`)
   - [x] Test: Binary ops (`1 + 2`, `x == y`, `a && b`)
   - [x] Test: Precedence (`1 + 2 * 3` → correct AST)
   - [x] Test: Unary (`!x`, `-5`)
   - [x] Test: Calls (`log(a, b, c)`)
   - [x] Test: Member access (`commit.num`, `plan.length`)
   - [x] Test: Method calls (`self.receive(timeout)`)
   - [x] Test: Index access (`arr[i]`, `data[0]`)
   - [x] Test: Range (`1...3`)
   - [x] Test: Parenthesized expressions (`(x + y) * z`)
   - [x] Test: Complex nested expressions (`self.receive(timeout).status == "success"`)

**Implementation notes:**
- Used manual precedence climbing instead of lalrpop's #[precedence] annotations for better control and clarity
- Each precedence tier is a separate grammar rule (AssignExpr → PipeExpr → OrExpr → ... → UnaryExpr → PostfixExpr → PrimaryExpr)
- This approach avoids shift/reduce conflicts and makes precedence explicit
- Added support for `self` keyword as an identifier
- 38 tests passing (24 from M1-3 + 14 new M4 tests)

**Success criteria:**
- ✅ All basic expression types parse
- ✅ Operator precedence correct
- ✅ Can parse complex nested expressions
- ✅ Function calls and method calls work

---

### Milestone 5: Prompt Expressions (Think/Ask/Do) ✅

**Goal:** Parse the unique patchwork prompt expressions.

**Status:** COMPLETE

**Tasks:**

1. **Define PromptBlock AST**
   - [x] Add `PromptBlock<'input>` struct:
     - `items: Vec<PromptItem>`
   - [x] Add `PromptItem<'input>` enum:
     - `Text(&'input str)` - raw prompt text
     - `Code(Block)` - embedded `do { ... }`

2. **Extend Expr with prompt variants**
   - [x] Add to `Expr` enum:
     - `Think(PromptBlock)` - simplified from original plan (no fallback field)
     - `Ask(PromptBlock)`
     - `Do(Block)` - note: only used inside prompts, not standalone

3. **Add grammar rules for prompt expressions**
   - [x] `ThinkExpr`: `"think" "{" PromptBlock "}"`
     - Note: `think { } || ask { }` is handled by regular `||` binary operator
   - [x] `AskExpr`: `"ask" "{" PromptBlock "}"`
   - [x] `DoExpr`: `"do" "{" StatementList "}"` - only within PromptItem

4. **Parse PromptBlock content**
   - [x] Collect PromptText tokens into Text items
   - [x] Recognize `do {` within prompt and create Code item
   - [x] Handle newlines in prompt blocks (lexer emits them, parser filters)

5. **Test prompt expression parsing**
   - [x] Test: Simple think block
   - [x] Test: Simple ask block
   - [x] Test: Think with fallback (as binary OR expression)
   - [x] Test: Prompt with embedded do block
   - [x] Test: Multiline think blocks
   - [x] Test: Nested prompts in binary expressions

**Implementation notes:**
- Lexer splits prompt text into word-level tokens (one `PromptText` per word)
- Parser collects all prompt items, filtering out newlines which are just formatting
- `think { } || ask { }` pattern uses regular `||` binary operator, not special syntax
- `do { }` blocks are NOT standalone expressions - only used inside think/ask
- 7 new tests added, all passing (44 total tests)

**Success criteria:**
- ✅ Think blocks parse correctly
- ✅ Ask blocks parse correctly
- ✅ Think || ask fallback pattern works (via binary OR)
- ✅ Do blocks inside prompts are recognized
- ✅ Can parse analyst.pw prompt expressions

---

### Milestone 6: String Interpolation ✅

**Goal:** Parse strings with `${...}` and `$(...)` interpolation.

**Status:** COMPLETE

**Tasks:**

1. **Update StringLiteral AST**
   - [x] Change `StringLiteral<'input>` to:
     - `parts: Vec<StringPart>`
   - [x] Add `StringPart<'input>` enum:
     - `Text(&'input str)`
     - `Interpolation(Box<Expr>)`

2. **Parse chunked string tokens**
   - [x] Grammar rule for interpolated strings:
     - Match sequence: StringStart, StringPart*, StringEnd
     - Build StringPart::Text from StringText tokens
     - Build StringPart::Interpolation from expressions

3. **Handle interpolation contexts**
   - [x] Parse expression after Dollar:
     - Identifier → `$var` form
     - LBrace → `${expr}` form (parse until RBrace)
     - LParen → `$(expr)` form (parse until RParen)

4. **Test string interpolation**
   - [x] Test: Simple interpolation `"hello $name"`
   - [x] Test: Multiple interpolations `"$first $last"`
   - [x] Test: Expression interpolation `"Total: ${x + y}"`
   - [x] Test: Parenthesized interpolation `"session-$(timestamp)"`
   - [x] Test: All three forms: `"$id ${expr} $(expr)"`
   - [x] Test: historian examples with interpolation:
     - `"historian-${timestamp}"`
     - `"/tmp/${session_id}"`
     - `"${work_dir}/state.json"`

**Implementation notes:**
- StringLiteral now uses Vec<StringPart> instead of simple text field
- All three interpolation forms work: `$id`, `${expr}`, `$(expr)`
- Interpolation content is parsed as full patchwork expressions
- Note: `$(...)` parses content as patchwork expression, not bash syntax
  - `$(date)` is an identifier, `$(date())` would be a call (but lexer limitation with nested parens)
  - For bash command substitution, use simple identifiers: `$(timestamp)`
- Updated existing string literal test to work with new parts structure
- 6 new tests added, all passing (50 total tests)

**Success criteria:**
- ✅ String interpolation parses correctly
- ✅ All three forms ($id, ${expr}, $(expr)) work
- ✅ Historian string examples parse
- ✅ Zero parser conflicts maintained

---

### Milestone 7: Advanced Expressions ✅

**Goal:** Parse arrays, objects, destructuring, await, and bash substitution.

**Status:** COMPLETE

**Tasks:**

1. **Add array and object literals**
   - [x] Extend `Expr` with:
     - `Array(Vec<Expr>)`
     - `Object(Vec<ObjectField>)`
   - [x] Define `ObjectField<'input>`:
     - `{ key: &'input str, value: Option<Expr> }` (None for shorthand)
   - [x] Grammar rules:
     - Array: `"[" (Expr ("," Expr)*)? "]"`
     - Object: `"{" (ObjectField ("," ObjectField)*)? "}"`
     - ObjectField: `identifier (":" Expr)?` (shorthand or full)

2. **Add destructuring patterns**
   - [x] Define `Pattern<'input>` enum:
     - `Identifier { name, type_ann }` - supports both simple and typed patterns
     - `Object(Vec<ObjectPatternField>)`
   - [x] Define `ObjectPatternField<'input>`:
     - `{ key: &'input str, pattern: Pattern, type_ann: Option<TypeExpr> }`
   - [x] Update VarDecl to use Pattern instead of simple name:
     - `VarDecl { pattern: Pattern, init: Option<Expr> }`
   - [x] Grammar: `"var" Pattern ("=" Expr)?`

3. **Add await expressions**
   - [x] Extend `Expr` with:
     - `Await(Box<Expr>)`
   - [x] Grammar: `"await" Expr` (as part of UnaryExpr)
   - [x] Handle multiple awaits: `await coordinator(a(), b(), c())`
     - Parse as `Await(Call(...))` where args are multiple calls

4. **Add bash substitution**
   - [x] Already supported via string interpolation `$(...)`
   - Note: `$(expr)` parses as patchwork expression, not bash syntax
   - For bash commands, use simple identifiers: `$(timestamp)`

5. **Add output redirection**
   - [x] Pipe operator `|` already exists in binary operators
   - [x] Redirect operator `>` already exists (comparison operator)
   - Note: Semantic interpretation handled in later milestones

6. **Test advanced expressions**
   - [x] Test: Arrays `[1, 2, 3]`, `[{num: 1}, {num: 2}]`
   - [x] Test: Objects `{x: 1, y: 2}`, `{session_id, timestamp}`
   - [x] Test: Destructuring `var {x, y} = obj`
   - [x] Test: Destructuring with types `var {x: string, y: int} = msg`
   - [x] Test: Complex nested structures
   - [x] Test: Await `await foo()`
   - [x] Test: Await with args `await coordinator(a(), b(), c())`
   - Note: Bash and redirect tests deferred (work via existing infrastructure)
   - [x] Test: Complex nested expressions from historian examples

**Implementation notes:**
- Pattern::Identifier changed to struct variant with optional type_ann to support both `var x = ...` and `var x: type = ...`
- Object literal shorthand {x, y} means {x: x, y: y}
- Destructuring with types: `var {x: string, y: int} = obj` supported
- Await is a unary operator like ! and -
- String interpolation `$(...)` already handles bash-like syntax
- All 62 tests passing, zero parser conflicts maintained

**Success criteria:**
- ✅ Array and object literals parse
- ✅ Object shorthand syntax works
- ✅ Destructuring patterns parse
- ✅ Await expressions work
- ✅ Bash substitution recognized (via string interpolation)
- ✅ Zero parser conflicts maintained
- ✅ 62 tests passing (50 from M1-6 + 12 new M7 tests)

---

### Milestone 8: Type System ✅

**Goal:** Parse type annotations and type declarations.

**Status:** COMPLETE

**Tasks:**

1. **Define TypeExpr AST**
   - [x] Expand `TypeExpr<'input>` enum:
     - `Name(&'input str)` - simple types like `string`, `int`
     - `Object(Vec<TypeField>)` - object type `{ x: string, y: int }`
     - `Array(Box<TypeExpr>)` - array type `[string]`
     - `Union(Vec<TypeExpr>)` - union type `"success" | "error"`
     - `Literal(&'input str)` - string literal type `"success"`

2. **Define TypeField AST**
   - [x] `TypeField<'input>` struct:
     - `key: &'input str`
     - `type_expr: TypeExpr`
     - `optional: bool` (for future `key?:` syntax)

3. **Add type declaration**
   - [x] Extend `Item` with:
     - `TypeDecl(TypeDeclItem)`
   - [x] Define `TypeDeclItem<'input>`:
     - `name: &'input str`
     - `type_expr: TypeExpr`
   - [x] Grammar: `"type" identifier "=" TypeExpr`

4. **Add grammar rules for types**
   - [x] TypeExpr rules:
     - Simple name: `identifier`
     - Object type: `"{" (TypeField ("," TypeField)*)? "}"`
     - Array type: `"[" TypeExpr "]"`
     - Union type: `TypeExpr ("|" TypeExpr)+`
     - Literal type: `string` (for string literal types)
   - [x] TypeField: `identifier ":" TypeExpr`

5. **Test type parsing**
   - [x] Test: Simple type `var x: string`
   - [x] Test: Object type in destructuring `var {x: string, y: int} = msg`
   - [x] Test: Type declaration
     ```
     type scribe_result = {
       status: "success" | "error",
       commit_hash: string
     }
     ```
   - [x] Test: Array type `var items: [string]`
   - [x] Test: Union type `status: "success" | "error"`
   - [x] Test: Nested types `[[string]]`, `[{name: string, value: int}]`
   - [x] Test: Complex unions `string | int | "none"`
   - [x] Test: Multiple type declarations in a program

**Implementation notes:**
- Used manual precedence for union types (lowest precedence)
- TypeFieldList allows newlines for formatting (like other lists)
- String literals in type position extract literal text (no interpolation)
- Added 12 comprehensive tests covering all type features
- All 74 tests passing (62 from M1-7 + 12 new M8 tests)
- Zero parser conflicts maintained

**Success criteria:**
- ✅ Type annotations parse in variable declarations
- ✅ Type declarations parse
- ✅ Object types parse
- ✅ Union types parse
- ✅ Array types parse (including nested)
- ✅ Literal types parse
- ✅ Zero parser conflicts maintained
- ✅ 74 tests passing

---

### Milestone 9: Comments & Annotations ✅

**Goal:** Handle comments and decorator-style annotations.

**Status:** COMPLETE

**Tasks:**

1. **Handle comments in lexer/parser**
   - [x] Fixed lexer regex: Changed `# [^\n]*` to `#[^\n]*` to match empty comments
   - [x] Parser adapter filters out Comment tokens (line 171 of adapter.rs)
   - [x] Empty comment lines (standalone `#`) now work correctly

2. **Parse decorator annotations**
   - [x] Decision: Keep as Comment strings (Option 2)
   - [x] Annotations like `# @arg param_name description` and `# @color purple` are treated as comments
   - [x] Semantic analysis or tooling can parse annotation syntax later from comment strings

3. **Test comment handling**
   - [x] Test: Inline comments `var x = 1  # comment`
   - [x] Test: Comments before declarations
   - [x] Test: Comments between statements
   - [x] Test: Decorator annotations (@arg, @color)
   - [x] Test: Multiple comments and code together
   - [x] Test: Comments in expressions, if statements, loops
   - [x] Test: Comments with type annotations
   - [x] Test: Empty comment separators (just `#`)
   - [x] Test: Comment-only files
   - [x] Test: Simplified historian main.pw with comments

**Implementation notes:**
- Fixed lexer bug: `# [^\n]*` required space after `#`, changed to `#[^\n]*`
- Empty comment lines (standalone `#`) are used as visual separators in historian examples
- Comments are filtered in adapter.rs at line 171 along with whitespace
- All decorator annotations (@arg, @color, etc.) work as regular comments
- 13 comprehensive tests added covering all comment scenarios
- 88 total tests passing (74 from M1-8 + 13 new M9 tests + 1 historian test)

**Success criteria:**
- ✅ Comments don't interfere with parsing
- ✅ Empty comments (standalone `#`) work correctly
- ✅ Decorator annotations recognized as comments
- ✅ Historian file patterns with comments parse successfully
- ✅ Zero parser conflicts maintained

---

### Milestone 10: Bare Command Expressions

**Goal:** Parse shell-style bare command invocations with arguments and redirections.

**Status:** IN PROGRESS

**Design:** See [parser-design.md](parser-design.md#bare-command-expressions) for complete design rationale, semantic model, and disambiguation strategy.

**Tasks:**

1. **Add disambiguation token to lexer** ✅
   - [x] Add `IdentifierCall` token that matches `identifier(` with no space
   - [x] Update lexer.alex with `IdentifierCall: <Code> {{ID}}\(` rule (line 76)
   - [x] Place before generic `Identifier` rule for priority matching
   - [x] Update ParserToken enum with `IdentifierCall(&'input str)` (token.rs:46)
   - [x] Update adapter to convert `Rule::IdentifierCall` and strip trailing `(`

2. **Update grammar to handle both token patterns** ✅
   - [x] Add `IdentifierOrCall` helper rule accepting both `identifier` and `identifier_call` (patchwork.lalrpop:126-129)
   - [x] Update SkillDecl to use `IdentifierOrCall` with optional `(` (line 176)
   - [x] Update TaskDecl to use `IdentifierOrCall` with optional `(` (line 184)
   - [x] Update FunctionDecl to use `IdentifierOrCall` with optional `(` (line 192)
   - [x] Update PostfixExpr to handle function calls via `identifier_call` token (line 622)
   - [x] Add method call pattern: `PostfixExpr "." identifier_call ExprList ")"` (line 612)
   - [x] All 94 tests passing with zero parser conflicts

3. **Extend AST for bare commands** ✅
   - [x] Add `Expr::BareCommand` variant (ast.rs:288-291)
   - [x] Add `Expr::CommandSubst` variant (ast.rs:293-296)
   - [x] Add `CommandArg` enum (Literal | String) (ast.rs:214-221)
   - [x] Add `RedirectOp` enum (Out | Append | ErrOut | ErrToOut) (ast.rs:223-230)

4. **Add grammar rules for bare commands** (NEXT)
   - [ ] BareCommand production in statement context
   - [ ] CommandArgs production (one or more CommandArg)
   - [ ] Recognize bare command: `identifier` followed by arguments (not `identifier_call`)

5. **Add redirection grammar**
   - [ ] Extend PostfixExpr with `>`, `>>`, `2>`, `2>&1` operators
   - [ ] Ensure pipe `|` works in command context

6. **Test bare command parsing**
   - [ ] Simple command: `mkdir work_dir`
   - [ ] Command with flags: `mkdir -p work_dir`
   - [ ] Complex args: `date +%Y%m%d-%H%M%S`
   - [ ] Command with interpolation: `mkdir "${work_dir}"`
   - [ ] Command substitution: `var x = $(date +%s)`
   - [ ] Redirections: `echo "text" > file`
   - [ ] Pipes: `cat file | grep "pattern"`
   - [ ] Disambiguation: `f(x)` vs `f x` vs `f (x)`
   - [ ] Command in conditional: `if ! git diff_index --quiet HEAD -- { ... }`
   - [ ] Stderr redirection: `command 2>&1`

7. **Parse all historian example files**
   - [ ] `examples/historian/main.pw` parses completely
   - [ ] `examples/historian/analyst.pw` parses completely
   - [ ] `examples/historian/narrator.pw` parses completely
   - [ ] `examples/historian/scribe.pw` parses completely

8. **Validate AST structure**
   - [ ] Write test helper to dump AST
   - [ ] Verify key structures from historian examples

9. **Error reporting**
   - [ ] Test invalid command syntax
   - [ ] Test helpful error messages

**Implementation notes:**
- See [parser-design.md](parser-design.md#bare-command-expressions) for:
  - Semantic model (commands are scoped variables)
  - Whitespace-sensitive disambiguation strategy
  - Shell operator handling (context-dependent meanings)
  - Type system role in disambiguating `>` and `|`
  - Lexer mode switching details
- `eval` deferred - needs better portable syntax
- Pipe `|` reuses existing `BinOp::Pipe`, type system disambiguates

**Success criteria:**
- ✅ All four historian files parse without errors
- ✅ Bare commands parse correctly with arguments
- ✅ Command substitution `$(...)` works
- ✅ Redirections parse correctly
- ✅ Disambiguation works (function call vs bare command)
- ✅ Zero parser conflicts maintained

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
