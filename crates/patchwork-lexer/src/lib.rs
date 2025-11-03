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

    #[test]
    fn test_empty_input() -> Result<(), ParlexError> {
        let mut lexer = lex_str("")?;
        let mut context = ();
        let token = lexer.try_next_with_context(&mut context)?;
        assert_eq!(token.map(|t| t.rule), Some(Rule::End));
        Ok(())
    }
}
