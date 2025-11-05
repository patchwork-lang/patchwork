/// Parser token with lifetime-carrying string slices
/// Maps lexer Rule enum to parser tokens with &'input str references
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParserToken<'input> {
    // Whitespace
    Whitespace(&'input str),
    Newline(&'input str),

    // String literals (chunked tokenization)
    StringStart,
    StringEnd,
    StringText(&'input str),
    SingleQuoteString(&'input str),
    Dollar,

    // Prompt operators
    Think,
    Ask,
    Do,

    // Keywords
    Import,
    From,
    Var,
    If,
    Else,
    For,
    While,
    Await,
    Task,
    Skill,
    Fun,
    Type,
    Return,
    Succeed,
    Fail,
    Break,
    SelfKw,
    In,

    // Literals
    True,
    False,
    Number(&'input str),
    Identifier(&'input str),
    IdentifierCall(&'input str),  // identifier( with no space - function call

    // Multi-character operators
    Ellipsis,
    Arrow,
    Eq,
    Neq,
    Lte,
    Gte,
    AndAnd,
    OrOr,

    // Punctuation
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Semicolon,
    Comma,
    Colon,
    At,

    // Single-character operators
    Lt,
    Gt,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    Assign,
    Pipe,
    Ampersand,
    Dot,

    // Prompt text
    PromptText(&'input str),

    // Comments
    Comment(&'input str),

    // Special
    End,
    ErrorAny(&'input str),
}
