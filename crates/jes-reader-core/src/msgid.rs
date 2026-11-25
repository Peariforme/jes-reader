//! Recognition of IBM message IDs.
//!
//! General form: 3 to 5 component letters, 3 to 5 digits, one type letter
//! (`IEF142I`, `IEFA107I`, `IGD17101I`, `IGZ0035S`), or `$HASP`
//! followed by 3 or 4 digits. The parser only looks for an ID where the DD
//! expects one, which limits false positives (`STEP010`, `IGG0191B` and
//! `IEFBR14` are rejected).

use std::ops::Range;

/// Accepted type letters: information, warning, error, severe error, action,
/// decision, terminating.
const TYPE_LETTERS: &[u8] = b"IWESADT";

/// True if `token` has the shape of a message ID.
pub fn is_message_id(token: &str) -> bool {
    let b = token.as_bytes();
    if let Some(digits) = b.strip_prefix(b"$HASP") {
        return (3..=4).contains(&digits.len()) && digits.iter().all(u8::is_ascii_digit);
    }
    let letters = b.iter().take_while(|c| c.is_ascii_uppercase()).count();
    let digits = b[letters..]
        .iter()
        .take_while(|c| c.is_ascii_digit())
        .count();
    (3..=5).contains(&letters)
        && (3..=5).contains(&digits)
        && b.len() == letters + digits + 1
        && TYPE_LETTERS.contains(&b[b.len() - 1])
}

/// Columns of a message ID that starts exactly at `start` and ends with a
/// space or the end of the line.
pub fn message_id_at(line: &str, start: usize) -> Option<Range<usize>> {
    let rest = line.get(start..)?;
    let len = rest.find(' ').unwrap_or(rest.len());
    is_message_id(&rest[..len]).then_some(start..start + len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_ids() {
        for id in [
            "IEF142I",
            "IEFA107I",
            "IEFC001I",
            "IGD17101I",
            "ICH70001I",
            "IRR010I",
            "CSV003I",
            "IEC141I",
            "IGZ0035S",
            "IDC3012I",
            "CEE3207S",
            "DSNT408I",
            "$HASP373",
            "$HASP0395",
        ] {
            assert!(is_message_id(id), "{id}");
        }
    }

    #[test]
    fn rejected_ids() {
        for token in [
            "STEP010",
            "IGG0191B",
            "IEFBR14",
            "JOB00823",
            "IEF142",
            "IE142I",
            "IEFABC12I",
            "$HASP",
            "$HASP12",
            "READY",
            "",
        ] {
            assert!(!is_message_id(token), "{token}");
        }
    }

    #[test]
    fn id_position() {
        let line = " IEF142I JRS0C7 COBOL COBRUN - STEP WAS EXECUTED";
        assert_eq!(message_id_at(line, 1), Some(1..8));
        assert_eq!(message_id_at(line, 0), None);
        assert_eq!(message_id_at(line, 9), None);
        assert_eq!(message_id_at(" IGZ0035S", 1), Some(1..9));
        assert_eq!(message_id_at("short", 40), None);
    }
}
