//! Language server for JES2 sysouts, built on `lsp-server` (synchronous).
//!
//! Each open document is a DD, analysed on every change: diagnostics are
//! pushed to the client; Outline, folding, code lenses, semantic tokens,
//! Go to Definition and hovers are served on request.
//!
//! A whole spool (every DD of a job in one file) is cut into its DDs, each
//! analysed as if it had been opened on its own.
//!
//! Step programs (`PGM=`) come from the JESJCL of the same job: an open
//! document, or `JESJCL.txt` next to a downloaded file. When a JESJCL is
//! opened, the other DDs of its job are analysed again.
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
use std::time::Instant;

use anyhow::Result;
use jes_reader_core::{Dd, DdKind, Options, Rules, analyze, parse_document, parse_file};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument, DidOpenTextDocument,
    LogMessage, Notification as LspNotification, PublishDiagnostics, ShowMessage,
};
use lsp_types::request::{
    CodeLensRefresh, CodeLensRequest, DocumentSymbolRequest, FoldingRangeRequest, GotoDefinition,
    HoverRequest, Request as LspRequest, SemanticTokensFullRequest, SemanticTokensRefresh,
};
use lsp_types::{
    CodeLensOptions, CodeLensParams, Diagnostic, DocumentSymbolParams, DocumentSymbolResponse,
    FoldingRangeParams, FoldingRangeProviderCapability, GotoDefinitionParams,
    GotoDefinitionResponse, Hover, HoverContents, HoverParams, HoverProviderCapability,
    InitializeParams, Location, LogMessageParams, MarkupContent, MarkupKind, MessageType, OneOf,
    Position, PublishDiagnosticsParams, SemanticTokensFullOptions, SemanticTokensOptions,
    SemanticTokensParams, SemanticTokensResult, SemanticTokensServerCapabilities,
    ServerCapabilities, ShowMessageParams, TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
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
        code_lens_provider: Some(CodeLensOptions {
            resolve_provider: Some(false),
        }),
        semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
            SemanticTokensOptions {
                legend: convert::semantic_legend(),
                full: Some(SemanticTokensFullOptions::Bool(true)),
                range: None,
                work_done_progress_options: Default::default(),
            },
        )),
        ..ServerCapabilities::default()
    }
}

/// Open document: its URI, its text and its analysis.
struct Document {
    uri: Uri,
    text: String,
    /// One DD, or the DDs of a whole spool.
    dds: Vec<Dd>,
    /// Diagnostics last published for it.
    diagnostics: Vec<Diagnostic>,
}

impl Document {
    /// DD that holds line `line` (the last one starting before it).
    fn dd_at(&self, line: usize) -> &Dd {
        self.dds
            .iter()
            .rev()
            .find(|dd| dd.first_line <= line)
            .unwrap_or(&self.dds[0])
    }
}

/// A JESJCL and the document that holds it.
struct Jcl {
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
    /// The client accepts `workspace/codeLens/refresh`.
    code_lens_refresh: bool,
    /// The client accepts `workspace/semanticTokens/refresh`.
    semantic_tokens_refresh: bool,
    /// Number of requests sent to the client, for their IDs.
    sent_requests: u32,
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
        code_lens_refresh: params
            .capabilities
            .workspace
            .as_ref()
            .and_then(|w| w.code_lens.as_ref())
            .and_then(|c| c.refresh_support)
            .unwrap_or(false),
        semantic_tokens_refresh: params
            .capabilities
            .workspace
            .as_ref()
            .and_then(|w| w.semantic_tokens.as_ref())
            .and_then(|c| c.refresh_support)
            .unwrap_or(false),
        sent_requests: 0,
    };
    server.log(format!(
        "jes-reader-lsp {} started (process {}), {} workspace folder(s)",
        env!("CARGO_PKG_VERSION"),
        std::process::id(),
        server.workspace.len()
    ));
    server.load_rules()?;

    // A request or notification that fails or panics is reported and
    // skipped: the server keeps running. Ending it would make the client
    // restart it, losing the analysis of every open document meanwhile, and
    // give up after a few restarts.
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                let (id, method) = (req.id.clone(), req.method.clone());
                let started = Instant::now();
                if let Err(error) = guarded(|| server.request(req)) {
                    eprintln!("jes-reader-lsp: {method} failed: {error}");
                    server.log(format!("{method} failed: {error}"));
                    let response =
                        Response::new_err(id, ErrorCode::InternalError as i32, error.clone());
                    connection.sender.send(response.into())?;
                }
                let elapsed = started.elapsed();
                if elapsed >= SLOW_REQUEST {
                    server.log(format!("{method}: {}", millis(elapsed)));
                }
            }
            Message::Notification(not) => {
                let method = not.method.clone();
                if let Err(error) = guarded(|| server.notification(not)) {
                    eprintln!("jes-reader-lsp: {method} failed: {error}");
                    server.log(format!("{method} failed: {error}"));
                }
            }
            Message::Response(_) => {}
        }
    }
    Ok(())
}

