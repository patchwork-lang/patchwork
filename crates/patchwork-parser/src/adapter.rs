use patchwork_lexer::{LexerContext, PatchworkToken, Rule};
use try_next::TryNextWithContext;
use crate::token::ParserToken;

/// Build a lookup table of line start byte offsets
fn build_line_starts(input: &str) -> Vec<usize> {
    let mut line_starts = vec![0]; // Line 0 starts at byte 0
    for (i, ch) in input.char_indices() {
        if ch == '\n' {
            line_starts.push(i + 1); // Next line starts after the newline
        }
    }
    line_starts
}

/// Convert line/column position to byte offset using precomputed line starts
fn position_to_offset(input: &str, line_starts: &[usize], line: usize, column: usize) -> usize {
    // Get the start of the requested line
    let line_start = if line < line_starts.len() {
        line_starts[line]
    } else {
        // Line beyond end of file
        return input.len();
    };

    // Walk forward by `column` characters from line start
    let mut col = 0;
    for (offset, _) in input[line_start..].char_indices() {
        if col == column {
            return line_start + offset;
        }
        col += 1;
    }

    // Column beyond end of line
    line_start + input[line_start..].len()
}

/// Error type for the parser
#[derive(Debug)]
pub enum ParseError {
    LexerError(String),
    UnexpectedToken(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ParseError::LexerError(msg) => write!(f, "Lexer error: {}", msg),
            ParseError::UnexpectedToken(msg) => write!(f, "Unexpected token: {}", msg),
        }
    }
}

impl std::error::Error for ParseError {}

/// Adapter that wraps a patchwork lexer and produces tokens in lalrpop format
/// Implements Iterator<Item = Result<Spanned<ParserToken, usize>, ParseError>>
pub struct LexerAdapter<'input, L>
where
    L: TryNextWithContext<LexerContext, Item = PatchworkToken, Error: std::fmt::Display>,
{
    input: &'input str,
    lexer: L,
    context: LexerContext,
    /// Precomputed byte offsets of line starts for efficient position->offset conversion
    line_starts: Vec<usize>,
}

