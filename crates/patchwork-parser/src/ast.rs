/// Abstract Syntax Tree types for patchwork
///
/// These types represent the parsed structure of patchwork programs.
/// All types carry a lifetime 'input for zero-copy string slices.

/// A complete patchwork program
#[derive(Debug, Clone, PartialEq)]
pub struct Program<'input> {
    pub items: Vec<Item<'input>>,
}

/// Top-level item (import, skill, task, function, or type declaration)
#[derive(Debug, Clone, PartialEq)]
pub enum Item<'input> {
    Import(ImportDecl<'input>),
    Skill(SkillDecl<'input>),
    Task(TaskDecl<'input>),
    Function(FunctionDecl<'input>),
    Type(TypeDeclItem<'input>),
}

/// Import declaration: `import std.log` or `import ./{analyst, narrator}`
#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl<'input> {
    pub path: ImportPath<'input>,
}

/// Import path - either simple dotted path or relative multi-import
#[derive(Debug, Clone, PartialEq)]
pub enum ImportPath<'input> {
    /// Simple path: `std.log` or `./foo`
    Simple(Vec<&'input str>),
    /// Relative multi-import: `./{analyst, narrator, scribe}`
    RelativeMulti(Vec<&'input str>),
}

/// Skill declaration: `skill name(params) { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct SkillDecl<'input> {
    pub name: &'input str,
    pub params: Vec<Param<'input>>,
    pub body: Block<'input>,
}

/// Task declaration: `task name(params) { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct TaskDecl<'input> {
    pub name: &'input str,
    pub params: Vec<Param<'input>>,
    pub body: Block<'input>,
}

/// Function declaration: `fun name(params) { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl<'input> {
    pub name: &'input str,
    pub params: Vec<Param<'input>>,
    pub body: Block<'input>,
}

/// Type declaration: `type name = TypeExpr`
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDeclItem<'input> {
    pub name: &'input str,
    pub type_expr: TypeExpr<'input>,
}

/// Function/task/skill parameter
#[derive(Debug, Clone, PartialEq)]
pub struct Param<'input> {
    pub name: &'input str,
    // Type annotations will be added in Milestone 8
}

/// Block of statements: `{ stmt1; stmt2; ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct Block<'input> {
    pub statements: Vec<Statement<'input>>,
}

/// Pattern for destructuring in variable declarations (Milestone 7)
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern<'input> {
    /// Simple identifier pattern: `var x = ...` or `var x: type = ...`
    Identifier {
        name: &'input str,
        type_ann: Option<TypeExpr<'input>>,
    },
    /// Object destructuring pattern: `var {x, y} = ...`
    Object(Vec<ObjectPatternField<'input>>),
}

/// Field in an object destructuring pattern
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectPatternField<'input> {
    /// Key name in the object being destructured
    pub key: &'input str,
    /// Optional nested pattern (for now just identifier, could expand later)
    pub pattern: Pattern<'input>,
    /// Optional type annotation for this field
    pub type_ann: Option<TypeExpr<'input>>,
}

