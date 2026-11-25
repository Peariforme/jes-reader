//! Application DDs (`SYSPRINT`, `SYSOUT`, `CEEDUMP`…): only IBM messages
//! at the start of a line.
//!
//! ```text
//! 0IDC3012I ENTRY JESREAD.INEXIST.DELETE NOT FOUND
//!  IGZ0035S There was an unsuccessful OPEN or CLOSE of file INFILE …
//!           Neither FILE STATUS nor an ERROR declarative were specified.
//! ```
//!
//! The ID is in column 2 after an ASA character, in column 1 otherwise.
//! Only indented lines continue a message.

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
            messages.extend(i, body);
        } else {
            messages.close();
        }
    }
    dd.messages = messages.finish();
}
