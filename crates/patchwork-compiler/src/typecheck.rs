/// Type checking and semantic analysis
///
/// Provides compile-time type checking and semantic validation:
/// - Symbol table construction
/// - Scope analysis and variable binding validation
/// - Basic type inference
/// - Type annotation validation

use patchwork_parser::ast::*;
use crate::error::{CompileError, Result};
use std::collections::HashMap;

/// A type in the type system
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Unknown type (not yet inferred)
    Unknown,
    /// String type
    String,
    /// Number type (int or float)
    Number,
    /// Boolean type
    Bool,
    /// Array type with element type
    Array(Box<Type>),
    /// Object type with fields
    Object(HashMap<String, Type>),
    /// Union type (multiple alternatives)
    Union(Vec<Type>),
    /// Function type (params -> return)
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    /// Named type (user-defined or built-in)
    Named(String),
    /// Void type (for statements and functions with no return)
    Void,
}

impl Type {
    /// Convert a TypeExpr from the AST to our internal Type representation
    pub fn from_type_expr(expr: &TypeExpr) -> Type {
        match expr {
            TypeExpr::Name(name) => {
                match *name {
                    "string" => Type::String,
                    "number" | "int" | "float" => Type::Number,
                    "bool" | "boolean" => Type::Bool,
                    "void" => Type::Void,
                    _ => Type::Named(name.to_string()),
                }
            }
            TypeExpr::Array(elem_type) => {
                Type::Array(Box::new(Type::from_type_expr(elem_type)))
            }
            TypeExpr::Object(fields) => {
                let mut field_map = HashMap::new();
                for field in fields {
                    field_map.insert(
                        field.key.to_string(),
                        Type::from_type_expr(&field.type_expr)
                    );
                }
                Type::Object(field_map)
            }
            TypeExpr::Union(variants) => {
                Type::Union(variants.iter().map(Type::from_type_expr).collect())
            }
            TypeExpr::Literal(_) => {
                // String literal types are treated as strings for now
                Type::String
            }
        }
    }
}

/// Symbol information stored in the symbol table
#[derive(Debug, Clone)]
pub struct Symbol {
    /// Name of the symbol
    pub name: String,
    /// Type of the symbol
    pub ty: Type,
    /// Scope depth where this symbol was declared
    pub scope_depth: usize,
    /// Kind of symbol
    pub kind: SymbolKind,
}

/// Kind of symbol
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    /// Variable
    Variable,
    /// Function parameter
    Parameter,
    /// Function
    Function,
    /// Worker
    Worker,
    /// Trait
    Trait,
    /// Type alias
    TypeAlias,
}

/// Scope containing symbols
#[derive(Debug, Clone, Default)]
pub struct Scope {
    /// Symbols in this scope (name -> symbol)
    symbols: HashMap<String, Symbol>,
    /// Parent scope (None for global scope)
    parent: Option<Box<Scope>>,
    /// Depth of this scope (0 for global)
    depth: usize,
}

impl Scope {
    /// Create a new global scope
    pub fn new_global() -> Self {
        let mut scope = Self {
            symbols: HashMap::new(),
            parent: None,
            depth: 0,
        };

        // Add built-in symbols
        scope.add_builtin("print", Type::Function {
            params: vec![Type::String],
            ret: Box::new(Type::Void),
        });

        scope.add_builtin("cat", Type::Function {
            params: vec![Type::Unknown], // Accepts any type
            ret: Box::new(Type::String),
        });

        scope
    }

    /// Create a new child scope
    pub fn new_child(parent: Scope) -> Self {
        let depth = parent.depth + 1;
        Self {
            symbols: HashMap::new(),
            parent: Some(Box::new(parent)),
            depth,
        }
    }

    /// Add a built-in symbol
    fn add_builtin(&mut self, name: &str, ty: Type) {
        self.symbols.insert(name.to_string(), Symbol {
            name: name.to_string(),
            ty,
            scope_depth: 0,
            kind: SymbolKind::Function,
        });
    }

