/// AST dumping utilities for testing and debugging
///
/// Provides human-readable tree representations of AST nodes.

use crate::ast::*;
use std::fmt::Write as FmtWrite;

/// Dump a program AST as a pretty-printed tree
pub fn dump_program(program: &Program) -> String {
    let mut out = String::new();
    write_program(&mut out, program, 0).unwrap();
    out
}

fn write_program(out: &mut String, program: &Program, indent: usize) -> std::fmt::Result {
    writeln!(out, "{}Program:", "  ".repeat(indent))?;
    for item in &program.items {
        write_item(out, item, indent + 1)?;
    }
    Ok(())
}

fn write_item(out: &mut String, item: &Item, indent: usize) -> std::fmt::Result {
    let prefix = "  ".repeat(indent);
    match item {
        Item::Import(decl) => {
            writeln!(out, "{}Import:", prefix)?;
            write_import_path(out, &decl.path, indent + 1)?;
        }
        Item::Skill(decl) => {
            let export_prefix = if decl.is_exported { "export " } else { "" };
            writeln!(out, "{}{}Skill: {}", prefix, export_prefix, decl.name)?;
            write_params(out, &decl.params, indent + 1)?;
            write_block(out, &decl.body, indent + 1)?;
        }
        Item::Task(decl) => {
            let export_prefix = if decl.is_exported { "export " } else { "" };
            writeln!(out, "{}{}Task: {}", prefix, export_prefix, decl.name)?;
            write_params(out, &decl.params, indent + 1)?;
            write_block(out, &decl.body, indent + 1)?;
        }
        Item::Function(decl) => {
            let export_prefix = if decl.is_exported { "export " } else { "" };
            writeln!(out, "{}{}Function: {}", prefix, export_prefix, decl.name)?;
            write_params(out, &decl.params, indent + 1)?;
            write_block(out, &decl.body, indent + 1)?;
        }
        Item::Type(decl) => {
            writeln!(out, "{}Type: {} =", prefix, decl.name)?;
            write_type_expr(out, &decl.type_expr, indent + 1)?;
        }
    }
    Ok(())
}

fn write_import_path(out: &mut String, path: &ImportPath, indent: usize) -> std::fmt::Result {
    let prefix = "  ".repeat(indent);
    match path {
        ImportPath::Simple(parts) => {
            writeln!(out, "{}Simple: {}", prefix, parts.join("."))?;
        }
        ImportPath::RelativeMulti(names) => {
            writeln!(out, "{}RelativeMulti: ./{{{}}}", prefix, names.join(", "))?;
        }
    }
    Ok(())
}

fn write_params(out: &mut String, params: &[Param], indent: usize) -> std::fmt::Result {
    let prefix = "  ".repeat(indent);
    if params.is_empty() {
        writeln!(out, "{}Params: (none)", prefix)?;
    } else {
        writeln!(out, "{}Params:", prefix)?;
        for param in params {
            writeln!(out, "{}  - {}", prefix, param.name)?;
        }
    }
    Ok(())
}

fn write_block(out: &mut String, block: &Block, indent: usize) -> std::fmt::Result {
    let prefix = "  ".repeat(indent);
    writeln!(out, "{}Block:", prefix)?;
    if block.statements.is_empty() {
        writeln!(out, "{}  (empty)", prefix)?;
    } else {
        for stmt in &block.statements {
            write_statement(out, stmt, indent + 1)?;
        }
    }
    Ok(())
}

