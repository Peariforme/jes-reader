//! Full LSP sessions on corpus sysouts: open, diagnostics, Outline, folding,
//! Go to Definition, close.

use lsp_server::{Connection, Message, Notification, Request, RequestId};
use lsp_types::notification::{
    DidChangeWatchedFiles, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    PublishDiagnostics,
};
use lsp_types::request::{
    DocumentSymbolRequest, FoldingRangeRequest, GotoDefinition, HoverRequest, Request as _,
};
use lsp_types::{
    DiagnosticSeverity, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentSymbolParams, DocumentSymbolResponse, FoldingRange, FoldingRangeParams,
    GotoDefinitionParams, GotoDefinitionResponse, HoverParams, InitializeParams, Location,
    NumberOrString, Position, PublishDiagnosticsParams, TextDocumentIdentifier, TextDocumentItem,
    TextDocumentPositionParams, Uri,
};
use serde_json::Value;

struct Client {
    connection: Connection,
    next_id: i32,
}

impl Client {
    fn start() -> (Self, std::thread::JoinHandle<anyhow::Result<()>>) {
        Self::start_with(InitializeParams::default())
    }

    fn start_with(params: InitializeParams) -> (Self, std::thread::JoinHandle<anyhow::Result<()>>) {
        let (server, connection) = Connection::memory();
        let handle = std::thread::spawn(move || jes_reader_lsp::run(&server));
        let mut client = Self {
            connection,
            next_id: 0,
        };
        client.request("initialize", params);
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
        match next(&self.connection.receiver) {
            Message::Response(resp) if resp.id == id => resp
                .response_result
                .unwrap_or_else(|e| panic!("{method}: error {e:?}")),
            other => panic!("expected a response to {method}, got {other:?}"),
        }
    }

