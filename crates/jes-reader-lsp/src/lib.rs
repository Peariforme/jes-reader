//! Language server for JES2 sysouts, built on `lsp-server` (synchronous).
//!
//! Each open document is a DD, analysed on every change: diagnostics are
//! pushed to the client; Outline, folding, Go to Definition and hovers are
//! served on request.
//!
//! Initialization options (`initializationOptions`):
//!
//! ```json
//! { "maxReturnCode": 4, "rulesFile": "C:/team/.jesreader.toml" }
//! ```

mod convert;
mod explain;
mod navigate;

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use jes_reader_core::{Dd, Options, Rules, analyze, parse_file};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument, DidOpenTextDocument,
    Notification as LspNotification, PublishDiagnostics, ShowMessage,
};
use lsp_types::request::{
    DocumentSymbolRequest, FoldingRangeRequest, GotoDefinition, HoverRequest, Request as LspRequest,
};
use lsp_types::{
    DocumentSymbolParams, DocumentSymbolResponse, FoldingRangeParams,
    FoldingRangeProviderCapability, GotoDefinitionParams, GotoDefinitionResponse, Hover,
    HoverContents, HoverParams, HoverProviderCapability, InitializeParams, Location, MarkupContent,
    MarkupKind, MessageType, OneOf, Position, PublishDiagnosticsParams, ServerCapabilities,
    ShowMessageParams, TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
};

use convert::Lines;

/// Capabilities announced to the client.
pub fn capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        document_symbol_provider: Some(OneOf::Left(true)),
        folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        ..ServerCapabilities::default()
    }
}

/// Open document: its URI, its text and its analysis.
struct Document {
    uri: Uri,
    text: String,
    dd: Dd,
}

struct Server<'a> {
    connection: &'a Connection,
    options: Options,
    documents: HashMap<String, Document>,
    /// Rules file chosen by the user (`rulesFile` initialization option).
    rules_file: Option<PathBuf>,
    /// Workspace folders, searched for `.jesreader.toml`.
    workspace: Vec<PathBuf>,
}