fn write_statement(out: &mut String, stmt: &Statement, indent: usize) -> std::fmt::Result {
    let prefix = "  ".repeat(indent);
    match stmt {
        Statement::VarDecl { pattern, init } => {
            writeln!(out, "{}VarDecl:", prefix)?;
            write_pattern(out, pattern, indent + 1)?;
            if let Some(expr) = init {
                writeln!(out, "{}  Init:", prefix)?;
                write_expr(out, expr, indent + 2)?;
            }
        }
        Statement::Expr(expr) => {
            writeln!(out, "{}ExprStmt:", prefix)?;
            write_expr(out, expr, indent + 1)?;
        }
        Statement::If { condition, then_block, else_block } => {
            writeln!(out, "{}If:", prefix)?;
            writeln!(out, "{}  Condition:", prefix)?;
            write_expr(out, condition, indent + 2)?;
            writeln!(out, "{}  Then:", prefix)?;
            write_block(out, then_block, indent + 2)?;
            if let Some(else_blk) = else_block {
                writeln!(out, "{}  Else:", prefix)?;
                write_block(out, else_blk, indent + 2)?;
            }
        }
        Statement::ForIn { var, iter, body } => {
            writeln!(out, "{}For: var {} in", prefix, var)?;
            write_expr(out, iter, indent + 1)?;
            write_block(out, body, indent + 1)?;
        }
        Statement::While { condition, body } => {
            writeln!(out, "{}While:", prefix)?;
            write_expr(out, condition, indent + 1)?;
            write_block(out, body, indent + 1)?;
        }
        Statement::Return(expr) => {
            if let Some(e) = expr {
                writeln!(out, "{}Return:", prefix)?;
                write_expr(out, e, indent + 1)?;
            } else {
                writeln!(out, "{}Return (void)", prefix)?;
            }
        }
        Statement::Succeed => {
            writeln!(out, "{}Succeed", prefix)?;
        }
        Statement::Fail => {
            writeln!(out, "{}Fail", prefix)?;
        }
        Statement::Break => {
            writeln!(out, "{}Break", prefix)?;
        }
        Statement::TypeDecl { name, type_expr } => {
            writeln!(out, "{}TypeDecl: {} =", prefix, name)?;
            write_type_expr(out, type_expr, indent + 1)?;
        }
    }
    Ok(())
}

fn write_pattern(out: &mut String, pattern: &Pattern, indent: usize) -> std::fmt::Result {
    let prefix = "  ".repeat(indent);
    match pattern {
        Pattern::Identifier { name, type_ann } => {
            if let Some(ty) = type_ann {
                writeln!(out, "{}Pattern: {} :", prefix, name)?;
                write_type_expr(out, ty, indent + 1)?;
            } else {
                writeln!(out, "{}Pattern: {}", prefix, name)?;
            }
        }
        Pattern::Object(fields) => {
            writeln!(out, "{}ObjectPattern:", prefix)?;
            for field in fields {
                writeln!(out, "{}  {}: ", prefix, field.key)?;
                write_pattern(out, &field.pattern, indent + 2)?;
                if let Some(ty) = &field.type_ann {
                    writeln!(out, "{}    Type:", prefix)?;
                    write_type_expr(out, ty, indent + 3)?;
                }
            }
        }
    }
    Ok(())
}

