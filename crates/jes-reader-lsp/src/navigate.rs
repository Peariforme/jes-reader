//! Go to Definition: from JESYSMSG and JESMSGLG to the expanded JCL in
//! JESJCL.
//!
//! - On the statement number of a conversion message
//!   (`        11 IEF655I …`): the JCL statement with that number.
//! - On the line that gives a step's outcome (`IEF142I`, `IEF472I`, a row
//!   of the JESMSGLG step table…): the EXEC statement of that step.

use std::ops::Range;
use std::path::{Path, PathBuf};

use jes_reader_core::{Dd, DdKind, Statement, StepStatus};

/// What the cursor points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// A JCL statement number.
    Statement(u32),
    /// A step, by index in `Dd::steps`.
    Step(usize),
}

/// Target under the cursor at byte column `column` of line `line`.
pub fn target_at(dd: &Dd, line: usize, column: usize) -> Option<Target> {
    if !matches!(dd.kind, DdKind::JesYsMsg | DdKind::JesMsgLg) {
        return None;
    }
    let numbered = dd.messages.iter().find_map(|m| {
        let on_number = m.lines.start == line && column < m.id_columns.start;
        m.statement.filter(|_| on_number)
    });
    if let Some(number) = numbered {
        return Some(Target::Statement(number));
    }
    dd.steps
        .iter()
        .position(|s| s.status_line == line)
        .map(Target::Step)
}

/// JCL statement a target leads to, in the JESJCL `jcl` of the same job.
pub fn resolve<'a>(target: Target, from: &Dd, jcl: &'a Dd) -> Option<&'a Statement> {
    match target {
        Target::Statement(number) => jcl.statement(number),
        Target::Step(index) => jcl.exec_statement(from.steps.get(index)?),
    }
}

/// Columns of the target on its line: the statement number, or `None` for
/// a step (the whole line is the target).
pub fn target_columns(dd: &Dd, target: Target, line: usize) -> Option<Range<usize>> {
    let Target::Statement(number) = target else {
        return None;
    };
    let message = dd
        .messages
        .iter()
        .find(|m| m.lines.start == line && m.statement == Some(number))?;
    // `        11 IEF655I`: the number ends one space before the message ID.
    let end = message.id_columns.start.checked_sub(1)?;
    let start = end.checked_sub(number.to_string().len())?;
    Some(start..end)
}

/// Longest JCL excerpt shown in a hover.
const HOVER_LINES: usize = 12;

/// Markdown hover for a target: what it is and, when JESJCL is available,
/// the JCL statement it leads to.
pub fn hover_text(target: Target, from: &Dd, jcl: Option<(&Dd, &str)>) -> String {
    let mut out = match target {
        Target::Statement(number) => format!("**JCL statement {number}**"),
        Target::Step(index) => match from.steps.get(index) {
            Some(step) => format!(
                "**Step {}** · {}",
                step.display_name(),
                outcome(&step.status)
            ),
            None => String::new(),
        },
    };
    let statement = jcl.and_then(|(dd, text)| Some((resolve(target, from, dd)?, text)));
    let Some((statement, text)) = statement else {
        out.push_str("\n\nThe JESJCL of this job is not available: open it to follow the link.");
        return out;
    };
    if let Target::Step(_) = target {
        out.push_str(&format!("\n\nEXEC statement {}:", statement.number));
    }
    out.push_str("\n\n```jcl\n");
    let lines: Vec<&str> = text
        .lines()
        .skip(statement.lines.start)
        .take(statement.lines.len())
        .collect();
    for line in lines.iter().take(HOVER_LINES) {
        // Without the ASA column.
        out.push_str(line.get(1..).unwrap_or("").trim_end());
        out.push('\n');
    }
    if lines.len() > HOVER_LINES {
        out.push_str("…\n");
    }
    out.push_str("```\n\nF12 or Ctrl+click: open it in JESJCL.");
    out
}

fn outcome(status: &StepStatus) -> String {
    match status {
        StepStatus::Completed { return_code } => format!("return code {return_code}"),
        StepStatus::Abended {
            code,
            reason: Some(reason),
        } => format!("abend {code}, reason {reason}"),
        StepStatus::Abended { code, reason: None } => format!("abend {code}"),
        StepStatus::NotRun => "did not run".into(),
        StepStatus::Unknown => "outcome unknown".into(),
    }
}

