use parlex::{LexerData, LexerDriver, ParlexError, Span};
use try_next::{IterInput, TryNextWithContext};

// Include generated lexer code from build.rs
mod lexer {
    include!(concat!(env!("OUT_DIR"), "/lexer.rs"));
}

// Re-export the main types
pub use lexer::{Mode, Rule, LexData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DelimiterType {
    Brace,  // Waiting for }
    Paren,  // Waiting for )
}

/// Context for tracking lexer state transitions
#[derive(Debug, Clone)]
pub struct LexerContext {
    /// Stack of mode states for handling nesting
    mode_stack: Vec<Mode>,
    /// Stack of brace depths for each nested context
    depth_stack: Vec<usize>,
    /// Stack of delimiter types (what are we waiting for to close this context)
    delimiter_stack: Vec<DelimiterType>,
    /// Last token seen (for lookahead)
    last_token: Option<Rule>,
    /// Track if we just saw a Dollar in InString mode (for interpolation)
    in_string_interpolation: bool,
    /// Track if we just saw a Dollar in Prompt mode (for interpolation)
    in_prompt_interpolation: bool,
    /// Track if we just saw a Dollar in Shell mode (for interpolation)
    in_shell_interpolation: bool,
    /// Track if we're in shell mode (for command parsing)
    in_shell_mode: bool,
    /// Track if we should return to Shell mode after yielding current token
    return_to_shell: bool,
}

impl LexerContext {
    fn new() -> Self {
        Self {
            mode_stack: vec![],
            depth_stack: vec![],
            delimiter_stack: vec![],
            last_token: None,
            in_string_interpolation: false,
            in_prompt_interpolation: false,
            in_shell_interpolation: false,
            in_shell_mode: false,
            return_to_shell: false,
        }
    }

    fn push_mode(&mut self, mode: Mode, delimiter: DelimiterType) {
        self.mode_stack.push(mode);
        self.depth_stack.push(1);
        self.delimiter_stack.push(delimiter);
    }

    fn pop_mode(&mut self) -> Option<Mode> {
        self.depth_stack.pop();
        self.delimiter_stack.pop();
        self.mode_stack.pop()
    }

    #[allow(dead_code)]
    fn current_depth(&self) -> usize {
        self.depth_stack.last().copied().unwrap_or(0)
    }

    fn increment_depth(&mut self) {
        if let Some(depth) = self.depth_stack.last_mut() {
            *depth += 1;
        }
    }

    fn decrement_depth(&mut self) -> usize {
        if let Some(depth) = self.depth_stack.last_mut() {
            if *depth > 0 {
                *depth -= 1;
            }
            *depth
        } else {
            0
        }
    }
}

impl Default for LexerContext {
    fn default() -> Self {
        Self::new()
    }
}

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
    I: TryNextWithContext<LexerContext, Item = u8, Error: std::fmt::Display + 'static>,
{
    type LexerData = LexData;
    type Token = PatchworkToken;
    type Lexer = parlex::Lexer<I, Self, Self::Context>;
    type Context = LexerContext;

    fn action(
        &mut self,
        lexer: &mut Self::Lexer,
        context: &mut Self::Context,
        rule: <Self::LexerData as LexerData>::LexerRule,
    ) -> Result<(), ParlexError> {
        // Handle state transitions BEFORE yielding token
        // This ensures the mode is set correctly before the next token is read
        match rule {
            Rule::StringStart => {
                // Entering a string - transition to InString mode
                let span = lexer.span();
                let token = PatchworkToken::new(rule, Some(span));
                lexer.yield_token(token);

                context.push_mode(Mode::InString, DelimiterType::Brace);  // Waiting for StringEnd "
                lexer.begin(Mode::InString);
                context.last_token = None;
                return Ok(());
            }
            Rule::StringEnd => {
                // Exiting a string - pop back to previous mode
                let span = lexer.span();
                let token = PatchworkToken::new(rule, Some(span));
                lexer.yield_token(token);

                if let Some(_) = context.pop_mode() {
                    // Return to the mode before the string
                    if let Some(&parent_mode) = context.mode_stack.last() {
                        lexer.begin(parent_mode);
                    } else {
                        // Back to Code mode
                        lexer.begin(Mode::Code);
                    }
                }
                context.last_token = None;
                context.in_string_interpolation = false;
                return Ok(());
            }
            Rule::Dollar if context.last_token == Some(Rule::LParen) && lexer.mode() == Mode::Code => {
                // ($ pattern - enter Shell mode for shell expression
                let span = lexer.span();
                let token = PatchworkToken::new(rule, Some(span));
                lexer.yield_token(token);

                // Enter Shell mode - will exit on matching )
                context.push_mode(Mode::Shell, DelimiterType::Paren);
                lexer.begin(Mode::Shell);
                context.in_shell_mode = true;
                context.last_token = None;
                return Ok(());
            }
            Rule::Dollar => {
                // When we see $ in InString or Prompt mode, we need to check what follows
                // If it's { or (, we'll handle that in LBrace/LParen
                // If it's an identifier, we temporarily switch to Code mode
                // In Code mode, $ followed by whitespace enters Shell mode
                // In Shell mode, $ followed by identifier stays in Shell (identifier is now active in Shell)
                let span = lexer.span();
                let token = PatchworkToken::new(rule, Some(span));
                lexer.yield_token(token);

                // Mark that we're in interpolation mode - next token should be in Code mode
                match lexer.mode() {
                    Mode::InString => {
                        context.in_string_interpolation = true;
                        lexer.begin(Mode::Code);
                    }
                    Mode::Prompt => {
                        context.in_prompt_interpolation = true;
                        lexer.begin(Mode::Code);
                    }
                    Mode::Shell => {
                        // $ in Shell mode for variable interpolation
                        // Stay in Shell mode - the grammar will handle dollar shell_arg
                    }
                    Mode::Code => {
                        // $ in Code mode might start shell mode
                        // We'll check next token (Whitespace or LParen) to decide
                    }
                }
                context.last_token = Some(rule);
                return Ok(());
            }
            Rule::Whitespace if context.last_token == Some(Rule::Dollar) && lexer.mode() == Mode::Code => {
                // $ followed by whitespace in Code mode → enter Shell mode
                let span = lexer.span();
                let token = PatchworkToken::new(rule, Some(span));
                lexer.yield_token(token);

                context.push_mode(Mode::Shell, DelimiterType::Brace);  // Will exit on newline
                lexer.begin(Mode::Shell);
                context.in_shell_mode = true;
                context.last_token = None;
                return Ok(());
            }
            Rule::Identifier if context.in_string_interpolation && context.last_token == Some(Rule::Dollar) => {
                // We're tokenizing an identifier directly after $ in a string (simple $id case)
                // This is NOT ${...}, so return to InString mode after identifier
                let span = lexer.span();
                let token = PatchworkToken::new(rule, Some(span));
                lexer.yield_token(token);

                // Return to InString mode
                context.in_string_interpolation = false;
                lexer.begin(Mode::InString);
                context.last_token = None;
                return Ok(());
            }
            Rule::Identifier if context.in_prompt_interpolation && context.last_token == Some(Rule::Dollar) => {
                // We're tokenizing an identifier directly after $ in a prompt (simple $id case)
                // This is NOT ${...}, so return to Prompt mode after identifier
                let span = lexer.span();
                let token = PatchworkToken::new(rule, Some(span));
                lexer.yield_token(token);

                // Return to Prompt mode
                context.in_prompt_interpolation = false;
                lexer.begin(Mode::Prompt);
                context.last_token = None;
                return Ok(());
            }
            Rule::Think | Rule::Ask => {
                // When we see think/ask, record it. On next LBrace, transition to Prompt
                context.last_token = Some(rule);
            }
            Rule::Do => {
                // When we see do in Prompt state, record it. On next LBrace, transition to Code
                context.last_token = Some(rule);
            }
            Rule::LBrace => {
                // First yield the token
                let span = lexer.span();
                let token = PatchworkToken::new(rule, Some(span));
                lexer.yield_token(token);

                // Then check if this follows a context operator and transition states
                match context.last_token {
                    Some(Rule::Think) | Some(Rule::Ask) => {
                        // Transition Code -> Prompt
                        context.push_mode(Mode::Prompt, DelimiterType::Brace);
                        lexer.begin(Mode::Prompt);
                    }
                    Some(Rule::Do) if lexer.mode() == Mode::Prompt => {
                        // Transition Prompt -> Code
                        context.push_mode(Mode::Code, DelimiterType::Brace);
                        lexer.begin(Mode::Code);
                    }
                    Some(Rule::Dollar) if context.in_string_interpolation => {
                        // ${expression} in string - stay in Code mode and track depth
                        context.push_mode(Mode::Code, DelimiterType::Brace);
                        // Stay in Code mode (already there from Dollar handling)
                    }
                    Some(Rule::Dollar) if context.in_prompt_interpolation => {
                        // ${expression} in prompt - stay in Code mode and track depth
                        context.push_mode(Mode::Code, DelimiterType::Brace);
                        // Stay in Code mode (already there from Dollar handling)
                    }
                    Some(Rule::Dollar) if lexer.mode() == Mode::Shell || context.in_shell_mode => {
                        // ${expression} in shell mode - switch to Code mode temporarily
                        context.in_shell_interpolation = true;
                        context.push_mode(Mode::Code, DelimiterType::Brace);
                        lexer.begin(Mode::Code);
                    }
                    _ => {
                        // Just increment depth for nested braces
                        context.increment_depth();
                    }
                }
                context.last_token = None;
                return Ok(());
            }
            Rule::LParen if context.last_token == Some(Rule::Dollar) => {
                // $(...) - behavior depends on current mode
                let span = lexer.span();
                let token = PatchworkToken::new(rule, Some(span));
                lexer.yield_token(token);

                // In Code mode OR Prompt mode: $(command) enters Shell mode for command substitution
                // In InString mode: $(expr) stays in Code mode for expression (to support nested expressions)
                if !context.in_string_interpolation || context.in_prompt_interpolation {
                    // Code/Prompt mode: enter Shell mode for command substitution
                    context.push_mode(Mode::Shell, DelimiterType::Paren);
                    lexer.begin(Mode::Shell);
                    context.in_shell_mode = true;
                } else {
                    // InString mode only: stay in Code mode for nested expressions like "${func($(cmd))}"
                    context.push_mode(Mode::Code, DelimiterType::Paren);
                    // Already in Code mode from Dollar handling
                }
                context.last_token = None;
                return Ok(());
            }
            Rule::LParen if lexer.mode() == Mode::Code => {
                // Track LParen to detect ($ pattern
                context.last_token = Some(rule);
            }
            Rule::RParen if context.in_shell_mode && context.delimiter_stack.last() == Some(&DelimiterType::Paren) => {
                // ) in shell mode with Paren delimiter → exit shell mode
                let span = lexer.span();
                let token = PatchworkToken::new(rule, Some(span));
                lexer.yield_token(token);

                let depth = context.decrement_depth();
                if depth == 0 {
                    if let Some(_) = context.pop_mode() {
                        context.in_shell_mode = false;
                        // Return to parent mode
                        if let Some(&parent_mode) = context.mode_stack.last() {
                            lexer.begin(parent_mode);
                        } else {
                            // Back to Code mode
                            if context.in_string_interpolation {
                                context.in_string_interpolation = false;
                                lexer.begin(Mode::InString);
                            } else if context.in_prompt_interpolation {
                                context.in_prompt_interpolation = false;
                                lexer.begin(Mode::Prompt);
                            } else {
                                lexer.begin(Mode::Code);
                            }
                        }
                    }
                }
                context.last_token = None;
                return Ok(());
            }
            Rule::RParen if context.in_string_interpolation || context.in_prompt_interpolation => {
                let span = lexer.span();
                let token = PatchworkToken::new(rule, Some(span));
                lexer.yield_token(token);

                // Only handle closing of $(command) - check if top of delimiter stack is Paren
                if context.delimiter_stack.last() == Some(&DelimiterType::Paren) {
                    let depth = context.decrement_depth();
                    if depth == 0 {
                        if let Some(_) = context.pop_mode() {
                            // Check if we're still in a nested interpolation context
                            if let Some(&parent_mode) = context.mode_stack.last() {
                                // Still nested - return to parent mode (could be Code from ${...})
                                lexer.begin(parent_mode);
                            } else {
                                // No more nesting - return to original mode (InString or Prompt)
                                if context.in_string_interpolation {
                                    context.in_string_interpolation = false;
                                    lexer.begin(Mode::InString);
                                } else if context.in_prompt_interpolation {
                                    context.in_prompt_interpolation = false;
                                    lexer.begin(Mode::Prompt);
                                }
                            }
                        }
                    }
                }
                // Otherwise this is just a normal RParen in an expression like ${func(...)}
                context.last_token = None;
                return Ok(());
            }
            Rule::RBrace => {
                // First yield the token
                let span = lexer.span();
                let token = PatchworkToken::new(rule, Some(span));
                lexer.yield_token(token);

                // Then decrement depth and potentially pop mode
                let depth = context.decrement_depth();
                if depth == 0 {
                    // Pop back to previous mode
                    if let Some(_prev_mode) = context.pop_mode() {
                        // If we had a mode on stack, we need to return to the mode before that
                        if let Some(&parent_mode) = context.mode_stack.last() {
                            // Returning to a parent mode after closing interpolation/block
                            // Clear the interpolation flag if we're done with interpolation
                            if parent_mode == Mode::InString && context.in_string_interpolation {
                                // Finished ${...} or $(...) in string, back to parent string
                                context.in_string_interpolation = false;
                            } else if parent_mode == Mode::Prompt && context.in_prompt_interpolation {
                                // Finished ${...} or $(...) in prompt, back to parent prompt
                                context.in_prompt_interpolation = false;
                            } else if parent_mode == Mode::Shell && context.in_shell_interpolation {
                                // Finished ${...} in shell, back to parent shell
                                context.in_shell_interpolation = false;
                            }
                            lexer.begin(parent_mode);
                        } else {
                            // Back to Code, InString, Prompt, or Shell mode
                            if context.in_string_interpolation {
                                // Closing ${...} - return to InString
                                context.in_string_interpolation = false;
                                lexer.begin(Mode::InString);
                            } else if context.in_prompt_interpolation {
                                // Closing ${...} - return to Prompt
                                context.in_prompt_interpolation = false;
                                lexer.begin(Mode::Prompt);
                            } else if context.in_shell_interpolation {
                                // Closing ${...} - return to Shell
                                context.in_shell_interpolation = false;
                                lexer.begin(Mode::Shell);
                            } else {
                                lexer.begin(Mode::Code);
                            }
                        }
                    }
                }
                context.last_token = None;
                return Ok(());
            }
            Rule::Newline if context.in_shell_mode => {
                // Newline in shell mode → exit shell mode (unless backslash-escaped)
                let span = lexer.span();
                let token = PatchworkToken::new(rule, Some(span));
                lexer.yield_token(token);

                // Check if last token was backslash (line continuation)
                if context.last_token != Some(Rule::ShellBackslash) {
                    // Not escaped - exit shell mode
                    if let Some(_) = context.pop_mode() {
                        if let Some(&parent_mode) = context.mode_stack.last() {
                            lexer.begin(parent_mode);
                        } else {
                            lexer.begin(Mode::Code);
                        }
                    }
                    context.in_shell_mode = false;
                }
                context.last_token = None;
                return Ok(());
            }
            Rule::Whitespace | Rule::Newline => {
                // Keep last token for whitespace - don't clear it
            }
            _ => {
                // Clear last token for any other token
                context.last_token = None;
            }
        }

        // Yield token for all other rules
        let span = lexer.span();
        let token = PatchworkToken::new(rule, Some(span));
        lexer.yield_token(token);
        Ok(())
    }
}

/// High-level wrapper for the patchwork lexer
pub struct PatchworkLexer<I>
where
    I: TryNextWithContext<LexerContext, Item = u8, Error: std::fmt::Display + 'static>,
{
    lexer: parlex::Lexer<I, PatchworkLexerDriver<I>, LexerContext>,
}

impl<I> PatchworkLexer<I>
where
    I: TryNextWithContext<LexerContext, Item = u8, Error: std::fmt::Display + 'static>,
{
    pub fn try_new(input: I) -> Result<Self, ParlexError> {
        let driver = PatchworkLexerDriver::new();
        let lexer = parlex::Lexer::try_new(input, driver)?;
        Ok(Self { lexer })
    }
}

impl<I> TryNextWithContext<LexerContext> for PatchworkLexer<I>
where
    I: TryNextWithContext<LexerContext, Item = u8, Error: std::fmt::Display + 'static>,
{
    type Item = PatchworkToken;
    type Error = ParlexError;

    fn try_next_with_context(&mut self, context: &mut LexerContext) -> Result<Option<Self::Item>, Self::Error> {
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
        let mut context = LexerContext::new();
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
    fn test_strings_chunked() -> Result<(), ParlexError> {
        let tokens = collect_tokens(r#""hello""#)?;
        assert_eq!(tokens, vec![
            Rule::StringStart,
            Rule::StringText,
            Rule::StringEnd,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_string_empty() -> Result<(), ParlexError> {
        let tokens = collect_tokens(r#""""#)?;
        assert_eq!(tokens, vec![
            Rule::StringStart,
            Rule::StringEnd,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_string_with_escapes() -> Result<(), ParlexError> {
        let tokens = collect_tokens(r#""with \"quotes\"""#)?;
        assert_eq!(tokens, vec![
            Rule::StringStart,
            Rule::StringText,  // "with \"quotes\""
            Rule::StringEnd,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_string_escape_sequences() -> Result<(), ParlexError> {
        let tokens = collect_tokens(r#""Hello\nworld\t!""#)?;
        assert_eq!(tokens, vec![
            Rule::StringStart,
            Rule::StringText,  // "Hello\nworld\t!"
            Rule::StringEnd,
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
        assert!(tokens.contains(&Rule::StringStart));
        assert!(tokens.contains(&Rule::StringEnd));
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
        assert!(tokens.contains(&Rule::StringStart));

        Ok(())
    }

    #[test]
    fn test_simple_think_block() -> Result<(), ParlexError> {
        let input = "think { Hello world }";
        let tokens = collect_tokens(input)?;

        // PromptText matches non-whitespace sequences, Whitespace is separate
        assert_eq!(tokens, vec![
            Rule::Think,
            Rule::Whitespace,
            Rule::LBrace,
            Rule::Whitespace,
            Rule::PromptText,  // "Hello"
            Rule::Whitespace,
            Rule::PromptText,  // "world"
            Rule::Whitespace,
            Rule::RBrace,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_simple_ask_block() -> Result<(), ParlexError> {
        let input = "ask { What should I do? }";
        let tokens = collect_tokens(input)?;

        // PromptText matches non-whitespace sequences
        assert_eq!(tokens, vec![
            Rule::Ask,
            Rule::Whitespace,
            Rule::LBrace,
            Rule::Whitespace,
            Rule::PromptText,  // "What"
            Rule::Whitespace,
            Rule::PromptText,  // "should"
            Rule::Whitespace,
            Rule::PromptText,  // "I"
            Rule::Whitespace,
            Rule::PromptText,  // "do?"
            Rule::Whitespace,
            Rule::RBrace,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_think_with_do_block() -> Result<(), ParlexError> {
        let input = "think { Analyze this do { var x = 1 } }";
        let tokens = collect_tokens(input)?;

        // PromptText now matches non-whitespace sequences
        assert_eq!(tokens, vec![
            Rule::Think,
            Rule::Whitespace,
            Rule::LBrace,
            Rule::Whitespace,
            Rule::PromptText,  // "Analyze"
            Rule::Whitespace,
            Rule::PromptText,  // "this"
            Rule::Whitespace,
            Rule::Do,
            Rule::Whitespace,
            Rule::LBrace,
            Rule::Whitespace,
            Rule::Var,
            Rule::Whitespace,
            Rule::Identifier,  // x
            Rule::Whitespace,
            Rule::Assign,
            Rule::Whitespace,
            Rule::Number,  // 1
            Rule::Whitespace,
            Rule::RBrace,  // closes do block
            Rule::Whitespace,
            Rule::RBrace,  // closes think block
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_do_without_brace_in_prompt() -> Result<(), ParlexError> {
        let input = "think { What should I do here }";
        let tokens = collect_tokens(input)?;

        // "do" without following "{" stays as Do token but doesn't trigger transition
        assert_eq!(tokens, vec![
            Rule::Think,
            Rule::Whitespace,
            Rule::LBrace,
            Rule::Whitespace,
            Rule::PromptText,  // "What"
            Rule::Whitespace,
            Rule::PromptText,  // "should"
            Rule::Whitespace,
            Rule::PromptText,  // "I"
            Rule::Whitespace,
            Rule::Do,          // "do" - recognized but doesn't transition without {
            Rule::Whitespace,
            Rule::PromptText,  // "here"
            Rule::Whitespace,
            Rule::RBrace,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_nested_think_blocks() -> Result<(), ParlexError> {
        let input = "think { Outer do { think { Inner } } }";
        let tokens = collect_tokens(input)?;

        assert_eq!(tokens, vec![
            Rule::Think,
            Rule::Whitespace,
            Rule::LBrace,
            Rule::Whitespace,
            Rule::PromptText,  // "Outer"
            Rule::Whitespace,
            Rule::Do,
            Rule::Whitespace,
            Rule::LBrace,
            Rule::Whitespace,
            Rule::Think,
            Rule::Whitespace,
            Rule::LBrace,
            Rule::Whitespace,
            Rule::PromptText,  // "Inner"
            Rule::Whitespace,
            Rule::RBrace,  // closes inner think
            Rule::Whitespace,
            Rule::RBrace,  // closes do
            Rule::Whitespace,
            Rule::RBrace,  // closes outer think
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_do_as_identifier_in_code() -> Result<(), ParlexError> {
        let input = "var do_something = 1";
        let tokens = collect_tokens(input)?;

        // "do" should work as part of identifier in Code state
        assert_eq!(tokens, vec![
            Rule::Var,
            Rule::Whitespace,
            Rule::Identifier,  // do_something
            Rule::Whitespace,
            Rule::Assign,
            Rule::Whitespace,
            Rule::Number,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_bash_substitution() -> Result<(), ParlexError> {
        let input = r#"var timestamp = $(date +%Y%m%d-%H%M%S)"#;
        let tokens = collect_tokens(input)?;

        // Now tokenizes as individual tokens instead of BashSubst
        assert!(tokens.contains(&Rule::Var));
        assert!(tokens.contains(&Rule::Identifier));
        assert!(tokens.contains(&Rule::Assign));
        assert!(tokens.contains(&Rule::Dollar));
        assert!(tokens.contains(&Rule::LParen));
        assert!(tokens.contains(&Rule::RParen));
        assert!(tokens.contains(&Rule::End));
        Ok(())
    }

    #[test]
    fn test_bash_substitution_complex() -> Result<(), ParlexError> {
        let input = r#"var current_branch = $(git rev_parse --abbrev_ref HEAD)"#;
        let tokens = collect_tokens(input)?;

        // Now tokenizes as individual tokens
        assert!(tokens.contains(&Rule::Var));
        assert!(tokens.contains(&Rule::Identifier));
        assert!(tokens.contains(&Rule::Assign));
        assert!(tokens.contains(&Rule::Dollar));
        assert!(tokens.contains(&Rule::LParen));
        assert!(tokens.contains(&Rule::RParen));
        assert!(tokens.contains(&Rule::End));
        Ok(())
    }

    #[test]
    fn test_historian_main_example() -> Result<(), ParlexError> {
        let input = include_str!("../../../examples/historian/main.pw");
        let tokens = collect_tokens(input)?;

        // Just verify it tokenizes without error
        // Should contain various expected token types
        assert!(tokens.contains(&Rule::Import));
        assert!(tokens.contains(&Rule::Skill));
        assert!(tokens.contains(&Rule::Var));
        assert!(tokens.contains(&Rule::Dollar));  // Changed from BashSubst
        assert!(tokens.contains(&Rule::StringStart));
        assert!(tokens.contains(&Rule::Await));
        assert!(tokens.contains(&Rule::Task));
        assert!(tokens.contains(&Rule::End));

        Ok(())
    }

    #[test]
    fn test_historian_analyst_example() -> Result<(), ParlexError> {
        let input = include_str!("../../../examples/historian/analyst.pw");
        let tokens = collect_tokens(input)?;

        // Should tokenize without error
        assert!(tokens.contains(&Rule::Import));
        assert!(tokens.contains(&Rule::Task));
        assert!(tokens.contains(&Rule::Think));
        assert!(tokens.contains(&Rule::Ask));
        assert!(tokens.contains(&Rule::End));

        Ok(())
    }

    #[test]
    fn test_historian_narrator_example() -> Result<(), ParlexError> {
        let input = include_str!("../../../examples/historian/narrator.pw");
        let tokens = collect_tokens(input)?;

        // Should tokenize without error
        assert!(tokens.contains(&Rule::Import));
        assert!(tokens.contains(&Rule::Task));
        assert!(tokens.contains(&Rule::Fun));
        assert!(tokens.contains(&Rule::Think));
        assert!(tokens.contains(&Rule::End));

        Ok(())
    }

    #[test]
    fn test_historian_scribe_example() -> Result<(), ParlexError> {
        let input = include_str!("../../../examples/historian/scribe.pw");
        let tokens = collect_tokens(input)?;

        // Should tokenize without error
        assert!(tokens.contains(&Rule::Import));
        assert!(tokens.contains(&Rule::Task));
        assert!(tokens.contains(&Rule::Think));
        assert!(tokens.contains(&Rule::Do));
        assert!(tokens.contains(&Rule::End));

        Ok(())
    }

    // String interpolation tests
    #[test]
    fn test_string_interpolation_identifier() -> Result<(), ParlexError> {
        let input = r#""Hello $name""#;
        let tokens = collect_tokens(input)?;

        assert_eq!(tokens, vec![
            Rule::StringStart,
            Rule::StringText,     // "Hello "
            Rule::Dollar,
            Rule::Identifier,     // name
            Rule::StringEnd,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_string_interpolation_multiple() -> Result<(), ParlexError> {
        let input = r#""Hello $first $last""#;
        let tokens = collect_tokens(input)?;

        assert_eq!(tokens, vec![
            Rule::StringStart,
            Rule::StringText,     // "Hello "
            Rule::Dollar,
            Rule::Identifier,     // first
            Rule::StringText,     // " "
            Rule::Dollar,
            Rule::Identifier,     // last
            Rule::StringEnd,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_string_interpolation_expression() -> Result<(), ParlexError> {
        let input = r#""Total: ${x + y}""#;
        let tokens = collect_tokens(input)?;

        assert_eq!(tokens, vec![
            Rule::StringStart,
            Rule::StringText,     // "Total: "
            Rule::Dollar,
            Rule::LBrace,
            Rule::Identifier,     // x
            Rule::Whitespace,
            Rule::Plus,
            Rule::Whitespace,
            Rule::Identifier,     // y
            Rule::RBrace,
            Rule::StringEnd,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_string_interpolation_command() -> Result<(), ParlexError> {
        let input = r#""Date: $(date)""#;
        let tokens = collect_tokens(input)?;

        assert_eq!(tokens, vec![
            Rule::StringStart,
            Rule::StringText,     // "Date: "
            Rule::Dollar,
            Rule::LParen,
            Rule::Identifier,     // date
            Rule::RParen,
            Rule::StringEnd,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_string_interpolation_complex_expression() -> Result<(), ParlexError> {
        let input = r#""Result: ${func(x, y)}""#;
        let tokens = collect_tokens(input)?;

        assert_eq!(tokens, vec![
            Rule::StringStart,
            Rule::StringText,     // "Result: "
            Rule::Dollar,
            Rule::LBrace,
            Rule::Identifier,     // func
            Rule::LParen,         // (
            Rule::Identifier,     // x
            Rule::Comma,
            Rule::Whitespace,
            Rule::Identifier,     // y
            Rule::RParen,
            Rule::RBrace,
            Rule::StringEnd,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_string_interpolation_nested() -> Result<(), ParlexError> {
        let input = r#""Outer ${f("inner")}""#;
        let tokens = collect_tokens(input)?;

        assert_eq!(tokens, vec![
            Rule::StringStart,
            Rule::StringText,     // "Outer "
            Rule::Dollar,
            Rule::LBrace,
            Rule::Identifier,     // f
            Rule::LParen,         // (
            Rule::StringStart,    // nested string
            Rule::StringText,     // "inner"
            Rule::StringEnd,      // nested string end
            Rule::RParen,
            Rule::RBrace,
            Rule::StringEnd,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_string_only_interpolation() -> Result<(), ParlexError> {
        let input = r#""$name""#;
        let tokens = collect_tokens(input)?;

        assert_eq!(tokens, vec![
            Rule::StringStart,
            Rule::Dollar,
            Rule::Identifier,     // name
            Rule::StringEnd,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_string_interpolation_mixed() -> Result<(), ParlexError> {
        let input = r#""Name: $name, Age: ${age + 1}, Date: $(date)""#;
        let tokens = collect_tokens(input)?;

        // Verify it contains all the expected token types
        assert!(tokens.contains(&Rule::StringStart));
        assert!(tokens.contains(&Rule::StringEnd));
        assert!(tokens.contains(&Rule::Dollar));
        assert!(tokens.contains(&Rule::Identifier));
        assert!(tokens.contains(&Rule::LBrace));
        assert!(tokens.contains(&Rule::RBrace));
        assert!(tokens.contains(&Rule::LParen));
        assert!(tokens.contains(&Rule::RParen));
        assert!(tokens.contains(&Rule::Plus));
        assert!(tokens.contains(&Rule::End));

        Ok(())
    }

    #[test]
    fn test_string_interpolation_deeply_nested() -> Result<(), ParlexError> {
        // Test mixed nesting: ${ ... $( ... ) ... }
        let input = r#""Result: ${x + $(cmd)}""#;
        let tokens = collect_tokens(input)?;

        assert_eq!(tokens, vec![
            Rule::StringStart,
            Rule::StringText,     // "Result: "
            Rule::Dollar,
            Rule::LBrace,         // Start ${
            Rule::Identifier,     // x
            Rule::Whitespace,
            Rule::Plus,
            Rule::Whitespace,
            Rule::Dollar,
            Rule::LParen,         // Start $(
            Rule::Identifier,     // cmd
            Rule::RParen,         // End $) - should stay in Code mode
            Rule::RBrace,         // End $} - should return to InString
            Rule::StringEnd,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_string_interpolation_triple_nested() -> Result<(), ParlexError> {
        // Test deep nesting: ${ ... $( ... ${ ... } ... ) ... }
        let input = r#""A: ${a + $(b + ${c})}""#;
        let tokens = collect_tokens(input)?;

        assert_eq!(tokens, vec![
            Rule::StringStart,
            Rule::StringText,     // "A: "
            Rule::Dollar,
            Rule::LBrace,         // Start ${a...}
            Rule::Identifier,     // a
            Rule::Whitespace,
            Rule::Plus,
            Rule::Whitespace,
            Rule::Dollar,
            Rule::LParen,         // Start $(b...)
            Rule::Identifier,     // b
            Rule::Whitespace,
            Rule::Plus,
            Rule::Whitespace,
            Rule::Dollar,
            Rule::LBrace,         // Start ${c}
            Rule::Identifier,     // c
            Rule::RBrace,         // End ${c} - back to Code mode (inside $(...))
            Rule::RParen,         // End $(...) - back to Code mode (inside ${...})
            Rule::RBrace,         // End ${...} - back to InString
            Rule::StringEnd,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_string_escaped_dollar() -> Result<(), ParlexError> {
        // Test \$ escape for literal dollar sign
        let tokens = collect_tokens(r#""Price: \$100""#)?;
        assert_eq!(tokens, vec![
            Rule::StringStart,
            Rule::StringText,  // "Price: \$100" - includes escaped dollar
            Rule::StringEnd,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_string_escaped_dollar_mixed() -> Result<(), ParlexError> {
        // Test \$ escape mixed with actual interpolation
        let tokens = collect_tokens(r#""Price: \$${amount}""#)?;
        assert_eq!(tokens, vec![
            Rule::StringStart,
            Rule::StringText,  // "Price: \$"
            Rule::Dollar,
            Rule::LBrace,
            Rule::Identifier,  // amount
            Rule::RBrace,
            Rule::StringEnd,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_single_quote_string_simple() -> Result<(), ParlexError> {
        // Test simple single-quoted string
        let tokens = collect_tokens(r#"'hello world'"#)?;
        assert_eq!(tokens, vec![
            Rule::SingleQuoteString,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_single_quote_string_no_interpolation() -> Result<(), ParlexError> {
        // Test that $var is literal in single-quoted strings
        let tokens = collect_tokens(r#"'Price: $100 for ${item}'"#)?;
        assert_eq!(tokens, vec![
            Rule::SingleQuoteString,  // Contains literal "$100 for ${item}"
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_single_quote_string_with_escapes() -> Result<(), ParlexError> {
        // Test escape sequences in single-quoted strings
        let tokens = collect_tokens(r#"'can\'t and \\ backslash'"#)?;
        assert_eq!(tokens, vec![
            Rule::SingleQuoteString,  // Contains "can\'t and \\ backslash"
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_single_vs_double_quotes() -> Result<(), ParlexError> {
        // Test difference between single and double quotes
        let tokens = collect_tokens(r#"'$name' "$name""#)?;
        assert_eq!(tokens, vec![
            Rule::SingleQuoteString,  // '$name' - literal
            Rule::Whitespace,
            Rule::StringStart,
            Rule::Dollar,             // "$name" - interpolated
            Rule::Identifier,
            Rule::StringEnd,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_prompt_interpolation_identifier() -> Result<(), ParlexError> {
        // Test $identifier in prompt context
        let input = r#"think { Analyze $filename }"#;
        let tokens = collect_tokens(input)?;

        assert_eq!(tokens, vec![
            Rule::Think,
            Rule::Whitespace,
            Rule::LBrace,
            Rule::Whitespace,
            Rule::PromptText,      // "Analyze"
            Rule::Whitespace,
            Rule::Dollar,
            Rule::Identifier,      // filename
            Rule::Whitespace,
            Rule::RBrace,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_prompt_interpolation_expression() -> Result<(), ParlexError> {
        // Test ${expression} in prompt context
        let input = r#"think { Check ${x + y} items }"#;
        let tokens = collect_tokens(input)?;

        assert_eq!(tokens, vec![
            Rule::Think,
            Rule::Whitespace,
            Rule::LBrace,
            Rule::Whitespace,
            Rule::PromptText,      // "Check"
            Rule::Whitespace,
            Rule::Dollar,
            Rule::LBrace,
            Rule::Identifier,      // x
            Rule::Whitespace,
            Rule::Plus,
            Rule::Whitespace,
            Rule::Identifier,      // y
            Rule::RBrace,
            Rule::Whitespace,
            Rule::PromptText,      // "items"
            Rule::Whitespace,
            Rule::RBrace,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_prompt_interpolation_command() -> Result<(), ParlexError> {
        // Test $(command) in prompt context
        let input = r#"ask { What is $(date) today? }"#;
        let tokens = collect_tokens(input)?;

        assert_eq!(tokens, vec![
            Rule::Ask,
            Rule::Whitespace,
            Rule::LBrace,
            Rule::Whitespace,
            Rule::PromptText,      // "What"
            Rule::Whitespace,
            Rule::PromptText,      // "is"
            Rule::Whitespace,
            Rule::Dollar,
            Rule::LParen,
            Rule::ShellArg,        // date (in Shell mode now, not Code mode)
            Rule::RParen,
            Rule::Whitespace,
            Rule::PromptText,      // "today?"
            Rule::Whitespace,
            Rule::RBrace,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_prompt_interpolation_mixed() -> Result<(), ParlexError> {
        // Test multiple interpolation forms in one prompt
        let input = r#"think { Process $file with ${count} items from $(source) }"#;
        let tokens = collect_tokens(input)?;

        // Verify it contains all the expected token types
        assert!(tokens.contains(&Rule::Think));
        assert!(tokens.contains(&Rule::PromptText));
        assert!(tokens.contains(&Rule::Dollar));
        assert!(tokens.contains(&Rule::Identifier));
        assert!(tokens.contains(&Rule::LBrace));
        assert!(tokens.contains(&Rule::RBrace));
        assert!(tokens.contains(&Rule::LParen));
        assert!(tokens.contains(&Rule::RParen));
        assert!(tokens.contains(&Rule::Plus) == false);  // count not used in expression here
        assert!(tokens.contains(&Rule::End));
        Ok(())
    }

    // Shell mode tests (Milestone 10)
    #[test]
    fn test_shell_mode_basic() -> Result<(), ParlexError> {
        let input = "$ mkdir work_dir\n";
        let tokens = collect_tokens(input)?;

        assert_eq!(tokens, vec![
            Rule::Dollar,
            Rule::Whitespace,
            Rule::ShellArg,      // "mkdir"
            Rule::Whitespace,
            Rule::ShellArg,      // "work_dir"
            Rule::Newline,
            Rule::End
        ]);
        Ok(())
    }

    #[test]
    fn test_shell_mode_with_flags() -> Result<(), ParlexError> {
        let input = "$ mkdir -p work_dir\n";
        let tokens = collect_tokens(input)?;

        // Should tokenize flags as shell args
        assert!(tokens.contains(&Rule::Dollar));
        assert!(tokens.contains(&Rule::ShellArg));
        assert!(tokens.contains(&Rule::Newline));
        assert!(tokens.contains(&Rule::End));
        Ok(())
    }

    #[test]
    fn test_shell_command_substitution() -> Result<(), ParlexError> {
        let input = r#"var branch = $(git rev_parse HEAD)"#;
        let tokens = collect_tokens(input)?;

        // $(command) should enter shell mode
        assert!(tokens.contains(&Rule::Var));
        assert!(tokens.contains(&Rule::Identifier));  // branch
        assert!(tokens.contains(&Rule::Assign));
        assert!(tokens.contains(&Rule::Dollar));
        assert!(tokens.contains(&Rule::LParen));
        assert!(tokens.contains(&Rule::ShellArg));    // git, rev_parse, HEAD
        assert!(tokens.contains(&Rule::RParen));
        assert!(tokens.contains(&Rule::End));
        Ok(())
    }

    #[test]
    fn test_shell_expression_form() -> Result<(), ParlexError> {
        let input = r#"if ($ test -f file.txt) { }"#;
        let tokens = collect_tokens(input)?;

        // ($ command) should enter shell mode
        assert!(tokens.contains(&Rule::If));
        assert!(tokens.contains(&Rule::LParen));
        assert!(tokens.contains(&Rule::Dollar));
        assert!(tokens.contains(&Rule::ShellArg));    // test, -f, file.txt
        assert!(tokens.contains(&Rule::RParen));
        assert!(tokens.contains(&Rule::LBrace));
        assert!(tokens.contains(&Rule::RBrace));
        assert!(tokens.contains(&Rule::End));
        Ok(())
    }

    #[test]
    fn test_shell_mode_dollar_interpolation() -> Result<(), ParlexError> {
        // Test actual usage pattern: ${} inside quoted strings in shell commands
        let input = "$ git diff \"${base_commit}\"..HEAD\n";
        let tokens = collect_tokens(input)?;
        // ${base_commit} works because it's inside a quoted string
        // Sequence: $ git diff " ${ base_commit } " ..HEAD newline
        assert!(tokens.contains(&Rule::Dollar));       // initial $
        assert!(tokens.contains(&Rule::ShellArg));     // git, diff, ..HEAD
        assert!(tokens.contains(&Rule::StringStart));  // opening "
        assert!(tokens.contains(&Rule::LBrace));       // ${ interpolation
        assert!(tokens.contains(&Rule::Identifier));   // base_commit in Code mode
        assert!(tokens.contains(&Rule::RBrace));       // } closes interpolation
        assert!(tokens.contains(&Rule::StringEnd));    // closing "
        assert!(tokens.contains(&Rule::Newline));
        Ok(())
    }
}
