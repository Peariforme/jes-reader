//! Rules that turn a parsed DD into findings: what the language server
//! shows in Problems and what the CLI reports.
//!
//! Severity cannot be read from the message suffix alone: `IEF453I`
//! (JCL ERROR) and `IEFA107I` (data set not found) end in `I`. Rules
//! therefore look at the steps first, then at the catalog
//! (`catalog.toml`), and only then at the suffix (`E`, `S`, `C`, `W`).

use std::ops::Range;

use crate::catalog::{CatalogSeverity, catalog, leading_abend};
use crate::model::{Dd, Message, StepStatus};
use crate::rules::{RuleSeverity, Rules};

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
    /// Other lines of the same DD that explain this finding: the messages
    /// that caused a step's abend, or the step a message made fail.
    pub related: Vec<Related>,
}

/// Line related to a finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Related {
    /// Line in the same DD.
    pub line: usize,
    /// Why it is related.
    pub message: String,
}

/// Rule settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// A return code strictly above this threshold is a warning (editor or
    /// command-line setting; a rules file can override it).
    pub max_return_code: u16,
    /// House rules (`.jesreader.toml`).
    pub rules: Rules,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            max_return_code: 4,
            rules: Rules::default(),
        }
    }
}

/// Findings of a DD, most severe first, then by line.
pub fn analyze(dd: &Dd, options: &Options) -> Vec<Finding> {
    let mut findings = Vec::new();
    // (finding index, lines of its step) for steps that failed or did not run.
    let mut step_findings = Vec::new();
    let rules = &options.rules;

    for step in &dd.steps {
        let name = step.display_name();
        let rule = rules.step(&step.name, &name);
        let max_return_code = rule
            .and_then(|r| r.max_return_code)
            .or(rules.max_return_code)
            .unwrap_or(options.max_return_code);
        let (severity, code, mut message) = match &step.status {
            StepStatus::Abended { code, reason } => (
                FindingSeverity::Error,
                code.clone(),
                format!(
                    "Step {name}: abend {}",
                    describe_abend(code, reason.as_deref())
                ),
            ),
            StepStatus::Completed { return_code } if *return_code > max_return_code => (
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
        if let Some(note) = rule.and_then(|r| r.note.as_deref()) {
            message.push_str(&format!(" ({note})"));
        }
        if severity != FindingSeverity::Warning {
            step_findings.push((findings.len(), step.lines.clone()));
        }
        findings.push(Finding {
            severity,
            code,
            message,
            line: step.status_line,
            columns: None,
            related: Vec::new(),
        });
    }

    for m in &dd.messages {
        let rule = rules.message(&m.id, &m.text);
        let severity = match rule.and_then(|r| r.severity) {
            Some(RuleSeverity::Error) => Some(FindingSeverity::Error),
            Some(RuleSeverity::Warning) => Some(FindingSeverity::Warning),
            Some(RuleSeverity::Info) => Some(FindingSeverity::Information),
            Some(RuleSeverity::Ignore) => None,
            None => message_severity(m, dd),
        };
        let Some(severity) = severity else {
            continue;
        };
        let mut message = describe_message(m);
        if let Some(note) = rule.and_then(|r| r.note.as_deref()) {
            message.push_str(&format!(" ({note})"));
        }
        findings.push(Finding {
            severity,
            code: m.id.clone(),
            message,
            line: m.lines.start,
            columns: Some(m.id_columns.clone()),
            related: Vec::new(),
        });
    }

    relate_causes(&mut findings, &step_findings);
    findings.sort_by_key(|f| (f.severity, f.line));
    findings
}

/// Links each failed or skipped step to the error messages inside it, both
/// ways: the step points to its causes, each cause to the step. Only
/// messages the catalog knows count as causes: an unknown error message in
/// the same step (Fault Analyzer failing to write its history file) is not
/// why the step failed.
fn relate_causes(findings: &mut [Finding], step_findings: &[(usize, Range<usize>)]) {
    for (step_index, lines) in step_findings {
        let causes: Vec<usize> = findings
            .iter()
            .enumerate()
            .filter(|(i, f)| {
                i != step_index
                    && f.severity == FindingSeverity::Error
                    && f.columns.is_some()
                    && lines.contains(&f.line)
                    && catalog().message(&f.code).is_some()
            })
            .map(|(i, _)| i)
            .collect();
        for &cause in &causes {
            let (step_line, step_message) = {
                let step = &findings[*step_index];
                (step.line, step.message.clone())
            };
            let related = Related {
                line: findings[cause].line,
                message: format!("Cause: {}", findings[cause].message),
            };
            findings[*step_index].related.push(related);
            findings[cause].related.push(Related {
                line: step_line,
                message: step_message,
            });
        }
    }
}

/// `S013 (OPEN error), reason 18: member not found in the partitioned data set`.
fn describe_abend(code: &str, reason: Option<&str>) -> String {
    let entry = catalog().abend(code);
    let mut out = match entry {
        Some(entry) => format!("{code} ({})", entry.title),
        None => code.to_owned(),
    };
    if let Some(reason) = reason {
        match entry.and_then(|e| e.reason(reason)) {
            Some(meaning) => out.push_str(&format!(", reason {reason}: {meaning}")),
            None => out.push_str(&format!(", reason {reason}")),
        }
    }
    out
}

/// Catalog title, then the meaning of a leading abend reason, then the IBM
/// text: `OPEN failed (S013-18: member not found …): 013-18,IGG0191B,…`.
fn describe_message(m: &Message) -> String {
    let Some(entry) = catalog().message(&m.id) else {
        return m.text.clone();
    };
    let reason = leading_abend(&m.text).and_then(|(code, reason)| {
        let meaning = catalog().abend(&code)?.reason(reason)?;
        Some(format!(" ({code}-{reason}: {meaning})"))
    });
    format!("{}{}: {}", entry.title, reason.unwrap_or_default(), m.text)
}

fn message_severity(m: &Message, dd: &Dd) -> Option<FindingSeverity> {
    let id = m.id.as_str();
    // `IEF450I` (abend, JESMSGLG) duplicates the step table row; it only
    // counts when the installation does not print that table.
    if id == "IEF450I" {
        return dd.steps.is_empty().then_some(FindingSeverity::Error);
    }
    if let Some(severity) = catalog().message(id).and_then(|e| e.severity) {
        return match severity {
            CatalogSeverity::Error => Some(FindingSeverity::Error),
            CatalogSeverity::Warning => Some(FindingSeverity::Warning),
            CatalogSeverity::Info => None,
        };
    }
    // JCL converter message (`IEF…`, `IEFC…`) attached to a statement: the
    // JCL was corrected or completed. Other numbered messages (`IGD01007I`
    // from ACS routines) follow the general rule.
    if m.statement.is_some() && id.starts_with("IEF") {
        return Some(FindingSeverity::Warning);
    }
    match id.as_bytes().last() {
        Some(b'E' | b'S' | b'C') if !id.starts_with('$') => Some(FindingSeverity::Error),
        Some(b'W') => Some(FindingSeverity::Warning),
        _ => None,
    }
}
