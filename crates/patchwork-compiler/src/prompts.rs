/// Prompt block extraction and markdown generation
///
/// Handles compilation of `think { }` and `ask { }` blocks into markdown templates
/// and generates the runtime IPC coordination code.

use patchwork_parser::ast::*;
use crate::error::{CompileError, Result};
use std::collections::HashSet;

/// A compiled prompt template with its metadata
#[derive(Debug, Clone, PartialEq)]
pub struct PromptTemplate {
    /// Unique identifier for this template (e.g., "think_0", "ask_1")
    pub id: String,
    /// The type of prompt (think or ask)
    pub kind: PromptKind,
    /// Worker name this prompt belongs to (e.g., "example", "narrator")
    pub worker_name: String,
    /// Generated markdown content with ${variable} placeholders intact
    pub markdown: String,
    /// Set of variable names that need to be bound at runtime
    pub required_bindings: HashSet<String>,
}

/// Type of prompt block
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    Think,
    Ask,
}

impl PromptKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            PromptKind::Think => "think",
            PromptKind::Ask => "ask",
        }
    }
}

/// Extracts a prompt block into a markdown template
pub fn extract_prompt_template(
    block: &PromptBlock,
    kind: PromptKind,
    id: String,
    worker_name: String,
) -> Result<PromptTemplate> {
    let mut markdown = String::new();
    let mut required_bindings = HashSet::new();

    for (idx, item) in block.items.iter().enumerate() {
        match item {
            PromptItem::Text(text) => {
                // Plain text goes directly into markdown
                // But check if we need to prepend a space after an interpolation
                if idx > 0 {
                    if let PromptItem::Interpolation(_) = &block.items[idx - 1] {
                        // Previous item was interpolation - check if text starts with non-whitespace
                        if !text.is_empty() && !text.starts_with(char::is_whitespace) {
                            markdown.push(' ');
                        }
                    }
                }
                markdown.push_str(text);
            }
            PromptItem::Interpolation(expr) => {
                // Variable references become ${...} placeholders in markdown
                // and we track what needs to be bound
                extract_variable_refs(expr, &mut required_bindings)?;

                // Ensure spacing before interpolation
                // The lexer emits separate Whitespace tokens, but LALRPOP skips them
                // So we need to add a space if the preceding text doesn't end with whitespace
                if !markdown.is_empty() && !markdown.ends_with(char::is_whitespace) {
                    markdown.push(' ');
                }

                // Generate the placeholder syntax
                markdown.push_str("${");
                write_expr_as_placeholder(&mut markdown, expr)?;
                markdown.push('}');
            }
            PromptItem::Code(_block) => {
                // Embedded code in prompts not yet supported
                return Err(CompileError::Unsupported(
                    "Embedded code blocks (do { }) in prompts not yet supported ".into()
                ));
            }
        }
    }

    Ok(PromptTemplate {
        id,
        kind,
        worker_name,
        markdown,
        required_bindings,
    })
}

