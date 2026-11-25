//! Our catalog of IBM messages and abend codes (`catalog.toml`), embedded
//! in the binary: short summaries in our own words, used by hovers and
//! diagnostics, plus severity overrides for messages whose suffix is
//! misleading.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

/// Severity a catalog entry forces, whatever the message suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CatalogSeverity {
    /// Reported as an error.
    Error,
    /// Reported as a warning.
    Warning,
    /// Not reported.
    Info,
}

/// Catalog entry of a message.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MessageEntry {
    /// Short name (`OPEN failed`).
    pub title: String,
    /// What the message means.
    pub summary: String,
    /// What to check or do.
    pub action: Option<String>,
    /// Severity override.
    pub severity: Option<CatalogSeverity>,
}

/// Catalog entry of an abend code.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbendEntry {
    /// Short name (`Data exception`).
    pub title: String,
    /// What the abend means.
    pub summary: String,
    /// What to check or do.
    pub action: Option<String>,
    /// Reason code (hex, without leading zeros) to meaning.
    #[serde(default)]
    pub reasons: HashMap<String, String>,
}

impl AbendEntry {
    /// Meaning of a reason code, given as `18`, `0018` or `00000018`.
    pub fn reason(&self, code: &str) -> Option<&str> {
        let code = code.trim_start_matches('0');
        let code = if code.is_empty() { "0" } else { code };
        self.reasons
            .get(&code.to_ascii_uppercase())
            .map(String::as_str)
    }
}

/// The whole catalog.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    messages: HashMap<String, MessageEntry>,
    abends: HashMap<String, AbendEntry>,
}

impl Catalog {
    /// Entry of a message ID (`IEC141I`, `$HASP395`).
    pub fn message(&self, id: &str) -> Option<&MessageEntry> {
        self.messages.get(id)
    }

    /// Entry of an abend code (`S0C7`, `U4038`).
    pub fn abend(&self, code: &str) -> Option<&AbendEntry> {
        self.abends.get(code)
    }

    /// Message IDs the catalog reports as errors.
    pub fn error_messages(&self) -> impl Iterator<Item = &str> {
        self.messages
            .iter()
            .filter(|(_, e)| e.severity == Some(CatalogSeverity::Error))
            .map(|(id, _)| id.as_str())
    }
}

/// The embedded catalog, parsed once.
pub fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        // A broken catalog fails the `catalog_loads` test, never at run time
        // in a released binary.
        toml::from_str(include_str!("../catalog.toml")).expect("catalog.toml is valid")
    })
}

/// IBM Docs search page for a message ID or abend code.
pub fn docs_url(term: &str) -> String {
    let term: String = term
        .chars()
        .map(|c| match c {
            '$' => "%24".to_owned(),
            '#' => "%23".to_owned(),
            '@' => "%40".to_owned(),
            c => c.to_string(),
        })
        .collect();
    format!("https://www.ibm.com/docs/en/search/{term}")
}

/// Abend code and reason at the start of an IBM data management message
/// (`013-18,IGG0191B,…` in IEC141I): `("S013", "18")`.
pub fn leading_abend(text: &str) -> Option<(String, &str)> {
    let (code, rest) = text.split_once('-')?;
    let reason = rest.split([',', ' ']).next()?;
    let is_hex = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_hexdigit());
    (code.len() == 3 && is_hex(code) && (1..=2).contains(&reason.len()) && is_hex(reason))
        .then(|| (format!("S{code}"), reason))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_loads() {
        let c = catalog();
        assert_eq!(c.message("IEC141I").unwrap().title, "OPEN failed");
        assert!(c.message("$HASP395").is_some());
        assert_eq!(c.abend("S0C7").unwrap().title, "Data exception");
        assert!(c.message("NOPE001I").is_none());
    }

    #[test]
    fn reasons() {
        let s013 = catalog().abend("S013").unwrap();
        assert!(s013.reason("18").unwrap().contains("member not found"));
        assert_eq!(s013.reason("00000018"), s013.reason("18"));
        assert!(s013.reason("99").is_none());
        let s806 = catalog().abend("S806").unwrap();
        assert!(s806.reason("04").is_some());
    }

    #[test]
    fn every_entry_is_complete() {
        let c = catalog();
        for (id, e) in &c.messages {
            assert!(!e.title.is_empty() && !e.summary.is_empty(), "{id}");
            assert!(crate::msgid::is_message_id(id), "not a message ID: {id}");
        }
        for (code, e) in &c.abends {
            assert!(!e.title.is_empty() && !e.summary.is_empty(), "{code}");
            let hex = &code[1..];
            assert!(
                (code.starts_with('S')
                    && hex.len() == 3
                    && hex.chars().all(|c| c.is_ascii_hexdigit()))
                    || (code.starts_with('U')
                        && hex.len() == 4
                        && hex.chars().all(|c| c.is_ascii_digit())),
                "not an abend code: {code}"
            );
            for reason in e.reasons.keys() {
                assert!(
                    !reason.starts_with('0') && reason.chars().all(|c| c.is_ascii_hexdigit()),
                    "{code} reason {reason}"
                );
            }
        }
    }

    /// The TextMate grammar colors known error messages in red: it must list
    /// every message the catalog reports as an error.
    #[test]
    fn grammar_lists_error_messages() {
        let grammar = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../editors/vscode/syntaxes/jes-sysout.tmLanguage.json"
        ))
        .unwrap();
        let line = grammar
            .lines()
            .skip_while(|l| !l.contains("\"known-messages\""))
            .find(|l| l.contains("\"match\""))
            .unwrap();
        for id in catalog().error_messages() {
            assert!(
                line.contains(id),
                "{id} missing from the grammar's known-messages"
            );
        }
    }

    #[test]
    fn docs_urls() {
        assert_eq!(
            docs_url("IEC141I"),
            "https://www.ibm.com/docs/en/search/IEC141I"
        );
        assert_eq!(
            docs_url("$HASP395"),
            "https://www.ibm.com/docs/en/search/%24HASP395"
        );
    }

    #[test]
    fn leading_abends() {
        assert_eq!(
            leading_abend("013-18,IGG0191B,JRS013,STEP010,SYSUT1"),
            Some(("S013".into(), "18"))
        );
        assert_eq!(
            leading_abend("B37-04,IFG0554A"),
            Some(("SB37".into(), "04"))
        );
        assert_eq!(leading_abend("JRS0C7 GO COBRUN - STEP"), None);
        assert_eq!(leading_abend("DATA SET X NOT FOUND"), None);
    }
}
