use patchwork_parser::parse;
use patchwork_parser::ParseError;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Clone)]
struct Backend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, String>>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
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
    async fn initialize(&self, _: InitializeParams) -> tower_lsp::jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
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

    async fn shutdown(&self) -> tower_lsp::jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        {
            let mut docs = self.documents.write().await;
            docs.insert(params.text_document.uri.clone(), params.text_document.text.clone());
        }
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
        {
            let mut docs = self.documents.write().await;
            docs.insert(uri.clone(), text.clone());
        }
        self.publish_diagnostics(uri, text).await;
    }

    async fn hover(&self, params: HoverParams) -> tower_lsp::jsonrpc::Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let docs = self.documents.read().await;
        let text = if let Some(text) = docs.get(&uri) {
            text.clone()
        } else {
            return Ok(None);
        };

        if let Some((range, word)) = word_at_position(&text, position) {
            return Ok(Some(Hover {
                contents: HoverContents::Scalar(MarkedString::String(word)),
                range: Some(range),
            }));
        }

        Ok(None)
    }

    async fn completion(
        &self,
        _: CompletionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<CompletionResponse>> {
        // Placeholder: no completions yet.
        Ok(Some(CompletionResponse::Array(Vec::new())))
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
        let start_pos = byte_offset_to_position(text, start);
        // Ensure the range spans at least one character to avoid zero-length diagnostics.
        let end_pos = byte_offset_to_position(text, if end <= start { start + 1 } else { end });
        Range {
            start: start_pos,
            end: end_pos,
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

fn word_at_position(text: &str, position: Position) -> Option<(Range, String)> {
    let Position { line, character } = position;
    let line = line as usize;
    let character = character as usize;

    let line_str = text.lines().nth(line)?;
    if character > line_str.len() {
        return None;
    }

    let bytes = line_str.as_bytes();
    let mut start = character;
    while start > 0 && is_word_byte(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = character;
    while end < bytes.len() && is_word_byte(bytes[end]) {
        end += 1;
    }

    if start == end {
        return None;
    }

    let word = line_str[start..end].to_string();
    let range = Range {
        start: Position::new(line as u32, start as u32),
        end: Position::new(line as u32, end as u32),
    };
    Some((range, word))
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[tokio::main]
async fn main() {
    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
