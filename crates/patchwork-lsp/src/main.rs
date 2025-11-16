use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use patchwork_parser::parse;
use patchwork_parser::ParseError;

#[derive(Clone)]
struct Backend {
    client: Client,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self { client }
    }

    async fn publish_diagnostics(&self, uri: Url, text: String) {
        let diagnostics = compute_diagnostics(&text);
        let _ = self
            .client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "patchwork-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        let _ = self.client.log_message(MessageType::INFO, "Patchwork LSP ready").await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.publish_diagnostics(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params
            .content_changes
            .into_iter()
            .last()
            .map(|c| c.text)
            .unwrap_or_default();
        self.publish_diagnostics(uri, text).await;
    }
}

fn compute_diagnostics(text: &str) -> Vec<Diagnostic> {
    match parse(text) {
        Ok(_) => Vec::new(),
        Err(err) => vec![diagnostic_from_error(err, text)],
    }
}

fn diagnostic_from_error(err: ParseError, text: &str) -> Diagnostic {
    let (message, byte_offset, span) = match err {
        ParseError::LexerError {
            message,
            byte_offset,
            span,
        } => (message, byte_offset, span),
        ParseError::UnexpectedToken {
            message,
            byte_offset,
            span,
        } => (message, byte_offset, span),
    };

    let range = if let Some((start, end)) = span {
        Range {
            start: byte_offset_to_position(text, start),
            end: byte_offset_to_position(text, end),
        }
    } else if let Some(pos) = byte_offset {
        let p = byte_offset_to_position(text, pos);
        Range { start: p, end: p }
    } else {
        Range {
            start: Position::new(0, 0),
            end: Position::new(0, 1),
        }
    };


    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        code_description: None,
        source: Some("patchwork".to_string()),
        message,
        related_information: None,
        tags: None,
        data: None,
    }
}

fn byte_offset_to_position(text: &str, byte_offset: usize) -> Position {
    let mut line = 0;
    let mut col = 0;
    let mut bytes_seen = 0;

    for (idx, ch) in text.char_indices() {
        if idx >= byte_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
        bytes_seen = idx + ch.len_utf8();
    }

    // If byte_offset points beyond the last character, position at end of text
    if byte_offset > bytes_seen {
        for ch in text[bytes_seen..].chars() {
            if bytes_seen >= byte_offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
            bytes_seen += ch.len_utf8();
        }
    }

    Position::new(line as u32, col as u32)
}

#[tokio::main]
async fn main() {
    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
