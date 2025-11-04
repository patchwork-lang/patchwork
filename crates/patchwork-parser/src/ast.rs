/// Abstract Syntax Tree types for patchwork
///
/// These types represent the parsed structure of patchwork programs.
/// All types carry a lifetime 'input for zero-copy string slices.

use std::marker::PhantomData;

/// A complete patchwork program
#[derive(Debug, Clone, PartialEq)]
pub struct Program<'input> {
    pub items: Vec<Item<'input>>,
}

/// Top-level item (import, skill, task, or function declaration)
#[derive(Debug, Clone, PartialEq)]
pub enum Item<'input> {
    Import(ImportDecl<'input>),
    Skill(SkillDecl<'input>),
    Task(TaskDecl<'input>),
    Function(FunctionDecl<'input>),
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

/// Statement in a block
#[derive(Debug, Clone, PartialEq)]
pub enum Statement<'input> {
    /// Variable declaration: `var x` or `var x: type = expr`
    VarDecl {
        name: &'input str,
        type_ann: Option<TypeExpr<'input>>,
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
}

/// Type expression (Milestone 3: minimal placeholder, full implementation in Milestone 8)
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr<'input> {
    /// Simple type name: `string`, `int`, etc.
    Name(&'input str),
}

/// Expression (Milestone 3: minimal set for statement support, expanded in Milestone 4)
#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'input> {
    /// Identifier reference: `foo`
    Identifier(&'input str),
    /// Number literal: `42`, `3.14`
    Number(&'input str),
    /// Boolean literal: `true`
    True,
    /// Boolean literal: `false`
    False,
    /// Placeholder for unparsed expressions (temporary for incremental implementation)
    Placeholder(PhantomData<&'input ()>),
}
