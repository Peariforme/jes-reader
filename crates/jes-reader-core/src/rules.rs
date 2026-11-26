//! House rules (`.jesreader.toml`): adapt the analysis to a site's
//! conventions without changing the tool.
//!
//! ```toml
//! # Return code above which a step is reported (default 4).
//! max_return_code = 4
//!
//! # Messages: severity "error", "warning", "info" or "ignore".
//! [[messages]]
//! id = "IDI0078E"            # message ID, `*` allowed: "IDI*"
//! severity = "ignore"
//! note = "Fault Analyzer history file, not about the job"
//!
//! [[messages]]
//! id = "IEF287I"
//! text = "NOT CATLGD 2"      # optional: only when the text contains this
//! severity = "error"
//!
//! # Steps: return code tolerated, by step name and/or program (`*` allowed).
//! [[steps]]
//! name = "COBRUN.LKED"       # `step` or `step.procstep`
//! max_return_code = 8
//!
//! [[steps]]
//! program = "IDCAMS"         # `PGM=` of the EXEC statement, read in JESJCL
//! max_return_code = 4
//! note = "RC 4: empty file, expected"
//! ```
//!
//! Rules apply in file order: the first matching rule wins. A step rule
//! with both `name` and `program` needs both to match. A `program` rule
//! only applies when the JESJCL of the job is available (open in the
//! editor, or next to the downloaded file).

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Name of a house rules file.
pub const FILE_NAME: &str = ".jesreader.toml";

/// First `.jesreader.toml` found in `dirs`, in order, then in the user's
/// home directory.
pub fn find(dirs: &[PathBuf]) -> Option<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    dirs.iter()
        .cloned()
        .chain(home)
        .map(|dir| dir.join(FILE_NAME))
        .find(|path| path.is_file())
}

/// Severity a message rule gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleSeverity {
    /// Report as an error.
    Error,
    /// Report as a warning.
    Warning,
    /// Report as information.
    Info,
    /// Do not report.
    Ignore,
}

/// Rule on messages.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MessageRule {
    /// Message ID pattern (`IDI0078E`, `IDI*`).
    pub id: String,
    /// Only when the message text contains this.
    pub text: Option<String>,
    /// Severity to report the message with.
    pub severity: Option<RuleSeverity>,
    /// Added to the diagnostic text.
    pub note: Option<String>,
}

/// Rule on steps.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepRule {
    /// Step name pattern: `STEP020`, `COBRUN.LKED`, `IDC*`.
    pub name: Option<String>,
    /// Program pattern (`PGM=` of the EXEC statement): `IDCAMS`, `IEB*`.
    pub program: Option<String>,
    /// Return code tolerated for this step.
    pub max_return_code: Option<u16>,
    /// Added to the diagnostic text.
    pub note: Option<String>,
}

/// Contents of a `.jesreader.toml` file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    /// Return code above which a step is reported, for all steps.
    pub max_return_code: Option<u16>,
    /// Message rules, in file order.
    #[serde(default)]
    pub messages: Vec<MessageRule>,
    /// Step rules, in file order.
    #[serde(default)]
    pub steps: Vec<StepRule>,
}

impl Rules {
    /// Parses a rules file. The error message names the offending key or
    /// line.
    pub fn parse(text: &str) -> Result<Self, String> {
        let rules: Self = toml::from_str(text).map_err(|e| e.to_string())?;
        if let Some(i) = rules
            .steps
            .iter()
            .position(|r| r.name.is_none() && r.program.is_none())
        {
            return Err(format!(
                "[[steps]] rule {}: needs `name` or `program`",
                i + 1
            ));
        }
        Ok(rules)
    }

    /// Reads and parses a rules file; the error starts with its path.
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// First message rule matching a message ID and text.
    pub fn message(&self, id: &str, text: &str) -> Option<&MessageRule> {
        self.messages
            .iter()
            .find(|r| matches(&r.id, id) && r.text.as_deref().is_none_or(|t| text.contains(t)))
    }

