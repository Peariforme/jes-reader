//! Analysis engine for z/OS JES2 sysouts.
//!
//! The unit of analysis is the DD: Zowe Explorer opens spools one DD at a
//! time. The model follows the Job → DD → step → message hierarchy. The
//! parser never fails: whatever it does not understand stays plain text.
//!
//! ```
//! use jes_reader_core::{analyze, parse_file, Options};
//!
//! let text = " IEF472I JRS0C7 GO COBRUN - COMPLETION CODE - SYSTEM=0C7 USER=0000 REASON=00000007\n";
//! let dd = parse_file("JOB00896/JES2/JESYSMSG.txt", text);
//! let findings = analyze(&dd, &Options::default());
//! assert_eq!(findings[0].code, "S0C7");
//! ```

pub mod analysis;
pub mod catalog;
pub mod model;
pub mod msgid;
pub mod parse;
pub mod rules;
pub mod source;
pub mod spool;

pub use analysis::{Finding, FindingSeverity, Options, analyze};
pub use model::{
    Dd, DdKind, Job, JobEnd, Message, Severity, Statement, StatementOrigin, Step, StepStatus,
    duration_text,
};
pub use parse::{parse_dd, parse_file};
pub use rules::Rules;
pub use source::DdSource;
pub use spool::{parse_document, parse_spool};
