//! Parsing of one DD. The parser never fails: what it does not recognize
//! stays plain text, with no message and no step.

mod jcl;
mod msglg;
mod other;
mod sysmsg;

use std::ops::Range;
use std::time::Duration;

use crate::model::{Dd, DdKind, Message, Severity};
use crate::source::{DdSource, jes_dd_name};

/// Parses a document whose path (or URI) is known.
pub fn parse_file(path: &str, text: &str) -> Dd {
    parse_dd(&DdSource::from_path(path), text)
}

/// Parses a DD. The DD name in `source` decides how to read it; otherwise
/// the content does (JES2 headers).
pub fn parse_dd(source: &DdSource, text: &str) -> Dd {
    let by_name = source
        .dd_name
        .as_deref()
        .map_or(DdKind::Other, DdKind::from_dd_name);
    let kind = match by_name {
        DdKind::Other => DdKind::sniff(text).unwrap_or(DdKind::Other),
        kind => kind,
    };
    let original: Vec<&str> = text.lines().collect();
    // JES2 DDs saved without their ASA column (BMC AMI DevX): parse them
    // with a blank column added, then move the columns back.
    let without_asa = kind != DdKind::Other && lacks_asa(kind, &original);
    let padded: Vec<String> = if without_asa {
        original.iter().map(|l| format!(" {l}")).collect()
    } else {
        Vec::new()
    };
    let lines: Vec<&str> = if without_asa {
        padded.iter().map(String::as_str).collect()
    } else {
        original
    };
    let has_asa = match kind {
        DdKind::Other => detect_asa(&lines),
        _ => !without_asa,
    };
    let mut dd = Dd {
        name: source
            .dd_name
            .clone()
            .or_else(|| jes_dd_name(kind).map(str::to_owned)),
        kind,
        job_name: source.job_name.clone(),
        job_id: source.job_id.clone(),
        has_asa,
        first_line: 0,
        line_count: lines.len(),
        messages: Vec::new(),
        steps: Vec::new(),
        statements: Vec::new(),
        job_end: None,
    };
    match kind {
        DdKind::JesMsgLg => msglg::parse(&lines, &mut dd),
        DdKind::JesJcl => jcl::parse(&lines, &mut dd),
        DdKind::JesYsMsg => sysmsg::parse(&lines, &mut dd),
        DdKind::Other => other::parse(&lines, &mut dd),
    }
    if without_asa {
        remove_first_column(&mut dd);
    }
    dd
}

/// True if a JES2 system DD has no ASA column: its fixed fields start one
/// column to the left.
///
/// - JESMSGLG: time in column 1 (`04.12.28 JOB00823  IRR010I …`);
/// - JESJCL: `//` in column 10 after the statement number (`       1 //JOB…`);
/// - JESYSMSG: a message ID in column 1 (`IEF142I …`).
pub(crate) fn lacks_asa(kind: DdKind, lines: &[&str]) -> bool {
    let time = |l: &str| {
        let b = l.as_bytes();
        b.len() > 9
            && b[..8].iter().enumerate().all(|(k, c)| {
                if k % 3 == 2 {
                    *c == b'.'
                } else {
                    c.is_ascii_digit()
                }
            })
            && b[8] == b' '
    };
    let statement = |l: &str| {
        l.get(..8)
            .map(str::trim)
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()))
            && l.get(8..11) == Some(" //")
    };
    let message = |l: &str| crate::msgid::message_id_at(l, 0).is_some();
    let test: &dyn Fn(&str) -> bool = match kind {
        DdKind::JesMsgLg => &time,
        DdKind::JesJcl => &statement,
        DdKind::JesYsMsg => &message,
        DdKind::Other => return false,
    };
    lines.iter().take(200).any(|l| test(l))
}

/// Moves every column of `dd` one to the left: the DD was parsed with a
/// blank ASA column added in front of each line.
pub(crate) fn remove_first_column(dd: &mut Dd) {
    for m in &mut dd.messages {
        m.id_columns = m.id_columns.start.saturating_sub(1)..m.id_columns.end.saturating_sub(1);
    }
    dd.has_asa = false;
}

