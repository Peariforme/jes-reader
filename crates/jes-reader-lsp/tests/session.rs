//! Full LSP session on a corpus sysout: open, diagnostics, Outline,
//! folding, close.

use lsp_server::{Connection, Message, Notification, Request, RequestId};
use lsp_types::notification::{
    DidCloseTextDocument, DidOpenTextDocument, Notification as _, PublishDiagnostics,
};
use lsp_types::request::{DocumentSymbolRequest, FoldingRangeRequest, Request as _};
use lsp_types::{
    DiagnosticSeverity, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentSymbolParams, DocumentSymbolResponse, FoldingRange, FoldingRangeParams,
    InitializeParams, NumberOrString, PublishDiagnosticsParams, TextDocumentIdentifier,
    TextDocumentItem, Uri,
};
use serde_json::Value;

struct Client {
    connection: Connection,
    next_id: i32,
}

impl Client {
    fn start() -> (Self, std::thread::JoinHandle<anyhow::Result<()>>) {
        let (server, connection) = Connection::memory();
        let handle = std::thread::spawn(move || jes_reader_lsp::run(&server));
        let mut client = Self {
            connection,
            next_id: 0,
        };
        client.request("initialize", InitializeParams::default());
        client.notify("initialized", serde_json::json!({}));
        (client, handle)
    }

    fn notify(&self, method: &str, params: impl serde::Serialize) {
        self.connection
            .sender
            .send(Notification::new(method.into(), params).into())
            .unwrap();
    }

    fn request(&mut self, method: &str, params: impl serde::Serialize) -> Value {
        self.next_id += 1;
        let id = RequestId::from(self.next_id);
        self.connection
            .sender
            .send(Request::new(id.clone(), method.into(), params).into())
            .unwrap();
        match self.connection.receiver.recv().unwrap() {
            Message::Response(resp) if resp.id == id => resp
                .response_result
                .unwrap_or_else(|e| panic!("{method}: error {e:?}")),
            other => panic!("expected a response to {method}, got {other:?}"),
        }
    }

    fn diagnostics(&self) -> PublishDiagnosticsParams {
        match self.connection.receiver.recv().unwrap() {
            Message::Notification(n) if n.method == PublishDiagnostics::METHOD => {
                serde_json::from_value(n.params).unwrap()
            }
            other => panic!("expected diagnostics, got {other:?}"),
        }
    }
}

fn corpus(path: &str) -> String {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/sysout")
        .join(path);
    String::from_utf8_lossy(&std::fs::read(file).unwrap()).into_owned()
}

#[test]
fn abend_s0c7() {
    let (mut client, handle) = Client::start();
    // Name of a document opened by Zowe Explorer, without downloading.
    let uri: Uri = "zowe-jobs:/zosmf/JOB00896/JRS0C7.JOB00896.JES2.JESYSMSG.4"
        .parse()
        .unwrap();
    client.notify(
        DidOpenTextDocument::METHOD,
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem::new(
                uri.clone(),
                "jes-sysout".into(),
                1,
                corpus("JOB00896/JES2/JESYSMSG.txt"),
            ),
        },
    );

    // Diagnostics: the abend of step GO is the only error.
    let published = client.diagnostics();
    assert_eq!(published.uri, uri);
    let errors: Vec<_> = published
        .diagnostics
        .iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
        .collect();
    assert_eq!(errors.len(), 1, "{:#?}", published.diagnostics);
    assert_eq!(errors[0].code, Some(NumberOrString::String("S0C7".into())));
    assert_eq!(errors[0].range.start.line, 130);
    assert!(errors[0].message.contains("COBRUN.GO"));

    // Outline: the three procedure steps, with their outcome.
    let symbols = client.request(
        DocumentSymbolRequest::METHOD,
        DocumentSymbolParams {
            text_document: TextDocumentIdentifier::new(uri.clone()),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
    );
    let DocumentSymbolResponse::Nested(symbols) = serde_json::from_value(symbols).unwrap() else {
        panic!("expected nested symbols");
    };
    let steps: Vec<_> = symbols
        .iter()
        .map(|s| (s.name.as_str(), s.detail.as_deref().unwrap_or("")))
        .collect();
    assert_eq!(
        steps,
        [
            ("COBRUN.COBOL", "RC 0"),
            ("COBRUN.LKED", "RC 0"),
            ("COBRUN.GO", "abend S0C7")
        ]
    );
    for s in &symbols {
        assert!(s.range.start <= s.selection_range.start && s.selection_range.end <= s.range.end);
    }

    // Folding: at least one range per step.
    let folds = client.request(
        FoldingRangeRequest::METHOD,
        FoldingRangeParams {
            text_document: TextDocumentIdentifier::new(uri.clone()),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
    );
    let folds: Vec<FoldingRange> = serde_json::from_value(folds).unwrap();
    for start in [7, 88, 118] {
        assert!(folds.iter().any(|f| f.start_line == start), "{start}");
    }

    // Close: diagnostics are cleared.
    client.notify(
        DidCloseTextDocument::METHOD,
        DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier::new(uri),
        },
    );
    assert!(client.diagnostics().diagnostics.is_empty());

    client.request("shutdown", ());
    client.notify("exit", ());
    handle.join().unwrap().unwrap();
}

#[test]
fn outline_jesjcl() {
    let (mut client, handle) = Client::start();
    let uri: Uri = "file:///corpus/JOB00896/JES2/JESJCL.txt".parse().unwrap();
    client.notify(
        DidOpenTextDocument::METHOD,
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem::new(
                uri.clone(),
                "jes-sysout".into(),
                1,
                corpus("JOB00896/JES2/JESJCL.txt"),
            ),
        },
    );
    client.diagnostics();

    let symbols = client.request(
        DocumentSymbolRequest::METHOD,
        DocumentSymbolParams {
            text_document: TextDocumentIdentifier::new(uri),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
    );
    let DocumentSymbolResponse::Nested(symbols) = serde_json::from_value(symbols).unwrap() else {
        panic!("expected nested symbols");
    };
    // JOB → job EXEC → procedure EXEC.
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "JRS0C7");
    let exec = &symbols[0].children.as_ref().unwrap()[0];
    assert_eq!(exec.name, "COBRUN");
    let proc_steps: Vec<_> = exec
        .children
        .as_ref()
        .unwrap()
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(proc_steps, ["COBOL", "LKED", "GO"]);

    client.request("shutdown", ());
    client.notify("exit", ());
    handle.join().unwrap().unwrap();
}
