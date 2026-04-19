use tokio::sync::Mutex;

use senbonzakura_core::errors::CompileError;
use senbonzakura_core::lexer::lexer::Lexer;
use senbonzakura_core::parser::parser::Parser;
use senbonzakura_core::source::SourceFile;
use senbonzakura_core::typechecker::checker::TypeChecker;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,
    documents: Mutex<std::collections::HashMap<Url, String>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Mutex::new(std::collections::HashMap::new()),
        }
    }

    async fn check_document(&self, uri: &Url, text: &str) {
        let source = SourceFile::from_string(text);
        let diagnostics = match self.compile(&source) {
            Ok(()) => vec![],
            Err(e) => {
                let span = &e.diagnostic.span;
                let range = Range {
                    start: Position {
                        line: span.start.line.saturating_sub(1) as u32,
                        character: span.start.column as u32,
                    },
                    end: Position {
                        line: span.end.line.saturating_sub(1) as u32,
                        character: span.end.column as u32,
                    },
                };
                vec![Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("senbonzakura".to_string()),
                    message: e.diagnostic.message.clone(),
                    ..Default::default()
                }]
            }
        };

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }

    fn compile(&self, source: &SourceFile) -> std::result::Result<(), CompileError> {
        let tokens = Lexer::new(source).tokenize()?;
        let module = Parser::new(tokens, source).parse_module()?;
        TypeChecker::new().check_module(&module)?;
        Ok(())
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
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "senbonzakura LSP initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        self.documents
            .lock()
            .await
            .insert(uri.clone(), text.clone());
        self.check_document(&uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if let Some(change) = params.content_changes.into_iter().last() {
            let text = change.text;
            self.documents
                .lock()
                .await
                .insert(uri.clone(), text.clone());
            self.check_document(&uri, &text).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(text) = self.documents.lock().await.get(&uri).cloned() {
            self.check_document(&uri, &text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.lock().await.remove(&uri);
        self.client
            .publish_diagnostics(uri, vec![], None)
            .await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let text = match self.documents.lock().await.get(uri).cloned() {
            Some(t) => t,
            None => return Ok(None),
        };


        let lines: Vec<&str> = text.lines().collect();
        let line = match lines.get(pos.line as usize) {
            Some(l) => l,
            None => return Ok(None),
        };

        let col = pos.character as usize;
        if col >= line.len() {
            return Ok(None);
        }


        let chars: Vec<char> = line.chars().collect();
        if !chars.get(col).map_or(false, |c| c.is_alphanumeric() || *c == '_') {
            return Ok(None);
        }

        let start = (0..col)
            .rev()
            .take_while(|&i| chars[i].is_alphanumeric() || chars[i] == '_')
            .last()
            .unwrap_or(col);
        let end = (col..chars.len())
            .take_while(|&i| chars[i].is_alphanumeric() || chars[i] == '_')
            .last()
            .map_or(col, |i| i + 1);

        let word: String = chars[start..end].iter().collect();


        let source = SourceFile::from_string(&text);
        let info = self.get_type_info(&source, &word);

        match info {
            Some(type_str) => Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("```senbonzakura\n{word}: {type_str}\n```"),
                }),
                range: Some(Range {
                    start: Position {
                        line: pos.line,
                        character: start as u32,
                    },
                    end: Position {
                        line: pos.line,
                        character: end as u32,
                    },
                }),
            })),
            None => Ok(None),
        }
    }
}

impl Backend {
    fn get_type_info(&self, source: &SourceFile, name: &str) -> Option<String> {
        let tokens = Lexer::new(source).tokenize().ok()?;
        let module = Parser::new(tokens, source).parse_module().ok()?;
        let mut checker = TypeChecker::new();
        let _ = checker.check_module(&module);
        checker.lookup_type(name)
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
