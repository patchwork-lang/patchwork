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
    Export,
    From,
    Var,
    If,
    Else,
    For,
    While,
    Await,
    Worker,
    Trait,
    Skill,
    Fun,
    Default,
    Type,
    Return,
    Succeed,
    Throw,
    Break,
    SelfKw,
    In,
    Underscore,

    // Literals
    True,
    False,
    Number(&'input str),
    Identifier(&'input str),

    // Multi-character operators
    Ellipsis,
    Arrow,
    Eq,
    Neq,
    Lte,
    Gte,
    AndAnd,
    OrOr,
    PlusPlus,
    MinusMinus,

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
    Question,
    Assign,
    Pipe,
    Ampersand,
    Dot,

    // Prompt text
    PromptText(&'input str),
    PromptEscape(&'input str),

    // Comments
    Comment(&'input str),

    // Shell mode tokens
    ShellArg(&'input str),
    ShellRedirectOut,        // >
    ShellRedirectAppend,     // >>
    ShellRedirectIn,         // <
    ShellRedirectErr,        // 2>
    ShellRedirectErrToOut,   // 2>&1
    ShellPipe,               // |
    ShellAnd,                // &&
    ShellOr,                 // ||
    ShellBackground,         // &
    ShellAssign,             // =
    ShellBackslash,          // \

    // Special
    End,
    ErrorAny(&'input str),
}