/// True if `jcl` is the JESJCL of the job `dd` belongs to.
pub fn is_jcl_of(jcl: &Dd, dd: &Dd) -> bool {
    jcl.kind == DdKind::JesJcl && jcl.job_id.is_some() && jcl.job_id == dd.job_id
}

/// Local path of a `file:` URI path (`/c:/x/y` on Windows, `/x/y` elsewhere).
pub fn file_path(decoded_uri_path: &str) -> PathBuf {
    let b = decoded_uri_path.as_bytes();
    let windows_drive = b.len() >= 3 && b[0] == b'/' && b[1].is_ascii_alphabetic() && b[2] == b':';
    PathBuf::from(if windows_drive {
        &decoded_uri_path[1..]
    } else {
        decoded_uri_path
    })
}

/// JESJCL next to a downloaded JES2 DD (`…/JOB00896/JES2/JESJCL.txt`), if
/// the file exists.
pub fn sibling_jcl(path: &Path) -> Option<PathBuf> {
    let sibling = path.parent()?.join("JESJCL.txt");
    sibling.is_file().then_some(sibling)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jes_reader_core::parse_file;

    const SYSMSG: &str = "  STMT NO. MESSAGE\n\
         \x20       11 IEF655I INVALID DSNAME SPECIFIED\n\
         \x20IEF142I JRS0C7 COBOL COBRUN - STEP WAS EXECUTED - COND CODE 0000\n\
         \x20IEF373I STEP/COBOL   /START 2026329.0451\n\
         \x20IEF032I STEP/COBOL   /STOP  2026329.0451\n";

    #[test]
    fn targets() {
        let dd = parse_file("JOB00896/JES2/JESYSMSG.txt", SYSMSG);
        // On the statement number, not on the message ID.
        assert_eq!(target_at(&dd, 1, 9), Some(Target::Statement(11)));
        assert_eq!(target_at(&dd, 1, 12), None);
        // On the step status line.
        assert_eq!(target_at(&dd, 2, 20), Some(Target::Step(0)));
        assert_eq!(target_at(&dd, 3, 1), None);
    }

    #[test]
    fn hover_texts() {
        let dd = parse_file("JOB00896/JES2/JESYSMSG.txt", SYSMSG);
        let jcl_text = "        1 //JRS0C7   JOB (ACCT)\n\
                        \x20       2 //COBRUN   EXEC IGYWCLG\n\
                        \x20       4 XXCOBOL   EXEC PGM=IGYCRCTL,REGION=0M\n\
                        \x20      11 //COBOL.SYSIN DD *\n";
        let jcl = parse_file("JOB00896/JES2/JESJCL.txt", jcl_text);

        assert_eq!(target_columns(&dd, Target::Statement(11), 1), Some(8..10));
        assert_eq!(target_columns(&dd, Target::Step(0), 2), None);

        let text = hover_text(Target::Statement(11), &dd, Some((&jcl, jcl_text)));
        assert!(text.starts_with("**JCL statement 11**"), "{text}");
        assert!(text.contains("      11 //COBOL.SYSIN DD *\n"), "{text}");

        let text = hover_text(Target::Step(0), &dd, Some((&jcl, jcl_text)));
        assert!(
            text.starts_with("**Step COBRUN.COBOL** · return code 0"),
            "{text}"
        );
        assert!(text.contains("EXEC statement 4:"), "{text}");
        assert!(text.contains("XXCOBOL   EXEC PGM=IGYCRCTL"), "{text}");

        let text = hover_text(Target::Statement(11), &dd, None);
        assert!(text.contains("not available"), "{text}");
    }

    #[test]
    fn jcl_of_the_same_job() {
        let dd = parse_file("JOB00896/JES2/JESYSMSG.txt", SYSMSG);
        let jcl = parse_file("JOB00896/JES2/JESJCL.txt", "");
        let other = parse_file("JOB00893/JES2/JESJCL.txt", "");
        assert!(is_jcl_of(&jcl, &dd));
        assert!(!is_jcl_of(&other, &dd));
        assert!(!is_jcl_of(&dd, &dd));
    }

    #[test]
    fn file_paths() {
        assert_eq!(
            file_path("/c:/x/JESJCL.txt"),
            PathBuf::from("c:/x/JESJCL.txt")
        );
        assert_eq!(
            file_path("/home/x/JESJCL.txt"),
            PathBuf::from("/home/x/JESJCL.txt")
        );
    }
}
