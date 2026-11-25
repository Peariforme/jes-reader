//! Rules that turn a parsed DD into findings: what the language server
//! shows in Problems and what the CLI reports.
//!
//! Severity cannot be read from the message suffix alone: `IEF453I`
//! (JCL ERROR) and `IEFA107I` (data set not found) end in `I`. Rules
//! therefore look at the steps first, then at a list of known messages,
//! and only then at the suffix (`E`, `S`, `W`).

use std::ops::Range;

use crate::model::{Dd, Message, StepStatus};

/// Severity of a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FindingSeverity {
    /// The job or a step failed.
    Error,
    /// Worth checking: high return code, warning message.
    Warning,
    /// For information: step that did not run.
    Information,
}

/// Finding on a DD.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Severity.
    pub severity: FindingSeverity,
    /// Short code: message ID, abend code or `RC`.
    pub code: String,
    /// Displayed text.
    pub message: String,
    /// Line concerned.
    pub line: usize,
    /// Columns concerned; `None` for the whole line.
    pub columns: Option<Range<usize>>,
}

/// Rule settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// A return code strictly above this threshold is a warning.
    pub max_return_code: u16,
}

impl Default for Options {
    fn default() -> Self {
        Self { max_return_code: 4 }
    }
}

/// Messages that are errors whatever their suffix.
const ERROR_MESSAGES: &[&str] = &[
    "IEF453I",   // JOB FAILED - JCL ERROR
    "IEFA107I",  // DATA SET … NOT FOUND (allocation)
    "IEF212I",   // DATA SET NOT FOUND
    "IGD17101I", // DUPLICATE NAME EXISTS IN CATALOG
    "CSV003I",   // REQUESTED MODULE … NOT FOUND
    "IEC141I",   // 013-xx: OPEN failed
];

/// Messages that are warnings whatever their suffix.
const WARNING_MESSAGES: &[&str] = &[
    "IEF287I", // NOT CATLGD
];

/// Numbered conversion messages that are purely informational.
const INFO_STATEMENT_MESSAGES: &[&str] = &[
    "IEFC001I", // PROCEDURE … WAS EXPANDED
    "IEFC002I", // INCLUDE GROUP … WAS EXPANDED
];

/// Findings of a DD, most severe first, then by line.
pub fn analyze(dd: &Dd, options: &Options) -> Vec<Finding> {
    let mut findings = Vec::new();

    for step in &dd.steps {
        let name = step.display_name();
        let (severity, code, message) = match &step.status {
            StepStatus::Abended { code, reason } => (
                FindingSeverity::Error,
                code.clone(),
                match reason {
                    Some(reason) => format!("Step {name}: abend {code}, reason {reason}"),
                    None => format!("Step {name}: abend {code}"),
                },
            ),
            StepStatus::Completed { return_code } if *return_code > options.max_return_code => (
                FindingSeverity::Warning,
                "RC".to_owned(),
                format!("Step {name}: return code {return_code}"),
            ),
            StepStatus::NotRun => (
                FindingSeverity::Information,
                "FLUSH".to_owned(),
                format!("Step {name} did not run"),
            ),
            StepStatus::Completed { .. } | StepStatus::Unknown => continue,
        };
        findings.push(Finding {
            severity,
            code,
            message,
            line: step.status_line,
            columns: None,
        });
    }

    for m in &dd.messages {
        if let Some(severity) = message_severity(m, dd) {
            findings.push(Finding {
                severity,
                code: m.id.clone(),
                message: m.text.clone(),
                line: m.lines.start,
                columns: Some(m.id_columns.clone()),
            });
        }
    }

    findings.sort_by_key(|f| (f.severity, f.line));
    findings
}

fn message_severity(m: &Message, dd: &Dd) -> Option<FindingSeverity> {
    let id = m.id.as_str();
    if ERROR_MESSAGES.contains(&id) {
        return Some(FindingSeverity::Error);
    }
    if WARNING_MESSAGES.contains(&id) {
        return Some(FindingSeverity::Warning);
    }
    // `IEF450I` (abend, JESMSGLG) duplicates the step table row; it only
    // counts when the installation does not print that table.
    if id == "IEF450I" {
        return dd.steps.is_empty().then_some(FindingSeverity::Error);
    }
    // JCL converter message (`IEF…`, `IEFC…`) attached to a statement: the
    // JCL was corrected or completed. Other numbered messages (`IGD01007I`
    // from ACS routines) follow the general rule.
    if m.statement.is_some() && id.starts_with("IEF") && !INFO_STATEMENT_MESSAGES.contains(&id) {
        return Some(FindingSeverity::Warning);
    }
    match id.as_bytes().last() {
        Some(b'E' | b'S') if !id.starts_with('$') => Some(FindingSeverity::Error),
        Some(b'W') => Some(FindingSeverity::Warning),
        _ => None,
    }
}