/// A DD has ASA characters if every non-empty line starts with one of the
/// control characters ` `, `0`, `-`, `+` or `1`.
fn detect_asa(lines: &[&str]) -> bool {
    let mut any = false;
    for line in lines.iter().filter(|l| !l.is_empty()) {
        if !matches!(line.as_bytes()[0], b' ' | b'0' | b'-' | b'+' | b'1') {
            return false;
        }
        any = true;
    }
    any
}

/// Collects the messages of a DD and handles their continuation lines.
struct Messages {
    messages: Vec<Message>,
    open: bool,
}

impl Messages {
    fn new() -> Self {
        Self {
            messages: Vec::new(),
            open: false,
        }
    }

    /// Opens a message whose ID is at `id_columns` on line `line_no`.
    fn start(
        &mut self,
        line_no: usize,
        line: &str,
        id_columns: Range<usize>,
        statement: Option<u32>,
    ) {
        let id = line[id_columns.clone()].to_owned();
        self.messages.push(Message {
            severity: Severity::from_message_id(&id),
            id,
            lines: line_no..line_no + 1,
            text: line[id_columns.end..].trim().to_owned(),
            id_columns,
            statement,
        });
        self.open = true;
    }

    /// Attaches line `line_no` to the open message, if there is one.
    /// `content` is the line without its prefix (ASA character, JESMSGLG
    /// timestamp); it is appended to the message text.
    fn extend(&mut self, line_no: usize, content: &str) {
        if self.open
            && let Some(last) = self.messages.last_mut()
        {
            last.lines.end = line_no + 1;
            let content = content.trim();
            if !content.is_empty() {
                last.text.push(' ');
                last.text.push_str(content);
            }
        }
    }

    /// Closes the current message: following lines no longer attach to it.
    fn close(&mut self) {
        self.open = false;
    }

    fn finish(self) -> Vec<Message> {
        self.messages
    }
}

/// Step names in a `<job> [<procstep>] <step> - …` message.
struct StepNames<'a> {
    job: &'a str,
    step: &'a str,
    proc_step: Option<&'a str>,
}

impl<'a> StepNames<'a> {
    /// Reads the names before the first ` - `, which must be present.
    fn parse(text: &'a str) -> Option<Self> {
        Self::from_names(text.split_once(" - ")?.0)
    }

    /// Two names: job and step; three: job, procedure step, job step.
    fn from_names(head: &'a str) -> Option<Self> {
        let names: Vec<&str> = head.split_whitespace().collect();
        match names[..] {
            // Unnamed step (`// EXEC PGM=…` without a label): job name only.
            [job] => Some(Self {
                job,
                step: "",
                proc_step: None,
            }),
            [job, step] => Some(Self {
                job,
                step,
                proc_step: None,
            }),
            [job, proc_step, step] => Some(Self {
                job,
                step,
                proc_step: Some(proc_step),
            }),
            _ => None,
        }
    }
}

/// Decimal number of minutes (`.00`, `1.25`, `.0`), as printed in the
/// JESMSGLG step table, read exactly to the hundredth of a minute.
fn minutes(text: &str) -> Option<Duration> {
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    if whole.is_empty() && fraction.is_empty() {
        return None;
    }
    let digits = |s: &str| s.bytes().all(|c| c.is_ascii_digit());
    if !digits(whole) || !digits(fraction) {
        return None;
    }
    let whole: u64 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    let hundredths: u64 = format!("{fraction:0<2}").get(..2)?.parse().ok()?;
    Some(Duration::from_millis((whole * 100 + hundredths) * 600))
}