    /// First step rule matching a step, by `name` or `name.procstep`, and
    /// by program when the rule names one.
    pub fn step(&self, name: &str, display_name: &str, program: Option<&str>) -> Option<&StepRule> {
        self.steps.iter().find(|r| {
            let name_ok = r
                .name
                .as_deref()
                .is_none_or(|p| matches(p, display_name) || matches(p, name));
            let program_ok = r
                .program
                .as_deref()
                .is_none_or(|p| program.is_some_and(|program| matches(p, program)));
            name_ok && program_ok
        })
    }
}

/// Pattern match where `*` stands for any sequence of characters.
fn matches(pattern: &str, value: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    let [first, middle @ .., last] = parts.as_slice() else {
        return pattern == value;
    };
    let Some(mut rest) = value.strip_prefix(first) else {
        return false;
    };
    for part in middle {
        match rest.find(part) {
            Some(i) => rest = &rest[i + part.len()..],
            None => return false,
        }
    }
    rest.len() >= last.len() && rest.ends_with(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patterns() {
        assert!(matches("IDI0078E", "IDI0078E"));
        assert!(!matches("IDI0078E", "IDI0078"));
        assert!(matches("IDI*", "IDI0078E"));
        assert!(matches("*E", "IDI0078E"));
        assert!(matches("COBRUN.*", "COBRUN.GO"));
        assert!(matches("*.GO", "COBRUN.GO"));
        assert!(matches("C*N.*O", "COBRUN.GO"));
        assert!(!matches("COBRUN.*", "STEP010"));
        assert!(matches("*", ""));
        assert!(!matches("AB*AB", "AB"));
    }

    #[test]
    fn parse_and_lookup() {
        let rules = Rules::parse(
            r#"
            max_return_code = 8

            [[messages]]
            id = "IDI*"
            severity = "ignore"

            [[messages]]
            id = "IEF287I"
            text = "NOT CATLGD 2"
            severity = "error"
            note = "duplicate data set"

            [[steps]]
            name = "COBRUN.LKED"
            max_return_code = 4

            [[steps]]
            name = "IDC*"
            max_return_code = 12
            "#,
        )
        .unwrap();
        assert_eq!(rules.max_return_code, Some(8));
        assert_eq!(
            rules.message("IDI0078E", "").unwrap().severity,
            Some(RuleSeverity::Ignore)
        );
        assert!(rules.message("IEF287I", "X NOT CATLGD 2").is_some());
        assert!(rules.message("IEF287I", "X NOT CATLGD 5").is_none());
        assert_eq!(
            rules
                .step("COBRUN", "COBRUN.LKED", None)
                .unwrap()
                .max_return_code,
            Some(4)
        );
        assert_eq!(
            rules
                .step("IDCSTEP", "IDCSTEP", None)
                .unwrap()
                .max_return_code,
            Some(12)
        );
        assert!(rules.step("COBRUN", "COBRUN.GO", None).is_none());
    }

    #[test]
    fn step_rules_by_program() {
        let rules = Rules::parse(
            r#"
            [[steps]]
            name = "LOAD*"
            program = "IDCAMS"
            max_return_code = 8

            [[steps]]
            program = "IDCAMS"
            max_return_code = 4
            "#,
        )
        .unwrap();
        let rc = |name, program| rules.step(name, name, program).map(|r| r.max_return_code);
        assert_eq!(rc("LOAD01", Some("IDCAMS")), Some(Some(8)));
        assert_eq!(rc("STEP010", Some("IDCAMS")), Some(Some(4)));
        assert_eq!(rc("STEP010", Some("IEBGENER")), None);
        // Program unknown (no JESJCL): program rules do not apply.
        assert_eq!(rc("LOAD01", None), None);

        let error = Rules::parse("[[steps]]\nmax_return_code = 4\n").unwrap_err();
        assert!(
            error.contains("name") && error.contains("program"),
            "{error}"
        );
    }

    #[test]
    fn errors_name_the_problem() {
        let error = Rules::parse("[[messages]]\nid = \"X\"\nseverity = \"loud\"\n").unwrap_err();
        assert!(
            error.contains("loud") || error.contains("severity"),
            "{error}"
        );
        let error = Rules::parse("max_rc = 4\n").unwrap_err();
        assert!(error.contains("max_rc"), "{error}");
        assert_eq!(Rules::parse("").unwrap(), Rules::default());
    }
}
