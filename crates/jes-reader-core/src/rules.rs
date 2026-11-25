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
//! # Steps: return code tolerated, by step name (`*` allowed).
//! [[steps]]
//! name = "COBRUN.LKED"       # `step` or `step.procstep`
//! max_return_code = 8
//! ```
//!
//! Rules apply in file order: the first matching rule wins.

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
    pub name: String,
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
        toml::from_str(text).map_err(|e| e.to_string())
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

    /// First step rule matching a step, by `name` or `name.procstep`.
    pub fn step(&self, name: &str, display_name: &str) -> Option<&StepRule> {
        self.steps
            .iter()
            .find(|r| matches(&r.name, display_name) || matches(&r.name, name))
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
            rules.step("COBRUN", "COBRUN.LKED").unwrap().max_return_code,
            Some(4)
        );
        assert_eq!(
            rules.step("IDCSTEP", "IDCSTEP").unwrap().max_return_code,
            Some(12)
        );
        assert!(rules.step("COBRUN", "COBRUN.GO").is_none());
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
