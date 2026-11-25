//! Go to Definition: from JESYSMSG and JESMSGLG to the expanded JCL in
//! JESJCL.
//!
//! - On the statement number of a conversion message
//!   (`        11 IEF655I …`): the JCL statement with that number.
//! - On the line that gives a step's outcome (`IEF142I`, `IEF472I`, a row
//!   of the JESMSGLG step table…): the EXEC statement of that step.

use std::path::{Path, PathBuf};

use jes_reader_core::{Dd, DdKind, Statement};

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