impl<'input, L> LexerAdapter<'input, L>
where
    L: TryNextWithContext<LexerContext, Item = PatchworkToken, Error: std::fmt::Display>,
{
    pub fn new(input: &'input str, lexer: L) -> Self {
        let line_starts = build_line_starts(input);
        Self {
            input,
            lexer,
            context: LexerContext::default(),
            line_starts,
        }
    }

    /// Convert lexer Rule + span to ParserToken with &'input str references
    fn convert_token(&self, rule: Rule, start: usize, end: usize) -> ParserToken<'input> {
        // Defensive check for invalid spans
        if start > end {
            panic!("Invalid token span for {:?}: start={} > end={}", rule, start, end);
        }
        let text = &self.input[start..end];

        match rule {
            Rule::Empty => ParserToken::End,  // Empty rule maps to End token
            Rule::Whitespace => ParserToken::Whitespace(text),
            Rule::Newline => ParserToken::Newline(text),
            Rule::StringStart => ParserToken::StringStart,
            Rule::StringEnd => ParserToken::StringEnd,
            Rule::StringText => ParserToken::StringText(text),
            Rule::SingleQuoteString => ParserToken::SingleQuoteString(text),
            Rule::Dollar => ParserToken::Dollar,
            Rule::Think => ParserToken::Think,
            Rule::Ask => ParserToken::Ask,
            Rule::Do => ParserToken::Do,
            Rule::Import => ParserToken::Import,
            Rule::Export => ParserToken::Export,
            Rule::From => ParserToken::From,
            Rule::Var => ParserToken::Var,
            Rule::If => ParserToken::If,
            Rule::Else => ParserToken::Else,
            Rule::For => ParserToken::For,
            Rule::While => ParserToken::While,
            Rule::Await => ParserToken::Await,
            Rule::Worker => ParserToken::Worker,
            Rule::Trait => ParserToken::Trait,
            Rule::Skill => ParserToken::Skill,
            Rule::Fun => ParserToken::Fun,
            Rule::Default => ParserToken::Default,
            Rule::Type => ParserToken::Type,
            Rule::Return => ParserToken::Return,
            Rule::Succeed => ParserToken::Succeed,
            Rule::Throw => ParserToken::Throw,
            Rule::Break => ParserToken::Break,
            Rule::SelfKw => ParserToken::SelfKw,
            Rule::In => ParserToken::In,
            Rule::Underscore => ParserToken::Underscore,
            Rule::True => ParserToken::True,
            Rule::False => ParserToken::False,
            Rule::Number => ParserToken::Number(text),
            Rule::Identifier => ParserToken::Identifier(text),
            Rule::Ellipsis => ParserToken::Ellipsis,
            Rule::Arrow => ParserToken::Arrow,
            Rule::Eq => ParserToken::Eq,
            Rule::Neq => ParserToken::Neq,
            Rule::Lte => ParserToken::Lte,
            Rule::Gte => ParserToken::Gte,
            Rule::AndAnd => ParserToken::AndAnd,
            Rule::OrOr => ParserToken::OrOr,
            Rule::LBrace => ParserToken::LBrace,
            Rule::RBrace => ParserToken::RBrace,
            Rule::LParen => ParserToken::LParen,
            Rule::RParen => ParserToken::RParen,
            Rule::LBracket => ParserToken::LBracket,
            Rule::RBracket => ParserToken::RBracket,
            Rule::Semicolon => ParserToken::Semicolon,
            Rule::Comma => ParserToken::Comma,
            Rule::Colon => ParserToken::Colon,
            Rule::At => ParserToken::At,
            Rule::Lt => ParserToken::Lt,
            Rule::Gt => ParserToken::Gt,
            Rule::PlusPlus => ParserToken::PlusPlus,
            Rule::MinusMinus => ParserToken::MinusMinus,
            Rule::Plus => ParserToken::Plus,
            Rule::Minus => ParserToken::Minus,
            Rule::Star => ParserToken::Star,
            Rule::Slash => ParserToken::Slash,
            Rule::Percent => ParserToken::Percent,
            Rule::Bang => ParserToken::Bang,
            Rule::Question => ParserToken::Question,
            Rule::Assign => ParserToken::Assign,
            Rule::Pipe => ParserToken::Pipe,
            Rule::Ampersand => ParserToken::Ampersand,
            Rule::Dot => ParserToken::Dot,
            Rule::PromptText => ParserToken::PromptText(text),
            Rule::PromptEscape => {
                // Extract the character from $'<char>' pattern
                // The text will be "$'x'" where x is the escaped character
                // We want to extract just the 'x'
                let escaped_char = &text[2..text.len()-1];
                ParserToken::PromptEscape(escaped_char)
            },
            Rule::Comment => ParserToken::Comment(text),
            // Shell mode tokens
            Rule::ShellArg => ParserToken::ShellArg(text),
            Rule::ShellRedirectOut => ParserToken::ShellRedirectOut,
            Rule::ShellRedirectAppend => ParserToken::ShellRedirectAppend,
            Rule::ShellRedirectIn => ParserToken::ShellRedirectIn,
            Rule::ShellRedirectErr => ParserToken::ShellRedirectErr,
            Rule::ShellRedirectErrToOut => ParserToken::ShellRedirectErrToOut,
            Rule::ShellPipe => ParserToken::ShellPipe,
            Rule::ShellAnd => ParserToken::ShellAnd,
            Rule::ShellOr => ParserToken::ShellOr,
            Rule::ShellBackground => ParserToken::ShellBackground,
            Rule::ShellAssign => ParserToken::ShellAssign,
            Rule::ShellBackslash => ParserToken::ShellBackslash,
            Rule::End => ParserToken::End,
            Rule::ErrorAny => ParserToken::ErrorAny(text),
        }
    }
}

impl<'input, L> Iterator for LexerAdapter<'input, L>
where
    L: TryNextWithContext<LexerContext, Item = PatchworkToken, Error: std::fmt::Display>,
{
    type Item = Result<(usize, ParserToken<'input>, usize), ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.lexer.try_next_with_context(&mut self.context) {
                Ok(Some(token)) => {
                    // Skip whitespace and comments, but KEEP newlines for statement separation
                    if matches!(token.rule, Rule::Whitespace | Rule::Comment) {
                        continue;
                    }

                    let span = token.span.unwrap_or_else(|| {
                        // Default span if none provided
                        parlex::Span { start: parlex::Position::default(), end: parlex::Position::default() }
                    });
                    // Convert line/column positions to byte offsets using cached line starts
                    let start = position_to_offset(self.input, &self.line_starts, span.start.line, span.start.column);
                    let end = position_to_offset(self.input, &self.line_starts, span.end.line, span.end.column);

                    // Workaround for lexer span tracking bug in prompt mode with interpolation
                    // If we get an invalid span, skip this token and try the next one
                    if start > end {
                        eprintln!("Warning: Skipping token {:?} with invalid span {}..{}", token.rule, start, end);
                        continue;
                    }

                    let parser_token = self.convert_token(token.rule, start, end);
                    return Some(Ok((start, parser_token, end)));
                }
                Ok(None) => return None,
                Err(e) => return Some(Err(ParseError::LexerError(e.to_string()))),
            }
        }
    }
}