    fn diagnostics(&self) -> PublishDiagnosticsParams {
        match next(&self.connection.receiver) {
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

/// `file:` URI of a corpus file, as the editor would send it.
fn corpus_uri(path: &str) -> Uri {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/sysout")
        .join(path)
        .to_string_lossy()
        .replace('\\', "/");
    let uri = if file.starts_with('/') {
        format!("file://{file}")
    } else {
        format!("file:///{file}")
    };
    uri.parse().unwrap()
}

impl Client {
    /// Opens a document and consumes the diagnostics it triggers.
    fn open(&self, uri: &Uri, text: String) {
        self.notify(
            DidOpenTextDocument::METHOD,
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem::new(uri.clone(), "jes-sysout".into(), 1, text),
            },
        );
        self.diagnostics();
    }

    fn definition(&mut self, uri: &Uri, line: u32, character: u32) -> Option<Location> {
        let result = self.request(
            GotoDefinition::METHOD,
            GotoDefinitionParams {
                text_document_position_params: TextDocumentPositionParams::new(
                    TextDocumentIdentifier::new(uri.clone()),
                    Position::new(line, character),
                ),
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            },
        );
        match serde_json::from_value(result).unwrap() {
            Some(GotoDefinitionResponse::Scalar(location)) => Some(location),
            None => None,
            other => panic!("expected a single location, got {other:?}"),
        }
    }

    fn stop(mut self, handle: std::thread::JoinHandle<anyhow::Result<()>>) {
        self.request("shutdown", ());
        self.notify("exit", ());
        handle.join().unwrap().unwrap();
    }
}

#[test]
fn definition_with_open_jcl() {
    let (mut client, handle) = Client::start();
    // Zowe documents: the extension opens JESJCL in the background.
    let sysmsg: Uri = "zowe-jobs:/zosmf/JOB00896/JRS0C7.JOB00896.JES2.JESYSMSG.4"
        .parse()
        .unwrap();
    let msglg: Uri = "zowe-jobs:/zosmf/JOB00896/JRS0C7.JOB00896.JES2.JESMSGLG.2"
        .parse()
        .unwrap();
    let jcl: Uri = "zowe-jobs:/zosmf/JOB00896/JRS0C7.JOB00896.JES2.JESJCL.3"
        .parse()
        .unwrap();
    client.open(&sysmsg, corpus("JOB00896/JES2/JESYSMSG.txt"));
    client.open(&msglg, corpus("JOB00896/JES2/JESMSGLG.txt"));
    client.open(&jcl, corpus("JOB00896/JES2/JESJCL.txt"));

    // `        11 IEF655I …`: statement 11 of JESJCL.
    let location = client.definition(&sysmsg, 2, 9).unwrap();
    assert_eq!(location.uri, jcl);
    assert_eq!(location.range.start.line, 26);
    // On the message ID itself: nothing.
    assert!(client.definition(&sysmsg, 2, 13).is_none());

    // Abend of step COBRUN.GO: the expanded `XXGO EXEC` (statement 46).
    let location = client.definition(&sysmsg, 130, 5).unwrap();
    assert_eq!((location.uri, location.range.start.line), (jcl.clone(), 73));

    // Same step from the JESMSGLG step table.
    let location = client.definition(&msglg, 12, 25).unwrap();
    assert_eq!((location.uri, location.range.start.line), (jcl, 73));

    // Hover on the step names of the abend line: the step outcome and its
    // EXEC statement.
    let hover_at = |client: &mut Client, line, character| {
        client.request(
            HoverRequest::METHOD,
            HoverParams {
                text_document_position_params: TextDocumentPositionParams::new(
                    TextDocumentIdentifier::new(sysmsg.clone()),
                    Position::new(line, character),
                ),
                work_done_progress_params: Default::default(),
            },
        )
    };
    let hover = hover_at(&mut client, 130, 17);
    let text = hover["contents"]["value"].as_str().unwrap();
    assert!(
        text.starts_with("**Step COBRUN.GO** · abend S0C7"),
        "{text}"
    );
    assert!(text.contains("EXEC statement 46:"), "{text}");
    assert!(text.contains("XXGO "), "{text}");

    // On the message ID: its catalog entry and a link to IBM Docs.
    let hover = hover_at(&mut client, 130, 5);
    let text = hover["contents"]["value"].as_str().unwrap();
    assert!(text.starts_with("**IEF472I** · Step abended"), "{text}");
    assert!(
        text.contains("https://www.ibm.com/docs/en/zos/3.2.0?topic=messages-ief472i")
            && text.contains("https://www.ibm.com/docs/en/search/IEF472I"),
        "{text}"
    );

    // On the abend code: the abend entry.
    let line = corpus("JOB00896/JES2/JESYSMSG.txt")
        .lines()
        .nth(130)
        .unwrap()
        .to_owned();
    let column = u32::try_from(line.find("SYSTEM=0C7").unwrap() + 8).unwrap();
    let hover = hover_at(&mut client, 130, column);
    let text = hover["contents"]["value"].as_str().unwrap();
    assert!(
        text.starts_with("**Abend S0C7** · Data exception"),
        "{text}"
    );

    // Nothing on the column header line.
    assert!(hover_at(&mut client, 0, 3).is_null());

    client.stop(handle);
}

#[test]
fn definition_reads_downloaded_jcl() {
    let (mut client, handle) = Client::start();
    // Downloaded job: JESJCL is not open, the server reads it from disk.
    let sysmsg = corpus_uri("JOB00896/JES2/JESYSMSG.txt");
    client.open(&sysmsg, corpus("JOB00896/JES2/JESYSMSG.txt"));

    let location = client.definition(&sysmsg, 130, 5).unwrap();
    assert!(location.uri.as_str().ends_with("/JOB00896/JES2/JESJCL.txt"));
    assert_eq!(location.range.start.line, 73);

    client.stop(handle);
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

#[test]
fn house_rules_are_applied_and_reloaded() {
    let dir = std::env::temp_dir().join(format!("jes-reader-rules-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let rules = dir.join("team.toml");
    std::fs::write(
        &rules,
        "[[messages]]\nid = \"IDI*\"\nseverity = \"ignore\"\n",
    )
    .unwrap();

    let (client, handle) = Client::start_with(InitializeParams {
        initialization_options: Some(serde_json::json!({
            "rulesFile": rules.to_string_lossy(),
        })),
        ..InitializeParams::default()
    });
    let uri: Uri = "zowe-jobs:/zosmf/JOB00901/JRS013.JOB00901.JES2.JESYSMSG.4"
        .parse()
        .unwrap();
    client.notify(
        DidOpenTextDocument::METHOD,
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem::new(
                uri.clone(),
                "jes-sysout".into(),
                1,
                corpus("JOB00901/JES2/JESYSMSG.txt"),
            ),
        },
    );
    let codes = |p: PublishDiagnosticsParams| -> Vec<String> {
        p.diagnostics
            .into_iter()
            .filter_map(|d| match d.code {
                Some(NumberOrString::String(code)) => Some(code),
                _ => None,
            })
            .collect()
    };
    let first = codes(client.diagnostics());
    assert!(first.contains(&"S013".to_owned()), "{first:?}");
    assert!(!first.contains(&"IDI0078E".to_owned()), "{first:?}");

    // The rules file changes: the open document is analysed again.
    std::fs::write(&rules, "max_return_code = 4\n").unwrap();
    client.notify(
        DidChangeWatchedFiles::METHOD,
        serde_json::json!({ "changes": [] }),
    );
    let second = codes(client.diagnostics());
    assert!(second.contains(&"IDI0078E".to_owned()), "{second:?}");

    client.stop(handle);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn invalid_house_rules_are_reported() {
    let dir = std::env::temp_dir().join(format!("jes-reader-bad-rules-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let rules = dir.join("bad.toml");
    std::fs::write(&rules, "max_rc = 4\n").unwrap();

    let (server, connection) = Connection::memory();
    let handle = std::thread::spawn(move || jes_reader_lsp::run(&server));
    connection
        .sender
        .send(
            Request::new(
                RequestId::from(1),
                "initialize".into(),
                InitializeParams {
                    initialization_options: Some(serde_json::json!({
                        "rulesFile": rules.to_string_lossy(),
                    })),
                    ..InitializeParams::default()
                },
            )
            .into(),
        )
        .unwrap();
    assert!(matches!(next(&connection.receiver), Message::Response(_)));
    connection
        .sender
        .send(Notification::new("initialized".into(), serde_json::json!({})).into())
        .unwrap();
    match next(&connection.receiver) {
        Message::Notification(n) if n.method == "window/showMessage" => {
            let message = n.params["message"].as_str().unwrap().to_owned();
            assert!(message.contains("max_rc"), "{message}");
        }
        other => panic!("expected window/showMessage, got {other:?}"),
    }
    let client = Client {
        connection,
        next_id: 1,
    };
    client.stop(handle);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn code_lenses_per_step() {
    let (mut client, handle) = Client::start();
    // Downloaded job: programs come from `JESJCL.txt` next to the file.
    let sysmsg = corpus_uri("JOB00885/JES2/JESYSMSG.txt");
    client.open(&sysmsg, corpus("JOB00885/JES2/JESYSMSG.txt"));
    let lenses = client.request(
        "textDocument/codeLens",
        serde_json::json!({ "textDocument": { "uri": sysmsg.as_str() } }),
    );
    let titles: Vec<&str> = lenses
        .as_array()
        .unwrap()
        .iter()
        .map(|lens| lens["command"]["title"].as_str().unwrap())
        .collect();
    assert_eq!(
        titles,
        [
            "COBRUN.COBOL · IGYCRCTL · RC 0 · CPU 0.04 s · elapsed ~1 min",
            "COBRUN.LKED · IEWBLINK · RC 0 · CPU 0.01 s · elapsed < 1 min",
            "COBRUN.GO · *.LKED.SYSLMOD · abend U4038 · CPU 0.00 s · elapsed < 1 min",
        ]
    );
    // Above the first line of the step.
    assert_eq!(lenses[0]["range"]["start"]["line"], 7);

    // No lens in JESMSGLG: its step table already has the figures.
    let msglg = corpus_uri("JOB00885/JES2/JESMSGLG.txt");
    client.open(&msglg, corpus("JOB00885/JES2/JESMSGLG.txt"));
    let lenses = client.request(
        "textDocument/codeLens",
        serde_json::json!({ "textDocument": { "uri": msglg.as_str() } }),
    );
    assert_eq!(lenses, serde_json::json!([]));

    client.stop(handle);
}

#[test]
fn program_rules_wait_for_the_jcl() {
    let dir = std::env::temp_dir().join(format!("jes-reader-pgm-rules-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let rules = dir.join("team.toml");
    std::fs::write(
        &rules,
        "[[steps]]\nprogram = \"IDCAMS\"\nmax_return_code = 8\n",
    )
    .unwrap();
    let (client, handle) = Client::start_with(InitializeParams {
        initialization_options: Some(serde_json::json!({
            "rulesFile": rules.to_string_lossy(),
        })),
        ..InitializeParams::default()
    });
    let open = |uri: &Uri, path: &str| {
        client.notify(
            DidOpenTextDocument::METHOD,
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem::new(
                    uri.clone(),
                    "jes-sysout".into(),
                    1,
                    corpus(path),
                ),
            },
        );
    };
    let codes = |p: &PublishDiagnosticsParams| -> Vec<String> {
        p.diagnostics
            .iter()
            .filter_map(|d| match &d.code {
                Some(NumberOrString::String(code)) => Some(code.clone()),
                _ => None,
            })
            .collect()
    };
    let sysmsg: Uri = "zowe-jobs:/zosmf/JOB00868/JRIDC8.JOB00868.JES2.JESYSMSG.4"
        .parse()
        .unwrap();
    let jcl: Uri = "zowe-jobs:/zosmf/JOB00868/JRIDC8.JOB00868.JES2.JESJCL.3"
        .parse()
        .unwrap();

    // Without JESJCL the program is unknown: RC 8 of STEP010 is reported.
    open(&sysmsg, "JOB00868/JES2/JESYSMSG.txt");
    assert!(codes(&client.diagnostics()).contains(&"RC".to_owned()));

    // JESJCL arrives (opened in the background): STEP010 runs IDCAMS, RC 8
    // is tolerated, and JESYSMSG is published again.
    open(&jcl, "JOB00868/JES2/JESJCL.txt");
    let published = client.diagnostics();
    assert_eq!(published.uri, jcl);
    let published = client.diagnostics();
    assert_eq!(published.uri, sysmsg);
    assert!(!codes(&published).contains(&"RC".to_owned()));

    client.stop(handle);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn whole_spool() {
    let (mut client, handle) = Client::start();
    // JOB00896 saved as one file: its DDs one after the other.
    let mut text = String::new();
    let mut starts = Vec::new();
    for dd in [
        "JES2/JESMSGLG.txt",
        "JES2/JESJCL.txt",
        "JES2/JESYSMSG.txt",
        "GO/COBRUN/SYSOUT.txt",
    ] {
        starts.push(u32::try_from(text.lines().count()).unwrap());
        text.push_str(&corpus(&format!("JOB00896/{dd}")));
    }
    let uri: Uri = "untitled:JRS0C7-spool.txt".parse().unwrap();
    client.notify(
        DidOpenTextDocument::METHOD,
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem::new(uri.clone(), "jes-sysout".into(), 1, text),
        },
    );
    let (jcl, sysmsg) = (starts[1], starts[2]);

    // The abend of COBRUN.GO, on its line in the spool.
    let diagnostics = client.diagnostics().diagnostics;
    let abend = diagnostics
        .iter()
        .find(|d| {
            d.code == Some(NumberOrString::String("S0C7".into())) && d.range.start.line >= sysmsg
        })
        .expect("S0C7 in JESYSMSG");
    assert_eq!(abend.range.start.line, sysmsg + 130);

    // Outline: one entry per DD.
    let symbols = client.request(
        DocumentSymbolRequest::METHOD,
        DocumentSymbolParams {
            text_document: TextDocumentIdentifier::new(uri.clone()),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
    );
    let names: Vec<&str> = symbols
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["JESMSGLG", "JESJCL", "JESYSMSG", "Step output"]);

    // Go to Definition stays in the spool: the expanded `XXGO EXEC`.
    let location = client.definition(&uri, sysmsg + 130, 5).unwrap();
    assert_eq!(
        (location.uri, location.range.start.line),
        (uri.clone(), jcl + 73)
    );

    client.stop(handle);
}

/// A request the server cannot handle gets an error, and the server keeps
/// running: it used to stop, and the client restarted it, losing the
/// analysis of the open documents meanwhile.
#[test]
fn bad_request_does_not_stop_the_server() {
    let (mut client, handle) = Client::start();
    let uri = corpus_uri("JOB00896/JES2/JESYSMSG.txt");
    client.open(&uri, corpus("JOB00896/JES2/JESYSMSG.txt"));

    client.next_id += 1;
    let id = RequestId::from(client.next_id);
    client
        .connection
        .sender
        .send(
            Request::new(
                id.clone(),
                HoverRequest::METHOD.into(),
                serde_json::json!({}),
            )
            .into(),
        )
        .unwrap();
    match next(&client.connection.receiver) {
        Message::Response(resp) if resp.id == id => assert!(resp.response_result.is_err()),
        other => panic!("expected an error response, got {other:?}"),
    }
    // A bad notification is skipped too.
    client.notify(
        DidOpenTextDocument::METHOD,
        serde_json::json!({ "bad": true }),
    );

    // Still running, with the document still analysed.
    let symbols = client.request(
        DocumentSymbolRequest::METHOD,
        DocumentSymbolParams {
            text_document: TextDocumentIdentifier::new(uri),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
    );
    assert_eq!(symbols.as_array().map(Vec::len), Some(3));
    client.stop(handle);
}

/// Next message from the server, without its log lines (`window/logMessage`).
fn next(receiver: &crossbeam_channel::Receiver<Message>) -> Message {
    loop {
        match receiver.recv().unwrap() {
            Message::Notification(n) if n.method == "window/logMessage" => {}
            message => return message,
        }
    }
}

/// The server logs its start, its house rules and each analysis (file,
/// lines, DDs with their lines and column offset, diagnostics, time).
#[test]
fn logs_for_profiling() {
    let (server, connection) = Connection::memory();
    let handle = std::thread::spawn(move || jes_reader_lsp::run(&server));
    let mut logs = Vec::new();
    let mut log_until = |connection: &Connection, method: &str, logs: &mut Vec<String>| loop {
        match connection.receiver.recv().unwrap() {
            Message::Notification(n) if n.method == "window/logMessage" => {
                logs.push(n.params["message"].as_str().unwrap().to_owned());
            }
            Message::Notification(n) if n.method == method => return,
            Message::Response(_) if method == "response" => return,
            _ => {}
        }
    };
    connection
        .sender
        .send(
            Request::new(
                RequestId::from(1),
                "initialize".into(),
                InitializeParams::default(),
            )
            .into(),
        )
        .unwrap();
    log_until(&connection, "response", &mut logs);
    connection
        .sender
        .send(Notification::new("initialized".into(), serde_json::json!({})).into())
        .unwrap();

    // JOB00896 saved as one file, JESJCL one column to the right.
    let mut text = corpus("JOB00896/JES2/JESMSGLG.txt");
    let msglg_lines = text.lines().count();
    let jcl: String = corpus("JOB00896/JES2/JESJCL.txt")
        .lines()
        .map(|l| format!(" {l}\n"))
        .collect();
    text.push_str(&jcl);
    text.push_str(&corpus("JOB00896/JES2/JESYSMSG.txt"));
    connection
        .sender
        .send(
            Notification::new(
                DidOpenTextDocument::METHOD.into(),
                DidOpenTextDocumentParams {
                    text_document: TextDocumentItem::new(
                        "file:///tmp/JRS0C7_JOB00896.jes".parse().unwrap(),
                        "jes-sysout".into(),
                        1,
                        text,
                    ),
                },
            )
            .into(),
        )
        .unwrap();
    log_until(&connection, PublishDiagnostics::METHOD, &mut logs);

    assert!(
        logs[0].starts_with("jes-reader-lsp ") && logs[0].contains(" started"),
        "{logs:?}"
    );
    assert!(
        logs.iter().any(|l| l.starts_with("house rules: ")),
        "{logs:?}"
    );
    let analysed = logs
        .iter()
        .find(|l| l.starts_with("analysed JRS0C7_JOB00896.jes: "))
        .unwrap();
    assert!(
        analysed.contains(&format!(
            "JESMSGLG L1-{msglg_lines}, JESJCL L{}-",
            msglg_lines + 1
        )),
        "{analysed}"
    );
    assert!(
        analysed.contains("(+1 column)") && analysed.contains(" ms"),
        "{analysed}"
    );

    connection
        .sender
        .send(Request::new(RequestId::from(2), "shutdown".into(), ()).into())
        .unwrap();
    log_until(&connection, "response", &mut logs);
    connection
        .sender
        .send(Notification::new("exit".into(), ()).into())
        .unwrap();
    handle.join().unwrap().unwrap();
}