/// Performs the `initialize` handshake, then handles messages until `shutdown`.
pub fn run(connection: &Connection) -> Result<()> {
    let caps = serde_json::to_value(capabilities())?;
    let params: InitializeParams = serde_json::from_value(connection.initialize(caps)?)?;
    let init = params.initialization_options.as_ref();
    let workspace = params
        .workspace_folders
        .iter()
        .flatten()
        .filter(|folder| folder.uri.as_str().starts_with("file:"))
        .map(|folder| navigate::file_path(&decode_path(folder.uri.path().as_str())))
        .collect();
    let mut server = Server {
        connection,
        options: options(init),
        documents: HashMap::new(),
        rules_file: init
            .and_then(|v| v.get("rulesFile"))
            .and_then(serde_json::Value::as_str)
            .filter(|path| !path.is_empty())
            .map(PathBuf::from),
        workspace,
    };
    server.load_rules()?;

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
            // The client watches `.jesreader.toml` files: reload the rules
            // and analyse the open documents again.
            DidChangeWatchedFiles::METHOD => {
                self.load_rules()?;
                let documents: Vec<(Uri, String)> = self
                    .documents
                    .values()
                    .map(|doc| (doc.uri.clone(), doc.text.clone()))
                    .collect();
                for (uri, text) in documents {
                    self.update(uri, text)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Analyses a document and publishes its diagnostics.
    /// Loads the house rules: the `rulesFile` option, otherwise the first
    /// `.jesreader.toml` of the workspace folders or the home directory. An
    /// invalid file is reported to the user and ignored.
    fn load_rules(&mut self) -> Result<()> {
        let path = self
            .rules_file
            .clone()
            .or_else(|| jes_reader_core::rules::find(&self.workspace));
        self.options.rules = match path.as_deref().map(Rules::load) {
            Some(Ok(rules)) => rules,
            Some(Err(error)) => {
                let params = ShowMessageParams {
                    typ: MessageType::WARNING,
                    message: format!("JES Reader: house rules ignored. {error}"),
                };
                self.connection
                    .sender
                    .send(Notification::new(ShowMessage::METHOD.to_owned(), params).into())?;
                Rules::default()
            }
            None => Rules::default(),
        };
        Ok(())
    }

    fn update(&mut self, uri: Uri, text: String) -> Result<()> {
        let dd = parse_file(&decode_path(uri.path().as_str()), &text);
        let lines = Lines::new(&text);
        let diagnostics = analyze(&dd, &self.options)
            .iter()
            .map(|f| convert::diagnostic(f, &lines, &uri))
            .collect();
        self.documents.insert(
            uri.as_str().to_owned(),
            Document {
                uri: uri.clone(),
                text,
                dd,
            },
        );
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
            GotoDefinition::METHOD => {
                let params: GotoDefinitionParams = serde_json::from_value(req.params)?;
                let at = params.text_document_position_params;
                let location = self.definition(&at.text_document.uri, at.position);
                Response::new_ok(req.id, location.map(GotoDefinitionResponse::Scalar))
            }
            HoverRequest::METHOD => {
                let params: HoverParams = serde_json::from_value(req.params)?;
                let at = params.text_document_position_params;
                Response::new_ok(req.id, self.hover(&at.text_document.uri, at.position))
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

    /// Document under `uri` and the link target at `position`, if any.
    fn target(&self, uri: &Uri, position: Position) -> Option<(&Document, navigate::Target)> {
        let doc = self.document(uri)?;
        let line = usize::try_from(position.line).ok()?;
        let column = Lines::new(&doc.text).byte_column(line, position.character);
        Some((doc, navigate::target_at(&doc.dd, line, column)?))
    }

    /// JESJCL of the job `doc` belongs to: an open document first (the VS
    /// Code extension opens it in the background for Zowe documents),
    /// otherwise `JESJCL.txt` next to a downloaded file.
    fn jcl_of(&self, doc: &Document) -> Option<Document> {
        if let Some(jcl) = self
            .documents
            .values()
            .find(|other| navigate::is_jcl_of(&other.dd, &doc.dd))
        {
            return Some(Document {
                uri: jcl.uri.clone(),
                text: jcl.text.clone(),
                dd: jcl.dd.clone(),
            });
        }
        if !doc.uri.as_str().starts_with("file:") {
            return None;
        }
        let path = navigate::file_path(&decode_path(doc.uri.path().as_str()));
        let sibling = navigate::sibling_jcl(&path)?;
        let text = String::from_utf8_lossy(&std::fs::read(&sibling).ok()?).into_owned();
        let dd = parse_file(&sibling.to_string_lossy(), &text);
        let (directory, _) = doc.uri.as_str().rsplit_once('/')?;
        let uri = format!("{directory}/JESJCL.txt").parse().ok()?;
        Some(Document { uri, text, dd })
    }

    /// JCL statement behind the cursor, in the JESJCL of the same job.
    fn definition(&self, uri: &Uri, position: Position) -> Option<Location> {
        let (doc, target) = self.target(uri, position)?;
        let jcl = self.jcl_of(doc)?;
        let statement = navigate::resolve(target, &doc.dd, &jcl.dd)?;
        let range = Lines::new(&jcl.text).content(statement.lines.start);
        Some(Location::new(jcl.uri, range))
    }

    /// Hover, most specific first: a message ID or an abend code (from the
    /// catalog), then a link target (the JCL statement it leads to, so that
    /// the reader sees what is behind the link before following it).
    fn hover(&self, uri: &Uri, position: Position) -> Option<Hover> {
        let doc = self.document(uri)?;
        let line = usize::try_from(position.line).ok()?;
        let lines = Lines::new(&doc.text);
        let column = lines.byte_column(line, position.character);

        if let Some(message) = explain::message_at(&doc.dd, line, column) {
            let range = lines.span(line, message.id_columns.clone());
            return Some(markdown(explain::message_hover(message), Some(range)));
        }
        if let Some((code, reason)) = explain::abend_at(lines.line(line), column)
            && let Some(text) = explain::abend_hover(&code, reason.as_deref())
        {
            return Some(markdown(text, None));
        }

        let target = navigate::target_at(&doc.dd, line, column)?;
        let jcl = self.jcl_of(doc);
        let value = navigate::hover_text(
            target,
            &doc.dd,
            jcl.as_ref().map(|j| (&j.dd, j.text.as_str())),
        );
        let range = match navigate::target_columns(&doc.dd, target, line) {
            Some(columns) => lines.span(line, columns),
            None => lines.content(line),
        };
        Some(markdown(value, Some(range)))
    }
}

fn markdown(value: String, range: Option<lsp_types::Range>) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range,
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