    /// Add a symbol to this scope
    pub fn add_symbol(&mut self, name: String, ty: Type, kind: SymbolKind) -> Result<()> {
        if self.symbols.contains_key(&name) {
            return Err(CompileError::TypeError {
                message: format!("Duplicate declaration of '{}'", name),
            });
        }

        self.symbols.insert(name.clone(), Symbol {
            name,
            ty,
            scope_depth: self.depth,
            kind,
        });

        Ok(())
    }

    /// Look up a symbol in this scope or parent scopes
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        if let Some(symbol) = self.symbols.get(name) {
            Some(symbol)
        } else if let Some(parent) = &self.parent {
            parent.lookup(name)
        } else {
            None
        }
    }

    /// Get the depth of this scope
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Take the parent scope, leaving this scope without a parent
    pub fn take_parent(self) -> Option<Scope> {
        self.parent.map(|b| *b)
    }
}

/// Type checker that performs semantic analysis
pub struct TypeChecker {
    /// Current scope
    scope: Scope,
}

impl TypeChecker {
    /// Create a new type checker
    pub fn new() -> Self {
        Self {
            scope: Scope::new_global(),
        }
    }

    /// Check a program and return any errors found
    pub fn check_program(&mut self, program: &Program) -> Result<()> {
        // First pass: collect all top-level declarations
        for item in &program.items {
            match item {
                Item::Worker(worker) => {
                    self.declare_worker(worker)?;
                }
                Item::Function(func) => {
                    self.declare_function(func)?;
                }
                Item::Trait(trait_decl) => {
                    self.declare_trait(trait_decl)?;
                }
                Item::Type(type_decl) => {
                    self.declare_type(type_decl)?;
                }
                Item::Import(import) => {
                    self.declare_import(import)?;
                }
                Item::Skill(_) => {
                    // Skills don't declare new symbols
                }
            }
        }

        // Second pass: type check all declarations
        for item in &program.items {
            match item {
                Item::Worker(worker) => {
                    self.check_worker(worker)?;
                }
                Item::Function(func) => {
                    self.check_function(func)?;
                }
                Item::Trait(trait_decl) => {
                    self.check_trait(trait_decl)?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Declare imports in the symbol table
    fn declare_import(&mut self, import: &ImportDecl) -> Result<()> {
        match &import.path {
            ImportPath::Simple(segments) => {
                // Handle std library imports
                if segments.first() == Some(&"std") && segments.len() == 2 {
                    let name = segments[1];
                    // Add std library symbols (currently log and cat)
                    match name {
                        "log" => {
                            // log is a variadic function: log(...args)
                            self.scope.add_symbol(
                                "log".to_string(),
                                Type::Function {
                                    params: vec![], // Variadic, so params are unknown
                                    ret: Box::new(Type::Void),
                                },
                                SymbolKind::Function
                            )?;
                        }
                        "cat" => {
                            // cat is a function: cat(value: any) -> string
                            self.scope.add_symbol(
                                "cat".to_string(),
                                Type::Function {
                                    params: vec![Type::Unknown], // Accepts any type
                                    ret: Box::new(Type::String),
                                },
                                SymbolKind::Function
                            )?;
                        }
                        _ => {
                            // Unknown std library module - skip for now
                        }
                    }
                }
                // Other imports (relative) are handled during module resolution
            }
            ImportPath::RelativeMulti(_) => {
                // Multi-imports are handled during module resolution
            }
        }
        Ok(())
    }

    /// Declare a worker in the symbol table
    fn declare_worker(&mut self, worker: &WorkerDecl) -> Result<()> {
        let param_types = worker.params.iter()
            .map(|p| p.type_ann.as_ref()
                .map(Type::from_type_expr)
                .unwrap_or(Type::Unknown))
            .collect();

        let ty = Type::Function {
            params: param_types,
            ret: Box::new(Type::Void), // Workers don't return values
        };

        self.scope.add_symbol(
            worker.name.to_string(),
            ty,
            SymbolKind::Worker
        )
    }

    /// Declare a function in the symbol table
    fn declare_function(&mut self, func: &FunctionDecl) -> Result<()> {
        let param_types = func.params.iter()
            .map(|p| p.type_ann.as_ref()
                .map(Type::from_type_expr)
                .unwrap_or(Type::Unknown))
            .collect();

        let ty = Type::Function {
            params: param_types,
            ret: Box::new(Type::Unknown), // Infer return type later
        };

        self.scope.add_symbol(
            func.name.to_string(),
            ty,
            SymbolKind::Function
        )
    }

    /// Declare a trait in the symbol table
    fn declare_trait(&mut self, trait_decl: &TraitDecl) -> Result<()> {
        // For now, just record that the trait exists
        self.scope.add_symbol(
            trait_decl.name.to_string(),
            Type::Named(trait_decl.name.to_string()),
            SymbolKind::Trait
        )
    }

    /// Declare a type alias in the symbol table
    fn declare_type(&mut self, type_decl: &TypeDeclItem) -> Result<()> {
        let ty = Type::from_type_expr(&type_decl.type_expr);
        self.scope.add_symbol(
            type_decl.name.to_string(),
            ty,
            SymbolKind::TypeAlias
        )
    }

    /// Type check a worker declaration
    fn check_worker(&mut self, worker: &WorkerDecl) -> Result<()> {
        // Enter worker scope
        let parent_scope = std::mem::take(&mut self.scope);
        self.scope = Scope::new_child(parent_scope);

        // Add 'self' object with session context
        let self_type = Type::Object({
            let mut fields = HashMap::new();
            fields.insert("session".to_string(), Type::Named("SessionContext".to_string()));
            fields
        });
        self.scope.add_symbol(
            "self".to_string(),
            self_type,
            SymbolKind::Variable
        )?;

        // Add parameters to scope
        for param in &worker.params {
            let ty = param.type_ann.as_ref()
                .map(Type::from_type_expr)
                .unwrap_or(Type::Unknown);

            self.scope.add_symbol(
                param.name.to_string(),
                ty,
                SymbolKind::Parameter
            )?;
        }

        // Check the worker body
        self.check_block(&worker.body)?;

        // Exit worker scope
        let child_scope = std::mem::take(&mut self.scope);
        if let Some(parent) = child_scope.take_parent() {
            self.scope = parent;
        }

        Ok(())
    }

    /// Type check a function declaration
    fn check_function(&mut self, func: &FunctionDecl) -> Result<()> {
        // Enter function scope
        let parent_scope = std::mem::take(&mut self.scope);
        self.scope = Scope::new_child(parent_scope);

        // Add 'self' object with session context (available in trait methods)
        let self_type = Type::Object({
            let mut fields = HashMap::new();
            fields.insert("session".to_string(), Type::Named("SessionContext".to_string()));
            fields.insert("delegate".to_string(), Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Named("Session".to_string())),
            });
            fields
        });
        self.scope.add_symbol(
            "self".to_string(),
            self_type,
            SymbolKind::Variable
        )?;

        // Add parameters to scope
        for param in &func.params {
            let ty = param.type_ann.as_ref()
                .map(Type::from_type_expr)
                .unwrap_or(Type::Unknown);

            self.scope.add_symbol(
                param.name.to_string(),
                ty,
                SymbolKind::Parameter
            )?;
        }

        // Check the function body
        self.check_block(&func.body)?;

        // Exit function scope
        let child_scope = std::mem::take(&mut self.scope);
        if let Some(parent) = child_scope.take_parent() {
            self.scope = parent;
        }

        Ok(())
    }

    /// Type check a trait declaration
    fn check_trait(&mut self, trait_decl: &TraitDecl) -> Result<()> {
        // Check each method in the trait
        for method in &trait_decl.methods {
            self.check_function(method)?;
        }

        Ok(())
    }

    /// Type check a block of statements
    fn check_block(&mut self, block: &Block) -> Result<()> {
        for stmt in &block.statements {
            self.check_statement(stmt)?;
        }
        Ok(())
    }

    /// Type check a statement
    fn check_statement(&mut self, stmt: &Statement) -> Result<()> {
        match stmt {
            Statement::VarDecl { pattern, init } => {
                // Check the initializer expression (if present)
                let init_type = if let Some(expr) = init {
                    self.check_expr(expr)?
                } else {
                    Type::Unknown
                };

                // Declare variables from the pattern
                self.declare_pattern(pattern, init_type)?;
            }

            Statement::Expr(expr) => {
                self.check_expr(expr)?;
            }

            Statement::If { condition, then_block, else_block } => {
                self.check_expr(condition)?;
                self.check_block(then_block)?;
                if let Some(else_blk) = else_block {
                    self.check_block(else_blk)?;
                }
            }

            Statement::ForIn { var, iter, body } => {
                // Check the iterator expression
                self.check_expr(iter)?;

                // Enter loop scope
                let parent_scope = std::mem::take(&mut self.scope);
                self.scope = Scope::new_child(parent_scope);

                // Declare loop variable
                self.scope.add_symbol(
                    var.to_string(),
                    Type::Unknown, // Infer from iterator type
                    SymbolKind::Variable
                )?;

                // Check loop body
                self.check_block(body)?;

                // Exit loop scope
                let child_scope = std::mem::take(&mut self.scope);
                if let Some(parent) = child_scope.take_parent() {
                    self.scope = parent;
                }
            }

            Statement::While { condition, body } => {
                self.check_expr(condition)?;
                self.check_block(body)?;
            }

            Statement::Return(expr) => {
                if let Some(e) = expr {
                    self.check_expr(e)?;
                }
            }

            Statement::Succeed | Statement::Break => {
                // No type checking needed
            }

            Statement::TypeDecl { name, type_expr } => {
                // Validate the type expression is well-formed
                Type::from_type_expr(type_expr);

                // Add to scope
                let ty = Type::from_type_expr(type_expr);
                self.scope.add_symbol(
                    name.to_string(),
                    ty,
                    SymbolKind::TypeAlias
                )?;
            }
        }

        Ok(())
    }

    /// Declare variables from a pattern
    fn declare_pattern(&mut self, pattern: &Pattern, init_type: Type) -> Result<()> {
        match pattern {
            Pattern::Identifier { name, type_ann } => {
                let ty = if let Some(ann) = type_ann {
                    Type::from_type_expr(ann)
                } else {
                    init_type
                };

                self.scope.add_symbol(
                    name.to_string(),
                    ty,
                    SymbolKind::Variable
                )?;
            }

            Pattern::Ignore => {
                // Ignore pattern doesn't declare any variables
            }

            Pattern::Object(fields) => {
                // For object destructuring, extract field types from init_type if it's an object
                for field in fields {
                    let field_type = if let Type::Object(ref obj_fields) = init_type {
                        obj_fields.get(field.key).cloned().unwrap_or(Type::Unknown)
                    } else {
                        Type::Unknown
                    };

                    self.declare_pattern(&field.pattern, field_type)?;
                }
            }

            Pattern::Array(patterns) => {
                // For array destructuring, use element type from init_type if it's an array
                let elem_type = if let Type::Array(ref elem) = init_type {
                    (**elem).clone()
                } else {
                    Type::Unknown
                };

                for pat in patterns {
                    self.declare_pattern(pat, elem_type.clone())?;
                }
            }
        }

        Ok(())
    }

    /// Type check an expression and return its type
    fn check_expr(&mut self, expr: &Expr) -> Result<Type> {
        match expr {
            Expr::Identifier(name) => {
                // Look up the identifier in the symbol table
                if let Some(symbol) = self.scope.lookup(name) {
                    Ok(symbol.ty.clone())
                } else {
                    Err(CompileError::TypeError {
                        message: format!("Undefined variable '{}'", name),
                    })
                }
            }

            Expr::Number(_) => Ok(Type::Number),
            Expr::String(_) => Ok(Type::String),
            Expr::True | Expr::False => Ok(Type::Bool),

            Expr::Array(elements) => {
                // Infer element type from first element
                let elem_type = if let Some(first) = elements.first() {
                    self.check_expr(first)?
                } else {
                    Type::Unknown
                };

                // Check all elements (TODO: unify types)
                for elem in elements.iter().skip(1) {
                    self.check_expr(elem)?;
                }

                Ok(Type::Array(Box::new(elem_type)))
            }

            Expr::Object(fields) => {
                let mut field_types = HashMap::new();

                for field in fields {
                    let field_type = if let Some(ref value) = field.value {
                        self.check_expr(value)?
                    } else {
                        // Shorthand: {x} means {x: x}
                        self.check_expr(&Expr::Identifier(field.key))?
                    };

                    field_types.insert(field.key.to_string(), field_type);
                }

                Ok(Type::Object(field_types))
            }

            Expr::Binary { op, left, right } => {
                let _left_type = self.check_expr(left)?;
                let right_type = self.check_expr(right)?;

                // Simple type checking for binary operations
                // TODO: Add type compatibility checking between left and right
                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => Ok(Type::Number),
                    BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Gt => Ok(Type::Bool),
                    BinOp::And | BinOp::Or => Ok(Type::Bool),
                    BinOp::Assign => Ok(right_type),
                    _ => Ok(Type::Unknown),
                }
            }

            Expr::Unary { op, operand } => {
                let _operand_type = self.check_expr(operand)?;

                // TODO: Check operand type is compatible with operator
                match op {
                    UnOp::Not => Ok(Type::Bool),
                    UnOp::Neg => Ok(Type::Number),
                    UnOp::Throw => Ok(Type::Void),
                }
            }

            Expr::Call { callee, args } => {
                let callee_type = self.check_expr(callee)?;

                // Check argument types
                for arg in args {
                    self.check_expr(arg)?;
                }

                // Extract return type from function type
                if let Type::Function { ret, .. } = callee_type {
                    Ok(*ret)
                } else {
                    Ok(Type::Unknown)
                }
            }

            Expr::Member { object, field } => {
                let obj_type = self.check_expr(object)?;

                // If object type is known and is an object, look up field type
                if let Type::Object(fields) = obj_type {
                    Ok(fields.get(*field).cloned().unwrap_or(Type::Unknown))
                } else {
                    Ok(Type::Unknown)
                }
            }

            Expr::Index { object, index } => {
                let obj_type = self.check_expr(object)?;
                self.check_expr(index)?;

                // If object is an array, return element type
                if let Type::Array(elem_type) = obj_type {
                    Ok(*elem_type)
                } else {
                    Ok(Type::Unknown)
                }
            }

            Expr::Paren(inner) => self.check_expr(inner),

            Expr::Await(inner) => self.check_expr(inner),

            Expr::Think(_) | Expr::Ask(_) => {
                // Prompt blocks return unknown type (determined by LLM)
                Ok(Type::Unknown)
            }

            Expr::Do(block) => {
                self.check_block(block)?;
                Ok(Type::Unknown)
            }

            Expr::BareCommand { .. } | Expr::CommandSubst(_) => {
                // Shell commands return strings
                Ok(Type::String)
            }

            Expr::ShellPipe { left, right } |
            Expr::ShellAnd { left, right } |
            Expr::ShellOr { left, right } => {
                self.check_expr(left)?;
                self.check_expr(right)?;
                Ok(Type::String)
            }

            Expr::ShellRedirect { command, target, .. } => {
                self.check_expr(command)?;
                self.check_expr(target)?;
                Ok(Type::String)
            }

            Expr::PostIncrement(inner) | Expr::PostDecrement(inner) => {
                self.check_expr(inner)?;
                Ok(Type::Number)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undefined_variable() {
        let program = Program {
            items: vec![
                Item::Worker(WorkerDecl {
                    name: "test",
                    params: vec![],
                    body: Block {
                        statements: vec![
                            Statement::Expr(Expr::Identifier("undefined_var"))
                        ]
                    },
                    is_exported: false,
                    is_default: false,
                })
            ]
        };

        let mut checker = TypeChecker::new();
        let result = checker.check_program(&program);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Undefined variable"));
    }

    #[test]
    fn test_duplicate_declaration() {
        let program = Program {
            items: vec![
                Item::Worker(WorkerDecl {
                    name: "test",
                    params: vec![],
                    body: Block {
                        statements: vec![
                            Statement::VarDecl {
                                pattern: Pattern::Identifier {
                                    name: "x",
                                    type_ann: None,
                                },
                                init: Some(Expr::Number("1")),
                            },
                            Statement::VarDecl {
                                pattern: Pattern::Identifier {
                                    name: "x",
                                    type_ann: None,
                                },
                                init: Some(Expr::Number("2")),
                            },
                        ]
                    },
                    is_exported: false,
                    is_default: false,
                })
            ]
        };

        let mut checker = TypeChecker::new();
        let result = checker.check_program(&program);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Duplicate declaration"));
    }
}