/// Requests that take at least this long are logged.
const SLOW_REQUEST: std::time::Duration = std::time::Duration::from_millis(20);

/// `12.3 ms`
fn millis(duration: std::time::Duration) -> String {
    format!("{:.1} ms", duration.as_secs_f64() * 1000.0)
}

/// Last segment of a URI path, decoded: the document's file name.
fn file_name(uri: &Uri) -> String {
    let path = decode_path(uri.path().as_str());
    path.rsplit(['/', '\\']).next().unwrap_or(&path).to_owned()
}

/// `JESMSGLG L1-47, JESJCL L48-60 (+1 column), step output L61-7500`
fn describe_dds(dds: &[Dd]) -> String {
    dds.iter()
        .map(|dd| {
            let name = dd.name.clone().unwrap_or_else(|| match dd.kind {
                DdKind::Other => "step output".to_owned(),
                kind => format!("{kind:?}"),
            });
            let offset = match dd.column_offset {
                0 => String::new(),
                n => format!(" ({n:+} column)"),
            };
            let lines = format!("L{}-{}", dd.first_line + 1, dd.first_line + dd.line_count);
            format!("{name} {lines}{offset}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Runs `handle`, turning an error or a panic into a message.
fn guarded(handle: impl FnOnce() -> Result<()>) -> std::result::Result<(), String> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(handle)) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(format!("{error:#}")),
        Err(panic) => Err(panic
            .downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| panic.downcast_ref::<String>().cloned())
            .map_or_else(|| "panic".to_owned(), |m| format!("panic: {m}"))),
    }
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
                // Severities may have changed without any document change.
                if self.semantic_tokens_refresh {
                    self.send_request(SemanticTokensRefresh::METHOD)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Loads the house rules: the `rulesFile` option, otherwise the first
    /// `.jesreader.toml` of the workspace folders or the home directory. An
    /// invalid file is reported to the user and ignored.
    fn load_rules(&mut self) -> Result<()> {
        let path = self
            .rules_file
            .clone()
            .or_else(|| jes_reader_core::rules::find(&self.workspace));
        self.options.rules = match path.as_deref().map(Rules::load) {
            Some(Ok(rules)) => {
                if let Some(path) = &path {
                    self.log(format!("house rules: {}", path.display()));
                }
                rules
            }
            Some(Err(error)) => {
                self.log(format!("house rules ignored: {error}"));
                let params = ShowMessageParams {
                    typ: MessageType::WARNING,
                    message: format!("JES Reader: house rules ignored. {error}"),
                };
                self.connection
                    .sender
                    .send(Notification::new(ShowMessage::METHOD.to_owned(), params).into())?;
                Rules::default()
            }
            None => {
                self.log("house rules: none".to_owned());
                Rules::default()
            }
        };
        Ok(())
    }

    /// Writes a line to the client's log (the JES Reader output channel).
    fn log(&self, message: String) {
        let params = LogMessageParams {
            typ: MessageType::INFO,
            message,
        };
        // A closed connection is reported by the main loop.
        let _ = self
            .connection
            .sender
            .send(Notification::new(LogMessage::METHOD.to_owned(), params).into());
    }

    /// Analyses a document and publishes its diagnostics. A JESJCL also
    /// gives the programs of the other DDs of its job: those are analysed
    /// again, and published again if their diagnostics changed.
    fn update(&mut self, uri: Uri, text: String) -> Result<()> {
        let started = Instant::now();
        let (dds, diagnostics) = self.analyse(&uri, &text);
        self.log(format!(
            "analysed {}: {} lines, {}; {} diagnostic(s) in {}",
            file_name(&uri),
            text.lines().count(),
            describe_dds(&dds),
            diagnostics.len(),
            millis(started.elapsed())
        ));
        let jcl = dds
            .iter()
            .find(|dd| dd.kind == DdKind::JesJcl && dd.job_id.is_some())
            .cloned();
        self.documents.insert(
            uri.as_str().to_owned(),
            Document {
                uri: uri.clone(),
                text,
                dds,
                diagnostics: diagnostics.clone(),
            },
        );
        self.publish(uri.clone(), diagnostics)?;

        let Some(jcl) = jcl else {
            return Ok(());
        };
        let job: Vec<(String, Uri, String)> = self
            .documents
            .iter()
            .filter(|(key, doc)| {
                key.as_str() != uri.as_str() && doc.dds.iter().any(|dd| jcl.is_jcl_of(dd))
            })
            .map(|(key, doc)| (key.clone(), doc.uri.clone(), doc.text.clone()))
            .collect();
        for (key, uri, text) in &job {
            let (dds, diagnostics) = self.analyse(uri, text);
            self.log(format!(
                "analysed {} again with the programs of the JESJCL",
                file_name(uri)
            ));
            let Some(doc) = self.documents.get_mut(key) else {
                continue;
            };
            doc.dds = dds;
            if doc.diagnostics != diagnostics {
                doc.diagnostics.clone_from(&diagnostics);
                self.publish(uri.clone(), diagnostics)?;
            }
        }
        if !job.is_empty() && self.code_lens_refresh {
            self.send_request(CodeLensRefresh::METHOD)?;
        }
        Ok(())
    }

    /// Sends a request without parameters to the client; its response is
    /// ignored.
    fn send_request(&mut self, method: &str) -> Result<()> {
        self.sent_requests += 1;
        let id = format!("jes-reader-{}", self.sent_requests);
        let request = Request::new(id.into(), method.to_owned(), ());
        self.connection.sender.send(request.into())?;
        Ok(())
    }

    /// Parses a document (one DD, or the DDs of a whole spool), fills the
    /// step programs from the JESJCL of the job and computes the diagnostics.
    fn analyse(&self, uri: &Uri, text: &str) -> (Vec<Dd>, Vec<Diagnostic>) {
        let mut dds = parse_document(&decode_path(uri.path().as_str()), text);
        let own_jcl: Vec<Dd> = dds
            .iter()
            .filter(|dd| dd.kind == DdKind::JesJcl)
            .cloned()
            .collect();
        for dd in &mut dds {
            if dd.kind == DdKind::JesJcl {
                continue;
            }
            if let Some(jcl) = own_jcl.iter().find(|jcl| jcl.is_jcl_of(dd)) {
                dd.attach_programs(jcl);
            } else if let Some(jcl) = self.jcl_of(uri, dd) {
                dd.attach_programs(&jcl.dd);
            }
        }
        let lines = Lines::new(text);
        let diagnostics = dds
            .iter()
            .flat_map(|dd| analyze(dd, &self.options))
            .map(|f| convert::diagnostic(&f, &lines, uri))
            .collect();
        (dds, diagnostics)
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
                    DocumentSymbolResponse::Nested(convert::document_symbols(
                        &doc.dds,
                        &Lines::new(&doc.text),
                    ))
                });
                Response::new_ok(req.id, symbols)
            }
            FoldingRangeRequest::METHOD => {
                let params: FoldingRangeParams = serde_json::from_value(req.params)?;
                let ranges = self
                    .document(&params.text_document.uri)
                    .map(|doc| convert::document_folding_ranges(&doc.dds));
                Response::new_ok(req.id, ranges)
            }
            GotoDefinition::METHOD => {
                let params: GotoDefinitionParams = serde_json::from_value(req.params)?;
                let at = params.text_document_position_params;
                let location = self.definition(&at.text_document.uri, at.position);
                Response::new_ok(req.id, location.map(GotoDefinitionResponse::Scalar))
            }
            CodeLensRequest::METHOD => {
                let params: CodeLensParams = serde_json::from_value(req.params)?;
                let lenses = self.document(&params.text_document.uri).map(|doc| {
                    let lines = Lines::new(&doc.text);
                    doc.dds
                        .iter()
                        .flat_map(|dd| convert::code_lenses(dd, &lines))
                        .collect::<Vec<_>>()
                });
                Response::new_ok(req.id, lenses)
            }
            SemanticTokensFullRequest::METHOD => {
                let params: SemanticTokensParams = serde_json::from_value(req.params)?;
                let tokens = self.document(&params.text_document.uri).map(|doc| {
                    SemanticTokensResult::Tokens(convert::semantic_tokens(
                        &doc.dds,
                        &self.options,
                        &Lines::new(&doc.text),
                    ))
                });
                Response::new_ok(req.id, tokens)
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
        Some((doc, navigate::target_at(doc.dd_at(line), line, column)?))
    }

    /// JESJCL of the job `dd` belongs to: in an open document first (the
    /// same whole spool, or the JESJCL that the VS Code extension opens in
    /// the background for Zowe documents), otherwise `JESJCL.txt` next to a
    /// downloaded file.
    fn jcl_of(&self, uri: &Uri, dd: &Dd) -> Option<Jcl> {
        let open = self.documents.values().find_map(|doc| {
            let jcl = doc
                .dds
                .iter()
                .find(|other| navigate::is_jcl_of(other, dd))?;
            Some((doc, jcl))
        });
        if let Some((doc, jcl)) = open {
            return Some(Jcl {
                uri: doc.uri.clone(),
                text: doc.text.clone(),
                dd: jcl.clone(),
            });
        }
        if !uri.as_str().starts_with("file:") {
            return None;
        }
        let path = navigate::file_path(&decode_path(uri.path().as_str()));
        let sibling = navigate::sibling_jcl(&path)?;
        let text = String::from_utf8_lossy(&std::fs::read(&sibling).ok()?).into_owned();
        let dd = parse_file(&sibling.to_string_lossy(), &text);
        let (directory, _) = uri.as_str().rsplit_once('/')?;
        let uri = format!("{directory}/JESJCL.txt").parse().ok()?;
        Some(Jcl { uri, text, dd })
    }

    /// JCL statement behind the cursor, in the JESJCL of the same job.
    fn definition(&self, uri: &Uri, position: Position) -> Option<Location> {
        let (doc, target) = self.target(uri, position)?;
        let dd = doc.dd_at(usize::try_from(position.line).ok()?);
        let jcl = self.jcl_of(&doc.uri, dd)?;
        let statement = navigate::resolve(target, dd, &jcl.dd)?;
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
        let dd = doc.dd_at(line);

        if let Some(message) = explain::message_at(dd, line, column) {
            let range = lines.span(line, message.id_columns.clone());
            return Some(markdown(explain::message_hover(message), Some(range)));
        }
        if let Some((code, reason)) = explain::abend_at(lines.line(line), column)
            && let Some(text) = explain::abend_hover(&code, reason.as_deref())
        {
            return Some(markdown(text, None));
        }

        let target = navigate::target_at(dd, line, column)?;
        let jcl = self.jcl_of(&doc.uri, dd);
        let value =
            navigate::hover_text(target, dd, jcl.as_ref().map(|j| (&j.dd, j.text.as_str())));
        let range = match navigate::target_columns(dd, target, line) {
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
    fn guarded_handlers() {
        assert_eq!(guarded(|| Ok(())), Ok(()));
        assert_eq!(
            guarded(|| Err(anyhow::anyhow!("bad params"))),
            Err("bad params".into())
        );
        let error = guarded(|| panic!("index out of bounds")).unwrap_err();
        assert!(error.contains("index out of bounds"), "{error}");
    }

    #[test]
    fn initialization_options() {
        assert_eq!(options(None).max_return_code, 4);
        let init = serde_json::json!({ "maxReturnCode": 8 });
        assert_eq!(options(Some(&init)).max_return_code, 8);
    }
}
