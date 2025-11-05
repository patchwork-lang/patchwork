# Patchwork Parser Design

## Overview

The patchwork parser will transform the token stream from our custom lexer into an Abstract Syntax Tree (AST) representing the program structure. We'll use **lalrpop** as our parser generator, integrating it with our existing parlex-gen-based lexer.

## Integration Architecture

### lalrpop Integration Pattern

**Tool:** [lalrpop](http://lalrpop.github.io/lalrpop/) - LR(1) parser generator for Rust

**Integration approach:** External lexer adapter pattern
- Our lexer produces tokens via the parlex-gen framework
- We create an adapter that implements `Iterator<Item = Spanned<Token, usize, ParseError>>`
- lalrpop consumes this iterator to build the AST

### Token Adapter Layer

**Purpose:** Bridge between parlex-gen lexer and lalrpop parser expectations

**Key components:**

1. **`ParserToken<'input>` enum** - Wraps our lexer's `TokenType` with lifetime for string references
   ```rust
   pub enum ParserToken<'input> {
       // Keywords
       Import, Var, If, Else, For, While, Await, Task, Skill, Fun, Type,
       Return, Succeed, Fail, Break, Self_, In,

       // Context operators
       Think, Ask, Do,

       // Literals with references to input
       Identifier(&'input str),
       StringStart,
       StringText(&'input str),
       StringEnd,
       Number(&'input str),
       PromptText(&'input str),

       // Operators and punctuation (no data)
       LBrace, RBrace, LParen, RParen, LBracket, RBracket,
       Assign, Eq, NotEq, Not, Or, And,
       Plus, Minus, Star, Slash, Dot, Arrow, Pipe, Ampersand,
       Lt, Gt, Ellipsis, Comma, Semi, Colon, At,
       Dollar,

       // Special
       Comment(&'input str),
       Whitespace,
       End,
   }
   ```

2. **`LexerAdapter<'input>` struct** - Adapts our lexer to lalrpop's expected interface
   ```rust
   pub struct LexerAdapter<'input> {
       input: &'input str,
       lexer: LexerDriver<IterInput<std::slice::Iter<'input, u8>>>,
       position: usize,
   }

   impl<'input> Iterator for LexerAdapter<'input> {
       type Item = Spanned<ParserToken<'input>, usize, ParseError>;

       fn next(&mut self) -> Option<Self::Item> {
           // Convert lexer tokens to ParserTokens
           // Extract string slices from input for Identifier, StringText, etc.
           // Track position for span information
       }
   }
   ```

3. **lalrpop grammar declaration**
   ```
   grammar<'input>(input: &'input str);

   extern {
       type Location = usize;
       type Error = ParseError;

       enum ParserToken<'input> {
           "import" => ParserToken::Import,
           "var" => ParserToken::Var,
           // ... map all tokens
           "identifier" => ParserToken::Identifier(<&'input str>),
           "string_text" => ParserToken::StringText(<&'input str>),
           // ...
       }
   }
   ```

**Why lifetimes?**
- Efficiency: Avoid copying identifier names and string content
- lalrpop supports this naturally via the `<'input>` lifetime parameter
- Tokens borrow from the original input string

## AST Structure

### Design Principles

1. **Represent structure, not syntax** - AST should capture semantics, not parsing details
2. **Incremental complexity** - Start simple, add detail milestone by milestone
3. **Type safety** - Use Rust's type system to make invalid ASTs unrepresentable
4. **Span tracking** - Track source locations for error reporting

### Core AST Nodes

**Program-level:**
```rust
pub struct Program<'input> {
    pub items: Vec<Item<'input>>,
}

pub enum Item<'input> {
    Import(ImportDecl<'input>),
    Skill(SkillDecl<'input>),
    Task(TaskDecl<'input>),
    Function(FunctionDecl<'input>),
    TypeDecl(TypeDecl<'input>),
}
```

**Declarations:**
```rust
pub struct SkillDecl<'input> {
    pub name: &'input str,
    pub params: Vec<Param<'input>>,
    pub body: Block<'input>,
}

pub struct TaskDecl<'input> {
    pub name: &'input str,
    pub params: Vec<Param<'input>>,
    pub body: Block<'input>,
}

pub struct FunctionDecl<'input> {
    pub name: &'input str,
    pub params: Vec<Param<'input>>,
    pub body: Block<'input>,
}
```

**Statements and Blocks:**
```rust
pub enum Statement<'input> {
    VarDecl {
        name: &'input str,
        init: Option<Expr<'input>>,
    },
    Expr(Expr<'input>),
    If {
        condition: Expr<'input>,
        then_block: Block<'input>,
        else_block: Option<Block<'input>>,
    },
    For {
        var: &'input str,
        iter: Expr<'input>,
        body: Block<'input>,
    },
    While {
        condition: Expr<'input>,
        body: Block<'input>,
    },
    Return(Option<Expr<'input>>),
    Succeed,
    Fail,
    Break,
}

pub struct Block<'input> {
    pub statements: Vec<Statement<'input>>,
}
```

**Expressions:**
```rust
pub enum Expr<'input> {
    // Literals
    Identifier(&'input str),
    Number(&'input str),  // Keep as string initially, parse later
    String(StringLiteral<'input>),

    // Prompt expressions - the unique patchwork feature!
    Think {
        content: PromptBlock<'input>,
        fallback: Option<Box<Expr<'input>>>,  // For || ask { ... }
    },
    Ask {
        content: PromptBlock<'input>,
    },

    // Code block as expression
    Do(Block<'input>),

    // Binary operations
    Binary {
        op: BinOp,
        left: Box<Expr<'input>>,
        right: Box<Expr<'input>>,
    },

    // Unary operations
    Unary {
        op: UnOp,
        operand: Box<Expr<'input>>,
    },

    // Function/task calls
    Call {
        callee: Box<Expr<'input>>,
        args: Vec<Expr<'input>>,
    },

    // Member access
    Member {
        object: Box<Expr<'input>>,
        field: &'input str,
    },

    // Array/object literals
    Array(Vec<Expr<'input>>),
    Object(Vec<(&'input str, Expr<'input>)>),

    // Bash substitution
    BashSubst(Vec<BashToken<'input>>),

    // Await expression
    Await(Box<Expr<'input>>),
}
```

**Prompt Blocks - The Core Innovation:**
```rust
pub struct PromptBlock<'input> {
    pub items: Vec<PromptItem<'input>>,
}

pub enum PromptItem<'input> {
    // Raw text content
    Text(&'input str),

    // Embedded code expressions via do { ... }
    Code(Block<'input>),

    // String interpolation within prompt text (future milestone)
    Interpolation(Expr<'input>),
}
```

**String Literals:**
```rust
pub struct StringLiteral<'input> {
    pub parts: Vec<StringPart<'input>>,
}

pub enum StringPart<'input> {
    Text(&'input str),
    Interpolation(Expr<'input>),  // ${...} or $(...)
}
```

**Operators:**
```rust
pub enum BinOp {
    // Arithmetic
    Add, Sub, Mul, Div,

    // Comparison
    Eq, NotEq, Lt, Gt,

    // Logical
    And, Or,

    // Other
    Assign, Pipe, Member, Range,
}

pub enum UnOp {
    Not, Neg,
}
```

## Language Constructs Analysis

Based on the historian examples, here are all the language features we need to support:

### Top-Level Constructs
- ✅ **Import declarations**: `import ./{analyst, narrator}`, `import std.log`
- ✅ **Skill declarations**: `skill rewriting_git_branch(params) { ... }`
- ✅ **Task declarations**: `task analyst(params) { ... }`
- ✅ **Function declarations**: `fun validate_trees(params) { ... }`
- ✅ **Type declarations**: `type scribe_result = { ... }`

### Statements
- ✅ **Variable declarations**: `var timestamp = $(date +%Y%m%d)`
- ✅ **If statements**: `if ! git diff_index ... { ... }`
- ✅ **For loops**: `for var commit in commits { ... }`
- ✅ **While loops**: `while(true) { ... }`
- ✅ **Return statements**: `return` or `return value`
- ✅ **Succeed/Fail**: `succeed`, `fail`
- ✅ **Break**: `break`
- ✅ **Expression statements**: `echo "..."`, `mkdir -p work_dir`

### Expressions

**Literals:**
- ✅ **Strings**: `"historian-${timestamp}"`
- ✅ **Numbers**: `0`, `1`, `3`, `10800000`
- ✅ **Identifiers**: `session_id`, `work_dir`
- ✅ **Objects**: `{ session_id, timestamp, original: current_branch }`
- ✅ **Arrays**: `[{num: 1, description: "..."}, ...]`

**Prompt Expressions (unique to patchwork!):**
- ✅ **Think blocks**: `think { ... }`
- ✅ **Ask blocks**: `ask { ... }`
- ✅ **Think with fallback**: `think { ... } || ask { ... }`
- ✅ **Do blocks in prompts**: Prompt text with embedded `do { ... }` code

**Operations:**
- ✅ **Binary operators**: `+`, `-`, `=`, `!=`, `!`, `||`, `&&`
- ✅ **Unary operators**: `!`, `-` (negation)
- ✅ **Member access**: `commit.num`, `commit.description`, `commit_plan.length`
- ✅ **Function calls**: `log(session_id, "ANALYST", "Starting")`, `mkdir -p "${dir}"`
- ✅ **Method calls**: `self.receive(10800000)`, `narrator.send(...)`
- ✅ **Bash substitution**: `$(date +%Y%m%d-%H%M%S)`, `$(git rev_parse ...)`
- ✅ **Await**: `await task analyst(...), narrator(...), scribe(...)`

**String Interpolation:**
- ✅ **Variable interpolation**: `"${timestamp}"`, `"${work_dir}"`
- ✅ **Expression interpolation**: `"${commits.length}"`, `"${commit.num}"`
- ✅ **Nested interpolation**: `"historian-${timestamp}"` inside bash command

**Destructuring:**
- ✅ **Object destructuring**: `var { type, branch, clean_branch, ... } = self.receive(...)`
- ✅ **Type annotations in destructuring**: `var { commit_num: int, description: string } = message`

**Range Expressions:**
- ✅ **Ranges**: `1...3` (in for loops)

### Special Features

**Pipe to file:**
- ✅ **Output redirection**: `cat({...}) > "${work_dir}/state.json"`

**Command execution:**
- ✅ **Shell command patterns**: Commands like `mkdir`, `echo`, `git`, `eval` appear as function calls

**Type annotations:**
- ✅ **Inline type declarations**: `type scribe_result = { ... }`
- ✅ **Type annotations**: `var result: scribe_result = ...`
- ✅ **Union types**: `status: "success" | "error"`

**Comments:**
- ✅ **Single-line comments**: `# Comment text`
- ✅ **Documentation comments**: `# @arg param_name` (decorator-style)
- ✅ **Color annotations**: `# @color purple`

## Implementation Strategy

### Phased Approach

We'll implement the parser in stages, starting with the structural skeleton and progressively adding detail:

**Milestone 1: Block Structure Recognition**
- Parse top-level items (import, skill, task, fun)
- Recognize blocks and their boundaries
- Handle think/ask/do operators correctly
- **Goal:** Successfully parse the overall structure without detailed expression parsing

**Milestone 2: Simple Statements**
- Variable declarations
- Expression statements (function calls, commands)
- Control flow (if/for/while/return)
- **Goal:** Parse statement-level structure

**Milestone 3: Basic Expressions**
- Literals (identifiers, numbers, strings without interpolation)
- Binary operators
- Function calls
- **Goal:** Parse simple expressions like `x = 1 + 2` and `log(id, "text")`

**Milestone 4: Prompt Expressions**
- Think blocks with prompt content
- Ask blocks with prompt content
- Think || ask fallback pattern
- Do blocks embedded in prompts
- **Goal:** Correctly parse the unique think/ask/do expressions

**Milestone 5: String Interpolation**
- Parse StringStart/StringText/StringEnd token sequences
- Build StringLiteral AST nodes with interpolation
- Handle nested interpolation
- **Goal:** Strings like `"historian-${timestamp}"` parse correctly

**Milestone 6: Advanced Expressions**
- Object and array literals
- Destructuring patterns
- Member access
- Bash substitution
- Await expressions
- **Goal:** Parse all expression forms in historian examples

**Milestone 7: Type System**
- Type annotations
- Type declarations
- Union types
- **Goal:** Parse type syntax (semantic checking comes later)

**Milestone 8: Full Historian Example**
- Parse all four historian files successfully
- Validate AST structure
- **Goal:** Complete parser for all features in examples

### Grammar Organization

The lalrpop grammar will be organized hierarchically:

```
Program
  ├─ Items (Import | Skill | Task | Function | TypeDecl)
  │   └─ Parameters
  │   └─ Block
  │       └─ Statements
  │           └─ Expressions
  │               ├─ Literals
  │               ├─ Prompt Blocks (Think/Ask/Do)
  │               ├─ Binary/Unary Operations
  │               ├─ Calls
  │               └─ String Interpolation
  └─ (End of file)
```

### Error Handling

**Strategy:**
- lalrpop provides basic error recovery
- Use custom `ParseError` type for detailed error messages
- Track source spans for all AST nodes (future enhancement)
- Initially focus on clear error messages over recovery

**Error types:**
```rust
pub enum ParseError {
    UnexpectedToken { expected: String, found: String },
    UnexpectedEof,
    LexerError(String),
}
```

## Testing Strategy

**Per-milestone testing:**
1. **Unit tests** - Test individual grammar rules in isolation
2. **Snippet tests** - Parse small code fragments for specific features
3. **Example tests** - Parse historian example files
4. **Roundtrip tests** - Parse → pretty-print → parse (future)

**Test organization:**
- Tests in `crates/patchwork-parser/src/lib.rs` or separate `tests/` directory
- Each milestone adds new test cases
- Never remove passing tests from previous milestones

## Open Questions & Future Considerations

### Questions to Resolve

1. **Object literal syntax**: Do we need to distinguish between `{key: value}` and `{key}` shorthand?
   - Examples show both forms: `{ session_id, timestamp }` and `{ type: "done" }`

2. **Command vs function call**: How do we distinguish `echo "text"` from `log("text")`?
   - Both appear as function call syntax in AST initially
   - Semantic analysis phase will distinguish built-in commands vs functions

3. **Destructuring complexity**: How deeply nested can destructuring patterns be?
   - Example: `var { commits: [{num: number, description: string}] } = ...`
   - Start with one level, expand as needed

4. **Type annotation syntax**: Where can type annotations appear?
   - Variable declarations: `var x: string = ...`
   - Destructuring: `var { x: string, y: int } = ...`
   - Function parameters (not seen in examples yet)
   - Return types (not seen in examples yet)

### Future Enhancements

1. **Span tracking** - Add `Span { start: usize, end: usize }` to all AST nodes for error reporting
2. **Comments in AST** - Currently ignored, but might want to preserve for doc generation
3. **Pretty printer** - Convert AST back to source code (useful for refactoring tools)
4. **Semantic analysis** - Type checking, scope analysis (separate from parser)
5. **Macro expansion** - If we add macro support
6. **Error recovery** - Better handling of syntax errors to continue parsing

## Integration with Build System

**Build process:**
1. `build.rs` runs lalrpop to generate parser from `.lalrpop` file
2. Generated parser code goes to `OUT_DIR` (similar to lexer)
3. `lib.rs` includes generated parser via `include!` macro
4. Parser module exports public API

**File structure:**
```
crates/patchwork-parser/
├─ build.rs           # Runs lalrpop generator
├─ patchwork.lalrpop  # Grammar specification
├─ src/
│  ├─ lib.rs          # Public API, includes generated parser
│  ├─ ast.rs          # AST node definitions
│  ├─ token.rs        # ParserToken enum
│  └─ adapter.rs      # LexerAdapter implementation
└─ Cargo.toml         # Dependencies: lalrpop, parlex-gen
```

**Dependencies:**
```toml
[dependencies]
parlex-gen = { path = "../parlex-gen" }
patchwork-lexer = { path = "../patchwork-lexer" }

[build-dependencies]
lalrpop = "0.20"
```

## Bare Command Expressions

### Design Overview

Bare commands provide shell-like syntax for invoking executables and functions in a portable, OS-agnostic way. This is a key patchwork feature enabling seamless mixing of external process invocations and in-process function calls.

### Semantic Model

**Command names are variables in scope:**
- `mkdir` is not a hardcoded shell command - it's a variable that must be in scope
- Type can be:
  - `Cmd` - executable that runs in external process
  - `(string...) -> Proc` - patchwork function taking strings, producing process
  - Future: functions producing in-process results

**Arguments are implicitly quoted:**
- Every argument is treated as a string (bash-style)
- No evaluation as patchwork expressions unless explicitly quoted

**Standard library provides portability:**
- Common commands (`mkdir`, `echo`, `git`, `cat`, etc.) available via stdlib
- Standard prelude auto-imports most common commands
- Users can define their own command abstractions

**Examples:**
```patchwork
mkdir -p work_dir                    # Command with flag and argument
echo "Created session: ${session_id}" # Command with interpolated string
git checkout "${clean_branch}"       # Command with interpolated argument
var timestamp = $(date +%Y%m%d-%H%M%S) # Command substitution
cat({...}) > "${work_dir}/state.json"  # Function-style with redirection
```

### Disambiguation Strategy: Whitespace-Sensitive Parsing

**The core challenge:** How to distinguish:
- `f(x, y, z)` - function call
- `f x y z` - bare command invocation
- `f` - variable reference
- `f (x)` - bare command with parenthesized expression argument

**Solution:** Use whitespace sensitivity
- `f(...)` - function call (no space before `(`)
- `f ...` - bare command (space before first argument)
- This mirrors function call conventions in languages like Swift and Python linters

**Lexical mode switching:**
- When lexer sees `IDENTIFIER WHITESPACE (not-lparen)`, enter "shell argument mode"
- In shell mode, tokenization changes (see below)

### Command Substitution: `$(...)` vs `${...}`

Two interpolation syntaxes:
- `${expr}` - interpolate arbitrary patchwork expression (M6 feature)
- `$(command args)` - execute bare command, capture stdout (new in M10)

**Relationship:**
- `$(...)` requires bare command inside
- `${...}` allows any expression, including bare commands
- This makes `$(...)` somewhat redundant but familiar from bash
- Keep both for ergonomics

### Context Sensitivity

Bare commands work in multiple contexts:

**Statement position:**
```patchwork
mkdir -p work_dir
echo "Starting analysis"
```

**Expression position:**
```patchwork
var branch = $(git rev_parse --abbrev_ref HEAD)
if ! git diff_index --quiet HEAD -- { ... }
log(session_id, "ANALYST", "message")  # Arguments can be bare commands
```

**Redirections work in both contexts:**
```patchwork
# Statement
echo "done" > "${work_dir}/status"

# Expression
var diff = cat(file) | grep "pattern"
```

### Shell Operator Handling

When parsing bare command arguments, operators have context-dependent meanings:

**Shell operators** (special in command context):
- `|` - pipe (NOT array filter)
- `>` - output redirection (NOT comparison)
- `<` - input redirection
- `>>` - append redirection
- `2>` - stderr redirection
- `2>&1` - stderr to stdout
- `&&`, `||` - command chaining (same as logical operators)
- `&` - background process

**Regular operators** (become part of argument strings):
- `+` - not addition, part of argument like `+%Y%m%d`
- `-` - not subtraction, flag prefix like `-p` or `--abbrev_ref`
- `..` - not range, part of argument like `HEAD..origin/main`

**Type system disambiguates:**
- `int > int` → comparison operator (returns bool)
- `Proc > string` → redirection operator (returns Proc)
- `[T] | (T -> U)` → array filter pipe
- `Proc | Proc` → process pipe

### Argument Parsing Rules

**Implicit quoting:**
Every argument is an implicitly quoted string:
```patchwork
mkdir -p work_dir
# Parses as: mkdir(["-p", "work_dir"])

date +%Y%m%d-%H%M%S
# Parses as: date(["+%Y%m%d-%H%M%S"])
```

**Quote handling:**
- Double quotes `"..."` - allow spaces and interpolation
- Single quotes `'...'` - literal strings, no interpolation (future feature)
- Unquoted - identifier-like tokens, no spaces

**Variable references require quotes:**
```patchwork
mkdir -p some_dir          # Literal string "some_dir"
mkdir -p $some_dir         # ERROR: not in string context
mkdir -p "$some_dir"       # Interpolates variable value
mkdir -p "${some_dir}"     # Also interpolates variable value
```

### Redirection and Pipes

**Redirection forms:**
- `expr > file` - stdout to file
- `expr >> file` - append stdout to file
- `expr 2> file` - stderr to file
- `expr 2>&1` - stderr to stdout
- `expr | command` - pipe stdout to command

**Type-based disambiguation:**
The same operators (`>`, `|`) serve different purposes based on context. The type system resolves ambiguity:
```patchwork
# Comparison
if x > 5 { ... }            # Type: int > int → bool

# Redirection
echo "done" > file          # Type: Proc > string → Proc

# Array filter
items | filter_fn           # Type: [T] | (T -> U) → [U]

# Process pipe
cat file | grep "pattern"   # Type: Proc | Proc → Proc
```

### Special Case: Overloaded Functions

Some stdlib functions work in multiple ways:

**`cat` function:**
- `cat(file)` - read file (function call syntax)
- `cat file` - read file (bare command syntax)
- `cat({x, y})` - serialize object to JSON (function call syntax)

Example combining function call with redirection:
```patchwork
cat({
    session_id,
    timestamp,
    original: current_branch,
    work_dir
}) > "${work_dir}/state.json"
```

This uses function call syntax `cat(...)` (no space before paren), then redirects the result.

### Lexer Mode Switching

**Shell argument mode:**

When lexer detects bare command context (identifier + whitespace + non-paren):
1. **Enter shell mode**
2. **Tokenize differently:**
   - Shell operators (`|`, `>`, `<`, `&&`, `||`, `&`, etc.) remain special tokens
   - Other operators (`+`, `-`, `*`, etc.) become part of `COMMAND_TOKEN`
   - Whitespace separates arguments
   - Quotes start string literals (with interpolation working normally)
3. **Exit shell mode** on:
   - Shell operator (but continue for next command in pipe/chain)
   - Statement terminator (`;`, newline, `}`)
   - Close paren `)` (for command substitution)

**Example tokenization:**

```patchwork
date +%Y%m%d-%H%M%S
```
Tokens: `IDENTIFIER("date")`, `COMMAND_TOKEN("+%Y%m%d-%H%M%S")`

```patchwork
git diff "${base}..HEAD" > file.diff
```
Tokens:
- `IDENTIFIER("git")`
- `COMMAND_TOKEN("diff")`
- `STRING_START`, `STRING_TEXT("")`, `DOLLAR`, `LBRACE`, `IDENTIFIER("base")`, `RBRACE`, `STRING_TEXT("..HEAD")`, `STRING_END`
- `REDIRECT_OUT(">")`
- `STRING_START`, `STRING_TEXT("file.diff")`, `STRING_END`

### AST Representation

**New expression variants:**
```rust
pub enum Expr<'input> {
    // ... existing variants ...

    /// Bare command invocation: mkdir -p work_dir
    BareCommand {
        name: &'input str,
        args: Vec<CommandArg<'input>>,
    },

    /// Command substitution: $(command args)
    CommandSubst {
        name: &'input str,
        args: Vec<CommandArg<'input>>,
    },
}
```

**New AST types:**
```rust
/// Command argument (implicitly quoted string)
pub enum CommandArg<'input> {
    /// Unquoted identifier-like token: work_dir, -p, +%Y%m%d
    Literal(&'input str),

    /// Quoted string with optional interpolation: "${work_dir}/state.json"
    String(StringLiteral<'input>),
}

/// Redirection operators
pub enum RedirectOp {
    Out,          // >
    Append,       // >>
    ErrOut,       // 2>
    ErrToOut,     // 2>&1
}
```

**Note on pipes:**
- Pipe `|` reuses existing `BinOp::Pipe`
- Type system disambiguates array filter vs process pipe
- May split into separate AST nodes if this proves problematic

### Design Decisions

**Q1: Should we implement `eval` in M10?**
- **Decision:** No, defer. The `eval` usage in scribe.pw needs a better portable syntax.
- Focus on core bare command functionality first.

**Q2: How to handle `cat({...}) > file` ambiguity?**
- **Decision:** Type system disambiguates.
- `cat(...)` is function call (no space), returns serializable content
- `>` operator typed to work with redirection when LHS is `Proc` or serializable type

**Q3: Pipe `|` - reuse BinOp or separate?**
- **Decision:** Keep existing `BinOp::Pipe`, type system distinguishes uses
- Can split into `BinOp::Pipe` and `Expr::ProcessPipe` later if needed

**Q4: Lexer mode switching vs parser lookahead?**
- **Decision:** Lexer mode switching
- Cleaner separation: lexer handles shell tokenization, parser handles structure
- Can fall back to parser-level disambiguation if lexer approach proves complex

**Q5: Single quotes for literal strings?**
- **Decision:** Defer for now, not in historian examples
- Can add later if needed

## Summary

This design establishes:
1. ✅ **Integration approach**: External lexer adapter pattern with lalrpop
2. ✅ **Token strategy**: Lifetime-carrying tokens for efficiency
3. ✅ **AST structure**: Hierarchical nodes representing program semantics
4. ✅ **Bare commands**: Shell-like syntax with portable semantics and whitespace-sensitive disambiguation
5. ✅ **Implementation plan**: Progressive milestones from structure to full features
6. ✅ **Testing strategy**: Per-milestone validation against historian examples

The key innovations of patchwork:
- Seamlessly mixing prompts and code via `think`/`ask`/`do` - captured in `PromptBlock` and related AST nodes
- Shell-like command syntax with OS-agnostic portability - captured in `BareCommand` and command argument handling
- Type-based operator disambiguation - enabling `>` and `|` to serve multiple purposes

Next step: Implement Milestone 10 with bare command expressions to complete parser coverage of historian examples.
