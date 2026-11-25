//! Full LSP handshake over an in-memory connection.

use lsp_server::{Connection, Message, Notification, Request, RequestId};
use lsp_types::{InitializeParams, InitializeResult, TextDocumentSyncCapability};

#[test]
fn initialize_then_shutdown() {
    let (server, client) = Connection::memory();
    let handle = std::thread::spawn(move || jes_reader_lsp::run(&server));

    client
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
    let Message::Response(resp) = client.receiver.recv().unwrap() else {
        panic!("expected a response to initialize");
    };
    let result: InitializeResult = serde_json::from_value(resp.response_result.unwrap()).unwrap();
    assert!(matches!(
        result.capabilities.text_document_sync,
        Some(TextDocumentSyncCapability::Kind(_))
    ));

    client
        .sender
        .send(Notification::new("initialized".into(), serde_json::json!({})).into())
        .unwrap();
    client
        .sender
        .send(Request::new(RequestId::from(2), "shutdown".into(), ()).into())
        .unwrap();
    let Message::Response(resp) = client.receiver.recv().unwrap() else {
        panic!("expected a response to shutdown");
    };
    assert_eq!(resp.id, RequestId::from(2));
    client
        .sender
        .send(Notification::new("exit".into(), ()).into())
        .unwrap();

    handle.join().unwrap().unwrap();
}