/// Statement in a block
#[derive(Debug, Clone, PartialEq)]
pub enum Statement<'input> {
    /// Variable declaration: `var x = expr` or `var {x, y} = expr`
    VarDecl {
        pattern: Pattern<'input>,
        init: Option<Expr<'input>>,
    },
    /// Expression statement (expression used as statement)
    Expr(Expr<'input>),
    /// If statement: `if expr { ... } else { ... }`
    If {
        condition: Expr<'input>,
        then_block: Block<'input>,
        else_block: Option<Block<'input>>,
    },
    /// For loop: `for var x in expr { ... }`
    For {
        var: &'input str,
        iter: Expr<'input>,
        body: Block<'input>,
    },
    /// While loop: `while (expr) { ... }`
    While {
        condition: Expr<'input>,
        body: Block<'input>,
    },
    /// Return statement: `return` or `return expr`
    Return(Option<Expr<'input>>),
    /// Succeed statement (for tasks): `succeed`
    Succeed,
    /// Fail statement (for tasks): `fail`
    Fail,
    /// Break statement (for loops): `break`
    Break,
    /// Type declaration: `type Foo = { ... }`
    TypeDecl {
        name: &'input str,
        type_expr: TypeExpr<'input>,
    },
}

/// Type expression (Milestone 8: complete type system)
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr<'input> {
    /// Simple type name: `string`, `int`, etc.
    Name(&'input str),
    /// Object type: `{ x: string, y: int }`
    Object(Vec<TypeField<'input>>),
    /// Array type: `[string]`
    Array(Box<TypeExpr<'input>>),
    /// Union type: `"success" | "error"` or `string | int`
    Union(Vec<TypeExpr<'input>>),
    /// String literal type: `"success"`
    Literal(&'input str),
}

/// Field in an object type
#[derive(Debug, Clone, PartialEq)]
pub struct TypeField<'input> {
    pub key: &'input str,
    pub type_expr: TypeExpr<'input>,
    /// For future optional field syntax `key?: type`
    pub optional: bool,
}

/// Binary operator
#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    // Arithmetic
    Add,      // +
    Sub,      // -
    Mul,      // *
    Div,      // /
    // Comparison
    Eq,       // ==
    NotEq,    // !=
    Lt,       // <
    Gt,       // >
    // Logical
    And,      // &&
    Or,       // ||
    // Other
    Pipe,     // |
    Range,    // ...
    Assign,   // =
}

/// Unary operator
#[derive(Debug, Clone, PartialEq)]
pub enum UnOp {
    Not,      // !
    Neg,      // -
}

/// String literal (Milestone 6: with interpolation support)
#[derive(Debug, Clone, PartialEq)]
pub struct StringLiteral<'input> {
    /// Parts of the string - mixture of text and interpolated expressions
    pub parts: Vec<StringPart<'input>>,
}

/// Part of a string literal - either text or an interpolated expression
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart<'input> {
    /// Plain text: `"hello"` or text between interpolations
    Text(&'input str),
    /// Interpolated expression: `${expr}`, `$(cmd)`, or `$id`
    Interpolation(Box<Expr<'input>>),
}

/// Command argument - either a literal string or an interpolated string
#[derive(Debug, Clone, PartialEq)]
pub enum CommandArg<'input> {
    /// Literal argument: `mkdir -p work_dir` → "-p" and "work_dir"
    Literal(&'input str),
    /// Interpolated string argument: `mkdir "${dir}"` → String with interpolation
    String(StringLiteral<'input>),
}

/// Redirection operator for shell-style I/O redirection
#[derive(Debug, Clone, PartialEq)]
pub enum RedirectOp {
    /// Standard output redirection: `>`
    Out,
    /// Append output redirection: `>>`
    Append,
    /// Standard input redirection: `<`
    In,
    /// Stderr redirection: `2>`
    ErrOut,
    /// Stderr to stdout redirection: `2>&1`
    ErrToOut,
}

/// Expression (Milestone 3: minimal set for statement support, expanded in Milestones 4-7)
#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'input> {
    /// Identifier reference: `foo`
    Identifier(&'input str),
    /// Number literal: `42`, `3.14`
    Number(&'input str),
    /// String literal: `"hello"`
    String(StringLiteral<'input>),
    /// Boolean literal: `true`
    True,
    /// Boolean literal: `false`
    False,
    /// Array literal: `[1, 2, 3]`
    Array(Vec<Expr<'input>>),
    /// Object literal: `{x: 1, y: 2}` or `{x, y}` (shorthand)
    Object(Vec<ObjectField<'input>>),
    /// Binary operation: `a + b`, `x == y`
    Binary {
        op: BinOp,
        left: Box<Expr<'input>>,
        right: Box<Expr<'input>>,
    },
    /// Unary operation: `!x`, `-5`
    Unary {
        op: UnOp,
        operand: Box<Expr<'input>>,
    },
    /// Function call: `foo(a, b, c)`
    Call {
        callee: Box<Expr<'input>>,
        args: Vec<Expr<'input>>,
    },
    /// Member access: `obj.field`
    Member {
        object: Box<Expr<'input>>,
        field: &'input str,
    },
    /// Index access: `arr[i]`
    Index {
        object: Box<Expr<'input>>,
        index: Box<Expr<'input>>,
    },
    /// Postfix increment: `x++`
    PostIncrement(Box<Expr<'input>>),
    /// Postfix decrement: `x--`
    PostDecrement(Box<Expr<'input>>),
    /// Parenthesized expression: `(expr)`
    Paren(Box<Expr<'input>>),
    /// Await expression: `await task_call()`
    Await(Box<Expr<'input>>),
    /// Task parallel execution: `task (expr1, expr2, expr3)`
    /// Semantically like Promise.race() - invokes tasks in parallel
    Task(Vec<Expr<'input>>),
    /// Think expression: `think { ... }`
    Think(PromptBlock<'input>),
    /// Ask expression: `ask { ... }`
    Ask(PromptBlock<'input>),
    /// Do expression: `do { ... }`
    Do(Block<'input>),
    /// Bare command invocation: `mkdir -p work_dir`
    BareCommand {
        name: &'input str,
        args: Vec<CommandArg<'input>>,
    },
    /// Command substitution: `$(shell_expr)`
    /// Executes shell expression and returns stdout as string
    CommandSubst(Box<Expr<'input>>),
    /// Shell pipe: `cmd1 | cmd2`
    ShellPipe {
        left: Box<Expr<'input>>,
        right: Box<Expr<'input>>,
    },
    /// Shell logical and: `cmd1 && cmd2`
    ShellAnd {
        left: Box<Expr<'input>>,
        right: Box<Expr<'input>>,
    },
    /// Shell logical or: `cmd1 || cmd2`
    ShellOr {
        left: Box<Expr<'input>>,
        right: Box<Expr<'input>>,
    },
    /// Shell redirect: `cmd > file` or `cmd 2> file`
    ShellRedirect {
        command: Box<Expr<'input>>,
        op: RedirectOp,
        target: Box<Expr<'input>>,
    },
}

/// Object field in an object literal
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectField<'input> {
    pub key: &'input str,
    /// Value expression - None for shorthand syntax `{x}` meaning `{x: x}`
    pub value: Option<Expr<'input>>,
}

/// Prompt block content - mixture of text and embedded code
#[derive(Debug, Clone, PartialEq)]
pub struct PromptBlock<'input> {
    pub items: Vec<PromptItem<'input>>,
}

/// Item within a prompt block
#[derive(Debug, Clone, PartialEq)]
pub enum PromptItem<'input> {
    /// Raw prompt text
    Text(&'input str),
    /// Variable or expression interpolation: `$var` or `${expr}`
    Interpolation(Expr<'input>),
    /// Embedded code block: `do { ... }`
    Code(Block<'input>),
}
