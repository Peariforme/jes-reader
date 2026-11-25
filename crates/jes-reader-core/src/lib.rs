//! Moteur d'analyse des sysouts JES2 z/OS.
//!
//! L'unité d'analyse est la DD : Zowe Explorer ouvre les spools DD par DD.
//! Le modèle suit la hiérarchie Job → DD → step → message. Le parser ne
//! plante jamais : ce qu'il ne comprend pas reste du texte brut.
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
pub mod model;
pub mod msgid;
pub mod parse;
pub mod source;

pub use analysis::{Finding, FindingSeverity, Options, analyze};
pub use model::{
    Dd, DdKind, Job, JobEnd, Message, Severity, Statement, StatementOrigin, Step, StepStatus,
};
pub use parse::{parse_dd, parse_file};
pub use source::DdSource;