fn write_expr(out: &mut String, expr: &Expr, indent: usize) -> std::fmt::Result {
    let prefix = "  ".repeat(indent);
    match expr {
        Expr::Identifier(name) => {
            writeln!(out, "{}Identifier: {}", prefix, name)?;
        }
        Expr::Number(n) => {
            writeln!(out, "{}Number: {}", prefix, n)?;
        }
        Expr::String(s) => {
            writeln!(out, "{}String:", prefix)?;
            write_string_literal(out, s, indent + 1)?;
        }
        Expr::True => {
            writeln!(out, "{}True", prefix)?;
        }
        Expr::False => {
            writeln!(out, "{}False", prefix)?;
        }
        Expr::Array(items) => {
            writeln!(out, "{}Array:", prefix)?;
            for item in items {
                write_expr(out, item, indent + 1)?;
            }
        }
        Expr::Object(fields) => {
            writeln!(out, "{}Object:", prefix)?;
            for field in fields {
                if let Some(value) = &field.value {
                    writeln!(out, "{}  {}: ", prefix, field.key)?;
                    write_expr(out, value, indent + 2)?;
                } else {
                    writeln!(out, "{}  {} (shorthand)", prefix, field.key)?;
                }
            }
        }
        Expr::Binary { op, left, right } => {
            writeln!(out, "{}Binary: {:?}", prefix, op)?;
            writeln!(out, "{}  Left:", prefix)?;
            write_expr(out, left, indent + 2)?;
            writeln!(out, "{}  Right:", prefix)?;
            write_expr(out, right, indent + 2)?;
        }
        Expr::Unary { op, operand } => {
            writeln!(out, "{}Unary: {:?}", prefix, op)?;
            write_expr(out, operand, indent + 1)?;
        }
        Expr::Call { callee, args } => {
            writeln!(out, "{}Call:", prefix)?;
            writeln!(out, "{}  Callee:", prefix)?;
            write_expr(out, callee, indent + 2)?;
            if !args.is_empty() {
                writeln!(out, "{}  Args:", prefix)?;
                for arg in args {
                    write_expr(out, arg, indent + 2)?;
                }
            }
        }
        Expr::Member { object, field } => {
            writeln!(out, "{}Member: .{}", prefix, field)?;
            write_expr(out, object, indent + 1)?;
        }
        Expr::Index { object, index } => {
            writeln!(out, "{}Index:", prefix)?;
            writeln!(out, "{}  Object:", prefix)?;
            write_expr(out, object, indent + 2)?;
            writeln!(out, "{}  Index:", prefix)?;
            write_expr(out, index, indent + 2)?;
        }
        Expr::Think(prompt) => {
            writeln!(out, "{}Think:", prefix)?;
            write_prompt_block(out, prompt, indent + 1)?;
        }
        Expr::Ask(prompt) => {
            writeln!(out, "{}Ask:", prefix)?;
            write_prompt_block(out, prompt, indent + 1)?;
        }
        Expr::Await(e) => {
            writeln!(out, "{}Await:", prefix)?;
            write_expr(out, e, indent + 1)?;
        }
        Expr::Task(tasks) => {
            writeln!(out, "{}Task:", prefix)?;
            for task in tasks {
                write_expr(out, task, indent + 1)?;
            }
        }
        Expr::BareCommand { name, args } => {
            writeln!(out, "{}BareCommand: {}", prefix, name)?;
            if !args.is_empty() {
                writeln!(out, "{}  Args:", prefix)?;
                for arg in args {
                    write_command_arg(out, arg, indent + 2)?;
                }
            }
        }
        Expr::CommandSubst(e) => {
            writeln!(out, "{}CommandSubst:", prefix)?;
            write_expr(out, e, indent + 1)?;
        }
        Expr::ShellPipe { left, right } => {
            writeln!(out, "{}ShellPipe:", prefix)?;
            writeln!(out, "{}  Left:", prefix)?;
            write_expr(out, left, indent + 2)?;
            writeln!(out, "{}  Right:", prefix)?;
            write_expr(out, right, indent + 2)?;
        }
        Expr::ShellAnd { left, right } => {
            writeln!(out, "{}ShellAnd:", prefix)?;
            writeln!(out, "{}  Left:", prefix)?;
            write_expr(out, left, indent + 2)?;
            writeln!(out, "{}  Right:", prefix)?;
            write_expr(out, right, indent + 2)?;
        }
        Expr::ShellOr { left, right } => {
            writeln!(out, "{}ShellOr:", prefix)?;
            writeln!(out, "{}  Left:", prefix)?;
            write_expr(out, left, indent + 2)?;
            writeln!(out, "{}  Right:", prefix)?;
            write_expr(out, right, indent + 2)?;
        }
        Expr::ShellRedirect { command, op, target } => {
            writeln!(out, "{}ShellRedirect: {:?}", prefix, op)?;
            writeln!(out, "{}  Command:", prefix)?;
            write_expr(out, command, indent + 2)?;
            writeln!(out, "{}  Target:", prefix)?;
            write_expr(out, target, indent + 2)?;
        }
        Expr::PostIncrement(e) => {
            writeln!(out, "{}PostIncrement:", prefix)?;
            write_expr(out, e, indent + 1)?;
        }
        Expr::PostDecrement(e) => {
            writeln!(out, "{}PostDecrement:", prefix)?;
            write_expr(out, e, indent + 1)?;
        }
        Expr::Paren(e) => {
            writeln!(out, "{}Paren:", prefix)?;
            write_expr(out, e, indent + 1)?;
        }
        Expr::Do(block) => {
            writeln!(out, "{}Do:", prefix)?;
            write_block(out, block, indent + 1)?;
        }
    }
    Ok(())
}

