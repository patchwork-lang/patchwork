pub mod token;
pub mod adapter;
pub mod ast;

#[cfg(test)]
mod milestone3_tests;

// Include generated parser code from lalrpop
#[allow(clippy::all)]
mod patchwork {
    include!(concat!(env!("OUT_DIR"), "/patchwork.rs"));
}

pub use adapter::{LexerAdapter, ParseError};
pub use token::ParserToken;
pub use ast::*;

use patchwork_lexer::lex_str;

/// Parse a patchwork program from a string
pub fn parse(input: &str) -> Result<Program<'_>, ParseError> {
    // Create lexer
    let lexer = lex_str(input).map_err(|e| ParseError::LexerError(e.to_string()))?;

    // Create adapter
    let adapter = LexerAdapter::new(input, lexer);

    // Parse using generated parser
    patchwork::ProgramParser::new()
        .parse(input, adapter)
        .map_err(|e| ParseError::UnexpectedToken(format!("{:?}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        // Empty input should parse successfully (empty program)
        let result = parse("");
        assert!(result.is_ok(), "Failed to parse empty input: {:?}", result);

        let program = result.unwrap();
        assert_eq!(program.items.len(), 0, "Expected empty program");
    }

    #[test]
    fn test_parse_simple_import() {
        let input = "import foo";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse simple import: {:?}", result);

        let program = result.unwrap();
        assert_eq!(program.items.len(), 1);

        match &program.items[0] {
            Item::Import(decl) => {
                match &decl.path {
                    ImportPath::Simple(parts) => {
                        assert_eq!(parts.len(), 1);
                        assert_eq!(parts[0], "foo");
                    }
                    _ => panic!("Expected Simple import path"),
                }
            }
            _ => panic!("Expected Import item"),
        }
    }

    #[test]
    fn test_parse_relative_multi_import() {
        let input = "import ./{analyst, narrator, scribe}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse relative multi-import: {:?}", result);

        let program = result.unwrap();
        assert_eq!(program.items.len(), 1);

        match &program.items[0] {
            Item::Import(decl) => {
                match &decl.path {
                    ImportPath::RelativeMulti(names) => {
                        assert_eq!(names.len(), 3);
                        assert_eq!(names[0], "analyst");
                        assert_eq!(names[1], "narrator");
                        assert_eq!(names[2], "scribe");
                    }
                    _ => panic!("Expected RelativeMulti import path"),
                }
            }
            _ => panic!("Expected Import item"),
        }
    }

    #[test]
    fn test_parse_skill_empty() {
        let input = "skill foo() {}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse empty skill: {:?}", result);

        let program = result.unwrap();
        assert_eq!(program.items.len(), 1);

        match &program.items[0] {
            Item::Skill(decl) => {
                assert_eq!(decl.name, "foo");
                assert_eq!(decl.params.len(), 0);
                assert_eq!(decl.body.statements.len(), 0);
            }
            _ => panic!("Expected Skill item"),
        }
    }

    #[test]
    fn test_parse_skill_with_params() {
        let input = "skill rewriting_git_branch(changeset_description) {}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse skill with params: {:?}", result);

        let program = result.unwrap();
        assert_eq!(program.items.len(), 1);

        match &program.items[0] {
            Item::Skill(decl) => {
                assert_eq!(decl.name, "rewriting_git_branch");
                assert_eq!(decl.params.len(), 1);
                assert_eq!(decl.params[0].name, "changeset_description");
            }
            _ => panic!("Expected Skill item"),
        }
    }

    #[test]
    fn test_parse_task() {
        let input = "task analyst(session_id, work_dir, changeset) {}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse task: {:?}", result);

        let program = result.unwrap();
        assert_eq!(program.items.len(), 1);

        match &program.items[0] {
            Item::Task(decl) => {
                assert_eq!(decl.name, "analyst");
                assert_eq!(decl.params.len(), 3);
                assert_eq!(decl.params[0].name, "session_id");
                assert_eq!(decl.params[1].name, "work_dir");
                assert_eq!(decl.params[2].name, "changeset");
            }
            _ => panic!("Expected Task item"),
        }
    }

    #[test]
    fn test_parse_function() {
        let input = "fun helper(x, y) {}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse function: {:?}", result);

        let program = result.unwrap();
        assert_eq!(program.items.len(), 1);

        match &program.items[0] {
            Item::Function(decl) => {
                assert_eq!(decl.name, "helper");
                assert_eq!(decl.params.len(), 2);
                assert_eq!(decl.params[0].name, "x");
                assert_eq!(decl.params[1].name, "y");
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_parse_multiple_items() {
        let input = r#"
            import ./{analyst, narrator, scribe}

            skill rewriting_git_branch(changeset_description) {}

            task analyst(session_id) {}
            task narrator(session_id) {}

            fun helper() {}
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse multiple items: {:?}", result);

        let program = result.unwrap();
        assert_eq!(program.items.len(), 5);

        // Check item types
        assert!(matches!(program.items[0], Item::Import(_)));
        assert!(matches!(program.items[1], Item::Skill(_)));
        assert!(matches!(program.items[2], Item::Task(_)));
        assert!(matches!(program.items[3], Item::Task(_)));
        assert!(matches!(program.items[4], Item::Function(_)));
    }

    #[test]
    fn test_parse_historian_main_structure() {
        // Parse just the structure (import + skill declaration) from historian main.pw
        // Can't parse the body yet (Milestone 3+), but structure should work
        let input = r#"
            import ./{analyst, narrator, scribe}

            skill rewriting_git_branch(changeset_description) {}
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse historian main structure: {:?}", result);

        let program = result.unwrap();
        assert_eq!(program.items.len(), 2);

        // Verify import
        match &program.items[0] {
            Item::Import(decl) => {
                match &decl.path {
                    ImportPath::RelativeMulti(names) => {
                        assert_eq!(names.len(), 3);
                        assert!(names.contains(&"analyst"));
                        assert!(names.contains(&"narrator"));
                        assert!(names.contains(&"scribe"));
                    }
                    _ => panic!("Expected RelativeMulti import"),
                }
            }
            _ => panic!("Expected Import item"),
        }

        // Verify skill
        match &program.items[1] {
            Item::Skill(decl) => {
                assert_eq!(decl.name, "rewriting_git_branch");
                assert_eq!(decl.params.len(), 1);
                assert_eq!(decl.params[0].name, "changeset_description");
            }
            _ => panic!("Expected Skill item"),
        }
    }
}
