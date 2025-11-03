use patchwork_lexer::{lex_str, LexerContext};
use std::env;
use std::fs;
use std::io::{self, Read};
use try_next::TryNextWithContext;

fn main() {
    let args: Vec<String> = env::args().collect();

    let input = if args.len() > 1 {
        // Read from file
        let filename = &args[1];
        fs::read_to_string(filename)
            .unwrap_or_else(|e| {
                eprintln!("Error reading file '{}': {}", filename, e);
                std::process::exit(1);
            })
    } else {
        // Read from stdin
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .unwrap_or_else(|e| {
                eprintln!("Error reading stdin: {}", e);
                std::process::exit(1);
            });
        buffer
    };

    // Convert input to bytes for span indexing
    let input_bytes = input.as_bytes();

    // Lex the input
    let mut lexer = match lex_str(&input) {
        Ok(lexer) => lexer,
        Err(e) => {
            eprintln!("Lexer error: {}", e);
            std::process::exit(1);
        }
    };

    let mut context = LexerContext::default();

    // Print tokens with location and content
    loop {
        match lexer.try_next_with_context(&mut context) {
            Ok(Some(token)) => {
                if let Some(span) = token.span {
                    // Calculate byte offsets from positions
                    let start_offset = position_to_offset(&input, span.start.line, span.start.column);
                    let end_offset = position_to_offset(&input, span.end.line, span.end.column);

                    // Extract token text
                    let text = if start_offset <= end_offset && end_offset <= input_bytes.len() {
                        String::from_utf8_lossy(&input_bytes[start_offset..end_offset]).to_string()
                    } else {
                        String::from("<invalid span>")
                    };

                    println!("{:?} @ {}:{}-{}:{} = {:?}",
                        token.rule,
                        span.start.line + 1, span.start.column + 1,
                        span.end.line + 1, span.end.column + 1,
                        text
                    );
                } else {
                    println!("{:?} (no span)", token.rule);
                }
            }
            Ok(None) => break,
            Err(e) => {
                eprintln!("Error during tokenization: {}", e);
                std::process::exit(1);
            }
        }
    }
}

/// Convert line/column position to byte offset
fn position_to_offset(input: &str, line: usize, column: usize) -> usize {
    let mut current_line = 0;
    let mut offset = 0;

    for (i, ch) in input.char_indices() {
        if current_line == line {
            let mut col = 0;
            for (j, _) in input[offset..].char_indices() {
                if col == column {
                    return offset + j;
                }
                col += 1;
            }
            return offset + input[offset..].len();
        }

        if ch == '\n' {
            current_line += 1;
            offset = i + 1;
        }
    }

    offset
}
