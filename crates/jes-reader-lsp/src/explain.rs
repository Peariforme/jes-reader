//! Hovers from the catalog: message IDs and abend codes, wherever they
//! appear (`IEF472I … SYSTEM=0C7`, `*S0C7` in the step table, `ABEND806-04`).

use jes_reader_core::catalog::{ZOS_RELEASE, catalog, docs_url, leading_abend, message_page_url};
use jes_reader_core::{Dd, Message};

/// Message whose ID is under byte column `column` of line `line`.
pub fn message_at(dd: &Dd, line: usize, column: usize) -> Option<&Message> {
    dd.messages
        .iter()
        .find(|m| m.lines.start == line && m.id_columns.contains(&column))
}

/// Markdown hover for a message: our summary, the meaning of a leading
/// abend reason (`013-18` in IEC141I), and links to IBM Docs: the message
/// page of the z/OS documentation when the prefix is documented there, and
/// a search as a fallback in case that page has another name.
pub fn message_hover(message: &Message) -> String {
    let id = &message.id;
    let search = docs_url(id);
    let link = match message_page_url(id) {
        Some(page) => {
            format!(
                "[{id} in the z/OS {ZOS_RELEASE} documentation]({page}) · [search IBM Docs]({search})"
            )
        }
        None => format!("[{id} in IBM Docs]({search})"),
    };
    let Some(entry) = catalog().message(id) else {
        return format!("**{id}**\n\nNot in the JES Reader catalog yet.\n\n{link}");
    };
    let mut out = format!("**{id}** · {}\n\n{}", entry.title, entry.summary);
    if let Some((code, reason)) = leading_abend(&message.text)
        && let Some(meaning) = catalog().abend(&code).and_then(|a| a.reason(reason))
    {
        out.push_str(&format!("\n\nHere: **{code}-{reason}**, {meaning}."));
    }
    if let Some(action) = &entry.action {
        out.push_str(&format!("\n\n*{action}*"));
    }
    out.push_str(&format!("\n\n{link}"));
    out
}

/// Abend code and optional reason in the word under byte column `column`.
pub fn abend_at(line: &str, column: usize) -> Option<(String, Option<String>)> {
    let is_word = |c: u8| c.is_ascii_alphanumeric() || matches!(c, b'=' | b'*' | b'-' | b'$');
    let bytes = line.as_bytes();
    if column >= bytes.len() || !is_word(bytes[column]) {
        return None;
    }
    let start = bytes[..column]
        .iter()
        .rposition(|&c| !is_word(c))
        .map_or(0, |i| i + 1);
    let end = bytes[column..]
        .iter()
        .position(|&c| !is_word(c))
        .map_or(bytes.len(), |i| column + i);
    abend_code(&line[start..end])
}

/// `S0C7`, `*S0C7`, `ABEND=S0C7`, `SYSTEM=0C7`, `USER=4038`, `U4038`,
/// `ABEND806-04`, `S806-04`. Zero codes (`SYSTEM=000`, `U0000`) are not
/// abends.
fn abend_code(word: &str) -> Option<(String, Option<String>)> {
    let word = word.trim_start_matches('*');
    let word = word.strip_prefix("ABEND=").unwrap_or(word);
    let (code, reason) = match word.split_once('-') {
        Some((code, reason)) => (code, Some(reason)),
        None => (word, None),
    };
    let hex = |s: &str| s.bytes().all(|b| b.is_ascii_hexdigit());
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    let code = if let Some(system) = code.strip_prefix("SYSTEM=") {
        format!("S{system}")
    } else if let Some(user) = code.strip_prefix("USER=") {
        format!("U{user}")
    } else if let Some(system) = code.strip_prefix("ABEND") {
        format!("S{system}")
    } else {
        code.to_owned()
    };
    let valid = match code.split_at_checked(1)? {
        ("S", rest) => rest.len() == 3 && hex(rest) && rest != "000",
        ("U", rest) => rest.len() == 4 && digits(rest) && rest != "0000",
        _ => false,
    };
    let reason = reason
        .filter(|r| !r.is_empty() && hex(r))
        .map(str::to_owned);
    valid.then_some((code, reason))
}

/// Markdown hover for an abend code, if the catalog knows it.
pub fn abend_hover(code: &str, reason: Option<&str>) -> Option<String> {
    let entry = catalog().abend(code)?;
    let mut out = format!("**Abend {code}** · {}\n\n{}", entry.title, entry.summary);
    if let Some(reason) = reason {
        match entry.reason(reason) {
            Some(meaning) => out.push_str(&format!("\n\nReason **{reason}**: {meaning}.")),
            None => out.push_str(&format!("\n\nReason **{reason}**.")),
        }
    }
    if let Some(action) = &entry.action {
        out.push_str(&format!("\n\n*{action}*"));
    }
    out.push_str(&format!("\n\n[{code} in IBM Docs]({})", docs_url(code)));
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abend_words() {
        let at = |line: &str, col| abend_at(line, col);
        let line =
            " IEF472I JRS0C7 GO COBRUN - COMPLETION CODE - SYSTEM=0C7 USER=0000 REASON=00000007";
        let system = line.find("SYSTEM").unwrap();
        assert_eq!(at(line, system + 8), Some(("S0C7".into(), None)));
        // USER=0000 is not an abend; job name JRS0C7 is not a code.
        assert_eq!(at(line, line.find("USER").unwrap()), None);
        assert_eq!(at(line, 10), None);

        assert_eq!(
            at(" 04.51.14 JOB00896  -COBRUN   GO       *S0C7    282", 41),
            Some(("S0C7".into(), None))
        );
        assert_eq!(
            at("$HASP395 JRIGZ    ENDED - ABEND=U4038", 33),
            Some(("U4038".into(), None))
        );
        assert_eq!(
            at(" CSV028I ABEND806-04  JOBNAME=JRS806", 12),
            Some(("S806".into(), Some("04".into())))
        );
        assert_eq!(at(" IEC141I 013-18,IGG0191B", 10), None);
        assert_eq!(at(" SYS S0W1", 6), None);
    }

    #[test]
    fn hovers() {
        let text = abend_hover("S806", Some("04")).unwrap();
        assert!(
            text.starts_with("**Abend S806** · Module not found"),
            "{text}"
        );
        assert!(text.contains("Reason **04**: module not found"), "{text}");
        assert!(
            text.contains("https://www.ibm.com/docs/en/search/S806"),
            "{text}"
        );
        assert!(abend_hover("S999", None).is_none());

        let dd = jes_reader_core::parse_file(
            "JOB00901/JES2/JESYSMSG.txt",
            " IEC141I 013-18,IGG0191B,JRS013,STEP010,SYSUT1,1001,VTMVSC,\n SYS1.MACLIB(JRNOMBR)\n",
        );
        let message = message_at(&dd, 0, 3).unwrap();
        let text = message_hover(message);
        assert!(text.starts_with("**IEC141I** · OPEN failed"), "{text}");
        assert!(
            text.contains("Here: **S013-18**, member not found"),
            "{text}"
        );
        assert!(message_at(&dd, 0, 12).is_none());
    }
}