/// Value after `key` in `text`, up to the next space.
fn value_after<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let start = text.find(key)? + key.len();
    let rest = &text[start..];
    Some(rest.split_whitespace().next().unwrap_or(""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asa() {
        assert!(detect_asa(&["1READY", " %CMD", "", "0FIN"]));
        assert!(!detect_asa(&["         MACRO", "*  comment"]));
        assert!(!detect_asa(&["", ""]));
    }

    #[test]
    fn step_names() {
        let n = StepNames::parse("JRS0C7 COBOL COBRUN - STEP WAS EXECUTED").unwrap();
        assert_eq!(
            (n.job, n.proc_step, n.step),
            ("JRS0C7", Some("COBOL"), "COBRUN")
        );
        let n = StepNames::parse("JRIDC8 STEP020 - STEP WAS NOT EXECUTED.").unwrap();
        assert_eq!((n.job, n.proc_step, n.step), ("JRIDC8", None, "STEP020"));
        assert!(StepNames::parse("NO DASH HERE").is_none());
        // Unnamed step: only the job name before ` - `.
        let n = StepNames::parse("JCLSETUP - STEP WAS EXECUTED - COND CODE 0000").unwrap();
        assert_eq!((n.job, n.proc_step, n.step), ("JCLSETUP", None, ""));
    }

    #[test]
    fn minute_values() {
        assert_eq!(minutes(".00"), Some(Duration::ZERO));
        assert_eq!(minutes(".01"), Some(Duration::from_millis(600)));
        assert_eq!(minutes(".5"), Some(Duration::from_secs(30)));
        assert_eq!(minutes("2.25"), Some(Duration::from_secs(135)));
        assert_eq!(minutes("12"), Some(Duration::from_secs(720)));
        assert_eq!(minutes("."), None);
        assert_eq!(minutes("BATCH"), None);
    }

    #[test]
    fn values() {
        let t = "COMPLETION CODE - SYSTEM=0C7 USER=0000 REASON=00000007";
        assert_eq!(value_after(t, "SYSTEM="), Some("0C7"));
        assert_eq!(value_after(t, "REASON="), Some("00000007"));
        assert_eq!(value_after(t, "ABSENT="), None);
    }

    /// DDs saved without the ASA column read like the originals, columns one
    /// to the left; JESMSGLG and JESJCL are still recognized by content.
    #[test]
    fn without_asa_column() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/sysout");
        for (path, name) in [
            ("JOB00896/JES2/JESMSGLG.txt", "document"),
            ("JOB00896/JES2/JESJCL.txt", "document"),
            ("JOB00896/JES2/JESYSMSG.txt", "JESYSMSG.txt"),
            ("JOB00889/JES2/JESYSMSG.txt", "JESYSMSG.txt"),
        ] {
            let text =
                String::from_utf8_lossy(&std::fs::read(root.join(path)).unwrap()).into_owned();
            let stripped: String = text
                .lines()
                .map(|l| format!("{}\n", l.get(1..).unwrap_or("")))
                .collect();
            let original = parse_file(path, &text);
            let dd = parse_file(name, &stripped);
            assert_eq!(dd.kind, original.kind, "{path}");
            assert!(!dd.has_asa, "{path}");
            assert_eq!(dd.steps, original.steps, "{path}");
            assert_eq!(dd.statements, original.statements, "{path}");
            assert_eq!(dd.job_end, original.job_end, "{path}");
            let ids = |d: &Dd| -> Vec<_> {
                d.messages
                    .iter()
                    .map(|m| (m.id.clone(), m.id_columns.start))
                    .collect()
            };
            let shifted: Vec<_> = ids(&original)
                .into_iter()
                .map(|(id, c)| (id, c - 1))
                .collect();
            assert_eq!(ids(&dd), shifted, "{path}");
        }
    }

    #[test]
    fn never_fails() {
        for text in ["", "\r\n", "1", " ", "\u{0}\u{1}", "é è à", "        1 //"] {
            for name in ["JESMSGLG", "JESJCL", "JESYSMSG", "SYSPRINT"] {
                let source = DdSource {
                    dd_name: Some(name.into()),
                    ..DdSource::default()
                };
                parse_dd(&source, text);
            }
        }
    }
}
