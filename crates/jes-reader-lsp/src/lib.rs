//! Language server for JES2 sysouts, built on `lsp-server` (synchronous).
//!
//! Each open document is a DD, analysed on every change: diagnostics are
//! pushed to the client, Outline and folding are served on request.
//!
//! Initialization options (`initializationOptions`):
//!
//! ```json
//! { "maxReturnCode": 4 }
//! ```

mod convert;

use std::collections::HashMap;

use anyhow::Result;
use jes_reader_core::{Dd, Options, analyze, parse_file};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument,
    Notification as LspNotification, PublishDiagnostics,
};
use lsp_types::request::{DocumentSymbolRequest, FoldingRangeRequest, Request as LspRequest};
use lsp_types::{
    DocumentSymbolParams, DocumentSymbolResponse, FoldingRangeParams,
    FoldingRangeProviderCapability, InitializeParams, OneOf, PublishDiagnosticsParams,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
};

use convert::Lines;

/// Capabilities announced to the client.
pub fn capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        document_symbol_provider: Some(OneOf::Left(true)),
        folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
        ..ServerCapabilities::default()
    }
}

/// Open document: its text and its analysis.
struct Document {
    text: String,
    dd: Dd,
}

struct Server<'a> {
    connection: &'a Connection,
    options: Options,
    documents: HashMap<String, Document>,
}

/// Performs the `initialize` handshake, then handles messages until `shutdown`.
pub fn run(connection: &Connection) -> Result<()> {
    let caps = serde_json::to_value(capabilities())?;
    let params: InitializeParams = serde_json::from_value(connection.initialize(caps)?)?;
    let mut server = Server {
        connection,
        options: options(params.initialization_options.as_ref()),
        documents: HashMap::new(),
    };

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                server.request(req)?;
            }
            Message::Notification(not) => server.notification(not)?,
            Message::Response(_) => {}
        }
    }
    Ok(())
}

/// Reads `maxReturnCode` from the initialization options, otherwise the default.
fn options(init: Option<&serde_json::Value>) -> Options {
    let mut options = Options::default();
    if let Some(max) = init
        .and_then(|v| v.get("maxReturnCode"))
        .and_then(serde_json::Value::as_u64)
        .and_then(|v| u16::try_from(v).ok())
    {
        options.max_return_code = max;
    }
    options
}

impl Server<'_> {
    fn notification(&mut self, not: Notification) -> Result<()> {
        match not.method.as_str() {
            DidOpenTextDocument::METHOD => {
                let params: <DidOpenTextDocument as LspNotification>::Params =
                    serde_json::from_value(not.params)?;
                self.update(params.text_document.uri, params.text_document.text)?;
            }
            DidChangeTextDocument::METHOD => {
                let params: <DidChangeTextDocument as LspNotification>::Params =
                    serde_json::from_value(not.params)?;
                // Full sync: the last change carries the whole text.
                if let Some(change) = params.content_changes.into_iter().last() {
                    self.update(params.text_document.uri, change.text)?;
                }
            }
            DidCloseTextDocument::METHOD => {
                let params: <DidCloseTextDocument as LspNotification>::Params =
                    serde_json::from_value(not.params)?;
                self.documents.remove(params.text_document.uri.as_str());
                self.publish(params.text_document.uri, Vec::new())?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Analyses a document and publishes its diagnostics.
    fn update(&mut self, uri: Uri, text: String) -> Result<()> {
        let dd = parse_file(&decode_path(uri.path().as_str()), &text);
        let lines = Lines::new(&text);
        let diagnostics = analyze(&dd, &self.options)
            .iter()
            .map(|f| convert::diagnostic(f, &lines))
            .collect();
        self.documents
            .insert(uri.as_str().to_owned(), Document { text, dd });
        self.publish(uri, diagnostics)
    }

    fn publish(&self, uri: Uri, diagnostics: Vec<lsp_types::Diagnostic>) -> Result<()> {
        let params = PublishDiagnosticsParams::new(uri, diagnostics, None);
        self.connection
            .sender
            .send(Notification::new(PublishDiagnostics::METHOD.to_owned(), params).into())?;
        Ok(())
    }

    fn request(&self, req: Request) -> Result<()> {
        let response = match req.method.as_str() {
            DocumentSymbolRequest::METHOD => {
                let params: DocumentSymbolParams = serde_json::from_value(req.params)?;
                let symbols = self.document(&params.text_document.uri).map(|doc| {
                    DocumentSymbolResponse::Nested(convert::symbols(
                        &doc.dd,
                        &Lines::new(&doc.text),
                    ))
                });
                Response::new_ok(req.id, symbols)
            }
            FoldingRangeRequest::METHOD => {
                let params: FoldingRangeParams = serde_json::from_value(req.params)?;
                let ranges = self
                    .document(&params.text_document.uri)
                    .map(|doc| convert::folding_ranges(&doc.dd));
                Response::new_ok(req.id, ranges)
            }
            method => Response::new_err(
                req.id,
                ErrorCode::MethodNotFound as i32,
                format!("unsupported method: {method}"),
            ),
        };
        self.connection.sender.send(response.into())?;
        Ok(())
    }

    fn document(&self, uri: &Uri) -> Option<&Document> {
        self.documents.get(uri.as_str())
    }
}

/// Decodes the `%XX` escapes of a URI path. The DD name depends on it:
/// `SYSPRINT%281%29.txt` must become `SYSPRINT(1).txt` again.
fn decode_path(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                i += 3;
            }
            (byte, _) => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoded_path() {
        assert_eq!(
            decode_path("/c%3A/corpus/JOB00896/LKED/COBRUN/SYSPRINT%281%29.txt"),
            "/c:/corpus/JOB00896/LKED/COBRUN/SYSPRINT(1).txt"
        );
        assert_eq!(decode_path("100%"), "100%");
        assert_eq!(decode_path("%zz"), "%zz");
    }

    #[test]
    fn initialization_options() {
        assert_eq!(options(None).max_return_code, 4);
        let init = serde_json::json!({ "maxReturnCode": 8 });
        assert_eq!(options(Some(&init)).max_return_code, 8);
    }
}
