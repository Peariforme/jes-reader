//! Serveur LSP pour sysouts JES2, construit sur `lsp-server` (synchrone).

use anyhow::Result;
use lsp_server::{Connection, Message};
use lsp_types::{ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind};

/// Capacités annoncées au client.
pub fn capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        ..ServerCapabilities::default()
    }
}

/// Fait la poignée de main `initialize`, puis traite les messages jusqu'à `shutdown`.
pub fn run(connection: &Connection) -> Result<()> {
    let caps = serde_json::to_value(capabilities())?;
    let _params = connection.initialize(caps)?;

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                // Les requêtes (outline, hovers…) arrivent en phase 1.
            }
            Message::Notification(_) | Message::Response(_) => {}
        }
    }
    Ok(())
}