/// Extract all variable references from an expression
fn extract_variable_refs(expr: &Expr, refs: &mut HashSet<String>) -> Result<()> {
    match expr {
        Expr::Identifier(name) => {
            refs.insert((*name).to_string());
        }
        Expr::Member { object, field: _ } => {
            // For member access like obj.field, we only need to bind the root object
            extract_variable_refs(object, refs)?;
        }
        Expr::Index { object, index } => {
            extract_variable_refs(object, refs)?;
            extract_variable_refs(index, refs)?;
        }
        Expr::Binary { left, right, .. } => {
            extract_variable_refs(left, refs)?;
            extract_variable_refs(right, refs)?;
        }
        Expr::Unary { operand, .. } => {
            extract_variable_refs(operand, refs)?;
        }
        Expr::Call { callee, args } => {
            extract_variable_refs(callee, refs)?;
            for arg in args {
                extract_variable_refs(arg, refs)?;
            }
        }
        Expr::Paren(inner) => {
            extract_variable_refs(inner, refs)?;
        }
        Expr::Array(items) => {
            for item in items {
                extract_variable_refs(item, refs)?;
            }
        }
        Expr::Object(fields) => {
            for field in fields {
                if let Some(value) = &field.value {
                    extract_variable_refs(value, refs)?;
                }
            }
        }
        Expr::String(lit) => {
            // String with interpolations
            for part in &lit.parts {
                if let StringPart::Interpolation(expr) = part {
                    extract_variable_refs(expr, refs)?;
                }
            }
        }
        // Literals don't contain variable references
        Expr::Number(_) | Expr::True | Expr::False => {}

        // Don't support complex expressions in prompts yet
        Expr::Await(_) | Expr::PostIncrement(_) | Expr::PostDecrement(_) => {
            return Err(CompileError::Unsupported(
                "Complex expressions in prompt interpolations not yet supported ".into()
            ));
        }

        // Shell and prompt expressions shouldn't appear in prompt interpolations
        Expr::BareCommand { .. } | Expr::CommandSubst(_) | Expr::ShellPipe { .. } |
        Expr::ShellAnd { .. } | Expr::ShellOr { .. } | Expr::ShellRedirect { .. } |
        Expr::Think(_) | Expr::Ask(_) | Expr::Do(_) => {
            return Err(CompileError::Unsupported(
                "Shell commands and nested prompts cannot appear in prompt interpolations".into()
            ));
        }
    }
    Ok(())
}

/// Write an expression as a placeholder string (e.g., "name" or "obj.field")
fn write_expr_as_placeholder(out: &mut String, expr: &Expr) -> Result<()> {
    match expr {
        Expr::Identifier(name) => {
            out.push_str(name);
        }
        Expr::Member { object, field } => {
            write_expr_as_placeholder(out, object)?;
            out.push('.');
            out.push_str(field);
        }
        Expr::Index { object, index } => {
            write_expr_as_placeholder(out, object)?;
            out.push('[');
            write_expr_as_placeholder(out, index)?;
            out.push(']');
        }
        _ => {
            // Complex expressions in prompt placeholders not yet supported
            return Err(CompileError::Unsupported(
                "Complex expressions in prompt placeholders not fully supported yet".into()
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_text() {
        let block = PromptBlock {
            items: vec![
                PromptItem::Text("Hello, world!"),
            ],
        };

        let template = extract_prompt_template(&block, PromptKind::Think, "think_0".into(), "test_worker".into()).unwrap();
        assert_eq!(template.markdown, "Hello, world!");
        assert_eq!(template.worker_name, "test_worker");
        assert!(template.required_bindings.is_empty());
    }

    #[test]
    fn test_extract_with_interpolation() {
        let block = PromptBlock {
            items: vec![
                PromptItem::Text("Hello, "),
                PromptItem::Interpolation(Expr::Identifier("name")),
                PromptItem::Text("!"),
            ],
        };

        let template = extract_prompt_template(&block, PromptKind::Think, "think_0".into(), "example".into()).unwrap();
        assert_eq!(template.markdown, "Hello, ${name}!");
        assert_eq!(template.worker_name, "example");
        assert!(template.required_bindings.contains("name"));
    }

    #[test]
    fn test_extract_member_access() {
        let block = PromptBlock {
            items: vec![
                PromptItem::Text("User: "),
                PromptItem::Interpolation(Expr::Member {
                    object: Box::new(Expr::Identifier("user")),
                    field: "name",
                }),
            ],
        };

        let template = extract_prompt_template(&block, PromptKind::Ask, "ask_0".into(), "greeter".into()).unwrap();
        assert_eq!(template.markdown, "User: ${user.name}");
        assert_eq!(template.worker_name, "greeter");
        // Should only bind "user", not "name"
        assert!(template.required_bindings.contains("user"));
        assert_eq!(template.required_bindings.len(), 1);
    }
}
