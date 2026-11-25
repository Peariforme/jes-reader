//! Data model of a sysout: Job → DD → step → message.
//!
//! Positions: line numbers are 0-based (exclusive upper bounds) and
//! columns are byte offsets in the line, ASA character included. Sysout
//! text is ASCII in practice; the conversion to LSP UTF-16 columns
//! happens in the server.

use std::ops::Range;

/// Severity of an IBM message, from the last character of its ID
/// (`IEF142I`, `IEC161W`, `IGD17101E`…).
///
/// Careful: many JES2 and MVS error messages end in `I` (`IEF453I` JCL
/// ERROR, `IEF472I` abend). The actual severity of a message is decided
/// by [`crate::analysis`], not by this suffix alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// `I`: information.
    Info,
    /// `W`: warning.
    Warning,
    /// `E`: error.
    Error,
    /// `S`: severe error.
    Severe,
    /// `A`: operator action required.
    Action,
}

impl Severity {
    /// Severity from the suffix of a message ID.
    ///
    /// Returns `None` if the last character is not a known severity letter
    /// (for example `$HASP373`, which has no suffix).
    pub fn from_message_id(id: &str) -> Option<Self> {
        match id.chars().last()? {
            'I' => Some(Self::Info),
            'W' => Some(Self::Warning),
            'E' => Some(Self::Error),
            'S' => Some(Self::Severe),
            'A' => Some(Self::Action),
            _ => None,
        }
    }
}

/// Kind of DD, which decides how it is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DdKind {
    /// Job log (`JESMSGLG`): timestamped messages, step table.
    JesMsgLg,
    /// Expanded JCL (`JESJCL`): numbered statements.
    JesJcl,
    /// System messages (`JESYSMSG`): allocations, step ends.
    JesYsMsg,
    /// Any other DD (`SYSPRINT`, `SYSOUT`, `CEEDUMP`…).
    Other,
}

/// Message recognized in a DD.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Message ID (`IEF142I`).
    pub id: String,
    /// Severity from the suffix, if the ID has one.
    pub severity: Option<Severity>,
    /// Lines covered, continuation lines included.
    pub lines: Range<usize>,
    /// Columns of the ID in the first line.
    pub id_columns: Range<usize>,
    /// Text of the first line after the ID, trimmed.
    pub text: String,
    /// JCL statement number (`STMT NO.` column of JESYSMSG).
    pub statement: Option<u32>,
}

/// Outcome of a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepStatus {
    /// Ran to the end with this return code.
    Completed {
        /// Return code (`COND CODE`).
        return_code: u16,
    },
    /// Abnormal end.
    Abended {
        /// Abend code: `S0C7` (system) or `U4038` (user).
        code: String,
        /// Reason code, if given.
        reason: Option<String>,
    },
    /// Did not run (condition codes, JCL ERROR, previous step abended).
    NotRun,
    /// Unknown outcome: step still running or end messages missing.
    Unknown,
}

/// Job step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// Job step name (`COBRUN`, `STEP010`).
    pub name: String,
    /// Step name inside the called procedure (`COBOL`, `GO`), if any.
    pub proc_step: Option<String>,
    /// Outcome of the step.
    pub status: StepStatus,
    /// Lines covered in the DD.
    pub lines: Range<usize>,
    /// Line that gives the outcome (`IEF142I`, `IEF472I`, JESMSGLG step table
    /// row…), otherwise the first line of the step.
    pub status_line: usize,
}

impl Step {
    /// Display name: `COBRUN.COBOL` for a procedure step, otherwise `STEP010`.
    pub fn display_name(&self) -> String {
        match &self.proc_step {
            Some(proc_step) => format!("{}.{}", self.name, proc_step),
            None => self.name.clone(),
        }
    }
}

/// Origin of a JESJCL statement, from its first two characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementOrigin {
    /// `//`: written in the job.
    Job,
    /// `XX` or `++`: from a procedure (cataloged or in-stream).
    Procedure,
    /// `X/` or `+/`: procedure statement overridden by the job.
    Override,
}

/// Numbered JCL statement of JESJCL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    /// Statement number (target of links from JESYSMSG).
    pub number: u32,
    /// Origin of the statement.
    pub origin: StatementOrigin,
    /// Name field (label): job, step or DD name.
    pub name: Option<String>,
    /// Operation: `JOB`, `EXEC`, `DD`, `PROC`, `SET`…
    pub operation: Option<String>,
    /// Lines covered, continuations and following comments included.
    pub lines: Range<usize>,
}

/// End of job, read from `$HASP395` and `IEF453I` (JESMSGLG).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobEnd {
    /// Ended with this highest return code.
    ReturnCode(u16),
    /// Ended with an abend.
    Abend(String),
    /// Stopped with a JCL ERROR.
    JclError,
    /// Ended, no details.
    Ended,
}

