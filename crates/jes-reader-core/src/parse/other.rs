//! DD applicatives (`SYSPRINT`, `SYSOUT`, `CEEDUMP`…) : seulement les
//! messages IBM en début de ligne.
//!
//! ```text
//! 0IDC3012I ENTRY JESREAD.INEXIST.DELETE NOT FOUND
//!  IGZ0035S There was an unsuccessful OPEN or CLOSE of file INFILE …
//!           Neither FILE STATUS nor an ERROR declarative were specified.
//! ```
//!
//! L'identifiant est en col. 2 derrière un caractère ASA, en col. 1 sinon.
//! Seules les lignes indentées prolongent un message.

use super::Messages;
use crate::model::Dd;
use crate::msgid::message_id_at;

pub(super) fn parse(lines: &[&str], dd: &mut Dd) {
    let start = usize::from(dd.has_asa);
    let mut messages = Messages::new();
    for (i, line) in lines.iter().enumerate() {
        let body = line.get(start..).unwrap_or("");
        if let Some(id) = message_id_at(line, start) {
            messages.start(i, line, id, None);
        } else if body.starts_with(' ') && !body.trim().is_empty() {
            messages.extend(i);
        } else {
            messages.close();
        }
    }
    dd.messages = messages.finish();
}