fn write_string_literal(out: &mut String, s: &StringLiteral, indent: usize) -> std::fmt::Result {
    let prefix = "  ".repeat(indent);
    for part in &s.parts {
        match part {
            StringPart::Text(t) => {
                writeln!(out, "{}Text: {:?}", prefix, t)?;
            }
            StringPart::Interpolation(expr) => {
                writeln!(out, "{}Interpolation:", prefix)?;
                write_expr(out, expr, indent + 1)?;
            }
        }
    }
    Ok(())
}

fn write_prompt_block(out: &mut String, prompt: &PromptBlock, indent: usize) -> std::fmt::Result {
    let prefix = "  ".repeat(indent);
    for item in &prompt.items {
        match item {
            PromptItem::Text(t) => {
                writeln!(out, "{}Text: {:?}", prefix, t)?;
            }
            PromptItem::Interpolation(expr) => {
                writeln!(out, "{}Interpolation:", prefix)?;
                write_expr(out, expr, indent + 1)?;
            }
            PromptItem::Code(block) => {
                writeln!(out, "{}Code:", prefix)?;
                write_block(out, block, indent + 1)?;
            }
        }
    }
    Ok(())
}

fn write_command_arg(out: &mut String, arg: &CommandArg, indent: usize) -> std::fmt::Result {
    let prefix = "  ".repeat(indent);
    match arg {
        CommandArg::Literal(s) => {
            writeln!(out, "{}Literal: {}", prefix, s)?;
        }
        CommandArg::String(s) => {
            writeln!(out, "{}String:", prefix)?;
            write_string_literal(out, s, indent + 1)?;
        }
    }
    Ok(())
}

fn write_type_expr(out: &mut String, ty: &TypeExpr, indent: usize) -> std::fmt::Result {
    let prefix = "  ".repeat(indent);
    match ty {
        TypeExpr::Name(name) => {
            writeln!(out, "{}Type: {}", prefix, name)?;
        }
        TypeExpr::Object(fields) => {
            writeln!(out, "{}ObjectType:", prefix)?;
            for field in fields {
                writeln!(out, "{}  {}: ", prefix, field.key)?;
                write_type_expr(out, &field.type_expr, indent + 2)?;
            }
        }
        TypeExpr::Array(elem_ty) => {
            writeln!(out, "{}ArrayType:", prefix)?;
            write_type_expr(out, elem_ty, indent + 1)?;
        }
        TypeExpr::Union(types) => {
            writeln!(out, "{}Union:", prefix)?;
            for ty in types {
                write_type_expr(out, ty, indent + 1)?;
            }
        }
        TypeExpr::Literal(lit) => {
            writeln!(out, "{}Literal: {:?}", prefix, lit)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn test_dump_simple_function() {
        let input = "fun test(x) { return x }";
        let program = parse(input).unwrap();
        let dump = dump_program(&program);

        // Basic validation - should contain key structural elements
        assert!(dump.contains("Program:"));
        assert!(dump.contains("Function: test"));
        assert!(dump.contains("Params:"));
        assert!(dump.contains("- x"));
        assert!(dump.contains("Return:"));
    }

    #[test]
    fn test_dump_with_types() {
        let input = "type Message = { status: string, code: int }";
        let program = parse(input).unwrap();
        let dump = dump_program(&program);

        assert!(dump.contains("Type: Message"));
        assert!(dump.contains("ObjectType:"));
        assert!(dump.contains("status:"));
        assert!(dump.contains("code:"));
    }
}