/// Output DD (JESMSGLG, JESJCL, JESYSMSG, SYSPRINT…).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dd {
    /// DD name, if known (file name, URI or content).
    pub name: Option<String>,
    /// Kind of DD.
    pub kind: DdKind,
    /// Job name, if the DD shows it.
    pub job_name: Option<String>,
    /// JES job ID (`JOB00823`, `TSU00812`), from the path or the content.
    pub job_id: Option<String>,
    /// True if column 1 holds an ASA carriage-control character.
    pub has_asa: bool,
    /// Number of lines in the DD.
    pub line_count: usize,
    /// Recognized messages, in text order.
    pub messages: Vec<Message>,
    /// Steps (JESYSMSG, and the JESMSGLG step table).
    pub steps: Vec<Step>,
    /// JCL statements (JESJCL).
    pub statements: Vec<Statement>,
    /// End of job (JESMSGLG).
    pub job_end: Option<JobEnd>,
}

impl Dd {
    /// JCL statement with this number (JESJCL).
    pub fn statement(&self, number: u32) -> Option<&Statement> {
        self.statements.iter().find(|st| st.number == number)
    }

    /// EXEC statement that runs `step` (JESJCL).
    ///
    /// A plain step is the job-level EXEC with its name. A procedure step
    /// (`COBRUN.GO`) is the EXEC named `GO` expanded under the job-level
    /// EXEC `COBRUN`; if the procedure is not expanded, the job-level EXEC.
    pub fn exec_statement(&self, step: &Step) -> Option<&Statement> {
        let is_exec = |st: &&Statement| st.operation.as_deref() == Some("EXEC");
        let job_exec = self.statements.iter().position(|st| {
            is_exec(&st)
                && st.origin == StatementOrigin::Job
                && st.name.as_deref() == Some(step.name.as_str())
        })?;
        let Some(proc_step) = step.proc_step.as_deref() else {
            return Some(&self.statements[job_exec]);
        };
        let expanded = self.statements[job_exec + 1..]
            .iter()
            .filter(is_exec)
            .take_while(|st| st.origin != StatementOrigin::Job)
            .find(|st| st.name.as_deref() == Some(proc_step));
        Some(expanded.unwrap_or(&self.statements[job_exec]))
    }
}

/// Whole sysout of a job: aggregate of the open DDs with the same JES job ID.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Job {
    /// Job name.
    pub name: Option<String>,
    /// JES job ID (`JOB01234`).
    pub id: Option<String>,
    /// DDs in spool order.
    pub dds: Vec<Dd>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_from_suffix() {
        assert_eq!(Severity::from_message_id("IEF142I"), Some(Severity::Info));
        assert_eq!(
            Severity::from_message_id("IEC161W"),
            Some(Severity::Warning)
        );
        assert_eq!(
            Severity::from_message_id("IGD17101E"),
            Some(Severity::Error)
        );
        assert_eq!(Severity::from_message_id("IEF450S"), Some(Severity::Severe));
        assert_eq!(Severity::from_message_id("IEA995A"), Some(Severity::Action));
    }

    #[test]
    fn severity_absent() {
        assert_eq!(Severity::from_message_id("$HASP373"), None);
        assert_eq!(Severity::from_message_id(""), None);
    }

    #[test]
    fn step_display_name() {
        let mut step = Step {
            name: "COBRUN".into(),
            proc_step: Some("GO".into()),
            status: StepStatus::Unknown,
            lines: 0..1,
            status_line: 0,
        };
        assert_eq!(step.display_name(), "COBRUN.GO");
        step.proc_step = None;
        assert_eq!(step.display_name(), "COBRUN");
    }

    fn statement(number: u32, origin: StatementOrigin, name: &str, operation: &str) -> Statement {
        Statement {
            number,
            origin,
            name: Some(name.into()),
            operation: Some(operation.into()),
            lines: 0..1,
        }
    }

    fn step(name: &str, proc_step: Option<&str>) -> Step {
        Step {
            name: name.into(),
            proc_step: proc_step.map(Into::into),
            status: StepStatus::Unknown,
            lines: 0..1,
            status_line: 0,
        }
    }

    #[test]
    fn exec_statement_of_steps() {
        use StatementOrigin::{Job, Procedure};
        let dd = Dd {
            name: Some("JESJCL".into()),
            kind: DdKind::JesJcl,
            job_name: None,
            job_id: None,
            has_asa: true,
            line_count: 0,
            messages: Vec::new(),
            steps: Vec::new(),
            statements: vec![
                statement(1, Job, "JRS0C7", "JOB"),
                statement(2, Job, "COBRUN", "EXEC"),
                statement(3, Procedure, "IGYWCLG", "PROC"),
                statement(4, Procedure, "COBOL", "EXEC"),
                statement(46, Procedure, "GO", "EXEC"),
                statement(50, Job, "STEP2", "EXEC"),
                statement(51, Procedure, "GO", "EXEC"),
            ],
            job_end: None,
        };
        let number = |s: &Step| dd.exec_statement(s).map(|st| st.number);
        assert_eq!(number(&step("COBRUN", Some("GO"))), Some(46));
        assert_eq!(number(&step("STEP2", Some("GO"))), Some(51));
        assert_eq!(number(&step("STEP2", None)), Some(50));
        // Procedure not expanded: falls back on the job-level EXEC.
        assert_eq!(number(&step("COBRUN", Some("LKED"))), Some(2));
        assert_eq!(number(&step("MISSING", None)), None);
        assert_eq!(
            dd.statement(46).map(|st| st.name.as_deref()),
            Some(Some("GO"))
        );
        assert!(dd.statement(99).is_none());
    }
}
