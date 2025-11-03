use parlex::{LexerData, LexerDriver, ParlexError, Span};
use try_next::{IterInput, TryNextWithContext};

// Include generated lexer code from build.rs
mod lexer {
    include!(concat!(env!("OUT_DIR"), "/lexer.rs"));
}

// Re-export the main types
pub use lexer::{Mode, Rule, LexData};

/// Token produced by the patchwork lexer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchworkToken {
    pub rule: Rule,
    pub span: Option<Span>,
}

impl PatchworkToken {
    pub fn new(rule: Rule, span: Option<Span>) -> Self {
        Self { rule, span }
    }
}

impl parlex::Token for PatchworkToken {
    type TokenID = Rule;

    fn token_id(&self) -> Self::TokenID {
        self.rule
    }

    fn span(&self) -> Option<Span> {
        self.span
    }
}

/// Driver that implements lexing actions
pub struct PatchworkLexerDriver<I>(std::marker::PhantomData<I>);

impl<I> PatchworkLexerDriver<I> {
    fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<I> LexerDriver for PatchworkLexerDriver<I>
where
    I: TryNextWithContext<(), Item = u8, Error: std::fmt::Display + 'static>,
{
    type LexerData = LexData;
    type Token = PatchworkToken;
    type Lexer = parlex::Lexer<I, Self, Self::Context>;
    type Context = ();

    fn action(
        &mut self,
        lexer: &mut Self::Lexer,
        _context: &mut Self::Context,
        rule: <Self::LexerData as LexerData>::LexerRule,
    ) -> Result<(), ParlexError> {
        let span = lexer.span();
        let token = PatchworkToken::new(rule, Some(span));
        lexer.yield_token(token);
        Ok(())
    }
}

/// High-level wrapper for the patchwork lexer
pub struct PatchworkLexer<I>
where
    I: TryNextWithContext<(), Item = u8, Error: std::fmt::Display + 'static>,
{
    lexer: parlex::Lexer<I, PatchworkLexerDriver<I>, ()>,
}

impl<I> PatchworkLexer<I>
where
    I: TryNextWithContext<(), Item = u8, Error: std::fmt::Display + 'static>,
{
    pub fn try_new(input: I) -> Result<Self, ParlexError> {
        let driver = PatchworkLexerDriver::new();
        let lexer = parlex::Lexer::try_new(input, driver)?;
        Ok(Self { lexer })
    }
}

impl<I> TryNextWithContext<()> for PatchworkLexer<I>
where
    I: TryNextWithContext<(), Item = u8, Error: std::fmt::Display + 'static>,
{
    type Item = PatchworkToken;
    type Error = ParlexError;

    fn try_next_with_context(&mut self, context: &mut ()) -> Result<Option<Self::Item>, Self::Error> {
        self.lexer.try_next_with_context(context)
    }
}

/// Create a new lexer from a string
pub fn lex_str(input: &str) -> Result<PatchworkLexer<IterInput<impl Iterator<Item = u8>>>, ParlexError> {
    let iter = input.bytes();
    let input = IterInput::from(iter);
    PatchworkLexer::try_new(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to collect all tokens from input
    fn collect_tokens(input: &str) -> Result<Vec<Rule>, ParlexError> {
        let mut lexer = lex_str(input)?;
        let mut context = ();
        let mut tokens = Vec::new();

        while let Some(token) = lexer.try_next_with_context(&mut context)? {
            tokens.push(token.rule);
        }

        Ok(tokens)
    }

    #[test]
    fn test_empty_input() -> Result<(), ParlexError> {
        let tokens = collect_tokens("")?;
        assert_eq!(tokens, vec![Rule::End]);
        Ok(())
    }

    #[test]
    fn test_keywords() -> Result<(), ParlexError> {
        let tokens = collect_tokens("import from var if else for while")?;
        assert_eq!(tokens, vec![
            Rule::Import, Rule::Whitespace,
            Rule::From, Rule::Whitespace,
            Rule::Var, Rule::Whitespace,
            Rule::If, Rule::Whitespace,
            Rule::Else, Rule::Whitespace,
            Rule::For, Rule::Whitespace,
            Rule::While,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_keywords_vs_identifiers() -> Result<(), ParlexError> {
        let tokens = collect_tokens("import imported var variable")?;
        assert_eq!(tokens, vec![
            Rule::Import, Rule::Whitespace,
            Rule::Identifier, Rule::Whitespace,
            Rule::Var, Rule::Whitespace,
            Rule::Identifier,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_task_keywords() -> Result<(), ParlexError> {
        let tokens = collect_tokens("await task skill fun")?;
        assert_eq!(tokens, vec![
            Rule::Await, Rule::Whitespace,
            Rule::Task, Rule::Whitespace,
            Rule::Skill, Rule::Whitespace,
            Rule::Fun,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_control_flow_keywords() -> Result<(), ParlexError> {
        let tokens = collect_tokens("return succeed fail break in")?;
        assert_eq!(tokens, vec![
            Rule::Return, Rule::Whitespace,
            Rule::Succeed, Rule::Whitespace,
            Rule::Fail, Rule::Whitespace,
            Rule::Break, Rule::Whitespace,
            Rule::In,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_booleans() -> Result<(), ParlexError> {
        let tokens = collect_tokens("true false")?;
        assert_eq!(tokens, vec![
            Rule::True, Rule::Whitespace,
            Rule::False,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_numbers() -> Result<(), ParlexError> {
        // Note: Float literals tokenize as Number Dot Number - parser will handle this
        let tokens = collect_tokens("123 456 0 42")?;

        assert_eq!(tokens, vec![
            Rule::Number, Rule::Whitespace,
            Rule::Number, Rule::Whitespace,
            Rule::Number, Rule::Whitespace,
            Rule::Number,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_strings() -> Result<(), ParlexError> {
        let tokens = collect_tokens(r#""hello" "world" "with \"quotes\"" """#)?;
        assert_eq!(tokens, vec![
            Rule::String, Rule::Whitespace,
            Rule::String, Rule::Whitespace,
            Rule::String, Rule::Whitespace,
            Rule::String,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_identifiers() -> Result<(), ParlexError> {
        let tokens = collect_tokens("foo bar_baz _underscore CamelCase snake_case")?;
        assert_eq!(tokens, vec![
            Rule::Identifier, Rule::Whitespace,
            Rule::Identifier, Rule::Whitespace,
            Rule::Identifier, Rule::Whitespace,
            Rule::Identifier, Rule::Whitespace,
            Rule::Identifier,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_comparison_operators() -> Result<(), ParlexError> {
        let tokens = collect_tokens("== != < > <= >=")?;
        assert_eq!(tokens, vec![
            Rule::Eq, Rule::Whitespace,
            Rule::Neq, Rule::Whitespace,
            Rule::Lt, Rule::Whitespace,
            Rule::Gt, Rule::Whitespace,
            Rule::Lte, Rule::Whitespace,
            Rule::Gte,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_arithmetic_operators() -> Result<(), ParlexError> {
        let tokens = collect_tokens("+ - * / %")?;
        assert_eq!(tokens, vec![
            Rule::Plus, Rule::Whitespace,
            Rule::Minus, Rule::Whitespace,
            Rule::Star, Rule::Whitespace,
            Rule::Slash, Rule::Whitespace,
            Rule::Percent,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_logical_operators() -> Result<(), ParlexError> {
        let tokens = collect_tokens("&& || !")?;
        assert_eq!(tokens, vec![
            Rule::AndAnd, Rule::Whitespace,
            Rule::OrOr, Rule::Whitespace,
            Rule::Bang,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_other_operators() -> Result<(), ParlexError> {
        let tokens = collect_tokens("= | & -> ...")?;
        assert_eq!(tokens, vec![
            Rule::Assign, Rule::Whitespace,
            Rule::Pipe, Rule::Whitespace,
            Rule::Ampersand, Rule::Whitespace,
            Rule::Arrow, Rule::Whitespace,
            Rule::Ellipsis,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_punctuation() -> Result<(), ParlexError> {
        let tokens = collect_tokens("{ } ( ) [ ] ; , . : @")?;
        assert_eq!(tokens, vec![
            Rule::LBrace, Rule::Whitespace,
            Rule::RBrace, Rule::Whitespace,
            Rule::LParen, Rule::Whitespace,
            Rule::RParen, Rule::Whitespace,
            Rule::LBracket, Rule::Whitespace,
            Rule::RBracket, Rule::Whitespace,
            Rule::Semicolon, Rule::Whitespace,
            Rule::Comma, Rule::Whitespace,
            Rule::Dot, Rule::Whitespace,
            Rule::Colon, Rule::Whitespace,
            Rule::At,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_comments() -> Result<(), ParlexError> {
        let tokens = collect_tokens("foo # this is a comment\nbar")?;
        assert_eq!(tokens, vec![
            Rule::Identifier,
            Rule::Whitespace,
            Rule::Comment,
            Rule::Newline,
            Rule::Identifier,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_simple_code_snippet() -> Result<(), ParlexError> {
        let input = r#"var x = 42
var name = "Alice"
if x > 10 {
    return true
}"#;
        let tokens = collect_tokens(input)?;

        // Just verify it tokenizes without error and contains expected tokens
        assert!(tokens.contains(&Rule::Var));
        assert!(tokens.contains(&Rule::Identifier));
        assert!(tokens.contains(&Rule::Assign));
        assert!(tokens.contains(&Rule::Number));
        assert!(tokens.contains(&Rule::String));
        assert!(tokens.contains(&Rule::If));
        assert!(tokens.contains(&Rule::Gt));
        assert!(tokens.contains(&Rule::LBrace));
        assert!(tokens.contains(&Rule::Return));
        assert!(tokens.contains(&Rule::True));
        assert!(tokens.contains(&Rule::RBrace));
        assert!(tokens.contains(&Rule::End));

        Ok(())
    }

    #[test]
    fn test_historian_example_snippet() -> Result<(), ParlexError> {
        let input = r#"var timestamp = $(date +%Y%m%d-%H%M%S)
var session_id = "historian-${timestamp}""#;

        let tokens = collect_tokens(input)?;

        // Verify basic tokenization works
        assert!(tokens.contains(&Rule::Var));
        assert!(tokens.contains(&Rule::Identifier));
        assert!(tokens.contains(&Rule::Assign));
        assert!(tokens.contains(&Rule::String));

        Ok(())
    }
}
