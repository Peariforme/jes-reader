//! `jes-reader check`: findings as text, JSON or SARIF.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use jes_reader_core::catalog::{catalog, docs_url, message_page_url};
use jes_reader_core::{Finding, FindingSeverity, Options, analyze};
use serde_json::{Value, json};

use crate::files::File;

/// Findings of every file.
pub struct Report<'a> {
    files: usize,
    findings: Vec<(&'a Path, Finding)>,
}

impl<'a> Report<'a> {
    pub fn new(files: &'a [File], options: &Options) -> Self {
        let findings = files
            .iter()
            .flat_map(|f| {
                analyze(&f.dd, options)
                    .into_iter()
                    .map(|finding| (f.path.as_path(), finding))
            })
            .collect();
        Self {
            files: files.len(),
            findings,
        }
    }

    fn count(&self, severity: FindingSeverity) -> usize {
        self.findings
            .iter()
            .filter(|(_, f)| f.severity == severity)
            .count()
    }

    pub fn errors(&self) -> usize {
        self.count(FindingSeverity::Error)
    }

    /// `file:line:column: severity[code] message`, then a total line.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for (path, f) in &self.findings {
            let column = f.columns.as_ref().map_or(1, |c| c.start + 1);
            writeln!(
                out,
                "{}:{}:{column}: {}[{}] {}",
                path.display(),
                f.line + 1,
                label(f.severity),
                f.code,
                f.message
            )
            .unwrap();
        }
        writeln!(
            out,
            "{} file(s), {} error(s), {} warning(s)",
            self.files,
            self.errors(),
            self.count(FindingSeverity::Warning)
        )
        .unwrap();
        out
    }

    /// Lines and columns counted from 1, as in the text output.
    pub fn json(&self) -> Value {
        let findings: Vec<Value> = self
            .findings
            .iter()
            .map(|(path, f)| {
                json!({
                    "file": path.display().to_string(),
                    "line": f.line + 1,
                    "column": f.columns.as_ref().map(|c| c.start + 1),
                    "endColumn": f.columns.as_ref().map(|c| c.end + 1),
                    "severity": label(f.severity),
                    "code": f.code,
                    "message": f.message,
                    "related": f.related.iter().map(|r| json!({
                        "line": r.line + 1,
                        "message": r.message,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect();
        json!({
            "files": self.files,
            "errors": self.errors(),
            "warnings": self.count(FindingSeverity::Warning),
            "findings": findings,
        })
    }

    /// SARIF 2.1.0: one rule per finding code (message ID, abend code, `RC`,
    /// `FLUSH`), one result per finding.
    pub fn sarif(&self) -> Value {
        let mut rules = BTreeMap::new();
        for (_, f) in &self.findings {
            rules
                .entry(f.code.as_str())
                .or_insert_with(|| rule(&f.code));
        }
        let index: BTreeMap<&str, usize> = rules.keys().enumerate().map(|(i, k)| (*k, i)).collect();
        let results: Vec<Value> = self
            .findings
            .iter()
            .map(|(path, f)| {
                let uri = uri(path);
                let mut region = json!({ "startLine": f.line + 1 });
                if let Some(c) = &f.columns {
                    region["startColumn"] = json!(c.start + 1);
                    region["endColumn"] = json!(c.end + 1);
                }
                let related: Vec<Value> = f
                    .related
                    .iter()
                    .enumerate()
                    .map(|(id, r)| {
                        json!({
                            "id": id,
                            "message": { "text": r.message },
                            "physicalLocation": {
                                "artifactLocation": { "uri": uri },
                                "region": { "startLine": r.line + 1 },
                            },
                        })
                    })
                    .collect();
                let mut result = json!({
                    "ruleId": f.code,
                    "ruleIndex": index[f.code.as_str()],
                    "level": match f.severity {
                        FindingSeverity::Error => "error",
                        FindingSeverity::Warning => "warning",
                        FindingSeverity::Information => "note",
                    },
                    "message": { "text": f.message },
                    "locations": [{
                        "physicalLocation": {
                            "artifactLocation": { "uri": uri },
                            "region": region,
                        },
                    }],
                });
                if !related.is_empty() {
                    result["relatedLocations"] = json!(related);
                }
                result
            })
            .collect();
        json!({
            "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
            "version": "2.1.0",
            "runs": [{
                "tool": {
                    "driver": {
                        "name": "jes-reader",
                        "version": env!("CARGO_PKG_VERSION"),
                        "informationUri": env!("CARGO_PKG_REPOSITORY"),
                        "rules": rules.into_values().collect::<Vec<_>>(),
                    },
                },
                "results": results,
            }],
        })
    }
}

fn label(severity: FindingSeverity) -> &'static str {
    match severity {
        FindingSeverity::Error => "error",
        FindingSeverity::Warning => "warning",
        FindingSeverity::Information => "info",
    }
}

/// SARIF rule of a finding code, described from the catalog.
fn rule(code: &str) -> Value {
    let (description, help) = match code {
        "RC" => (
            "Step return code above the tolerated value".to_owned(),
            None,
        ),
        "FLUSH" => ("Step did not run".to_owned(), None),
        _ => match (catalog().message(code), catalog().abend(code)) {
            (Some(entry), _) => (
                entry.title.clone(),
                Some(message_page_url(code).unwrap_or_else(|| docs_url(code))),
            ),
            (None, Some(entry)) => (
                format!("Abend {code}: {}", entry.title),
                Some(docs_url(code)),
            ),
            (None, None) if code.starts_with(['S', 'U']) && code.len() == 4 => {
                (format!("Abend {code}"), Some(docs_url(code)))
            }
            (None, None) => (
                format!("Message {code}"),
                Some(message_page_url(code).unwrap_or_else(|| docs_url(code))),
            ),
        },
    };
    let mut rule = json!({
        "id": code,
        "shortDescription": { "text": description },
    });
    if let Some(help) = help {
        rule["helpUri"] = json!(help);
    }
    rule
}

/// URI of a file for SARIF: relative paths stay relative (to the directory
/// the command ran in), with `/` separators.
fn uri(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let mut out = String::new();
    for c in text.chars() {
        match c {
            ' ' => out.push_str("%20"),
            '%' => out.push_str("%25"),
            '#' => out.push_str("%23"),
            '?' => out.push_str("%3F"),
            c => out.push(c),
        }
    }
    if out.starts_with('/') {
        format!("file://{out}")
    } else if out.as_bytes().get(1) == Some(&b':') {
        format!("file:///{out}")
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uris() {
        assert_eq!(
            uri(Path::new("JOB00896/LKED/COBRUN/SYSPRINT(1).txt")),
            "JOB00896/LKED/COBRUN/SYSPRINT(1).txt"
        );
        assert_eq!(
            uri(Path::new("/tmp/my job/x.txt")),
            "file:///tmp/my%20job/x.txt"
        );
        assert_eq!(uri(Path::new("C:\\jobs\\x.txt")), "file:///C:/jobs/x.txt");
    }

    #[test]
    fn rules() {
        assert_eq!(rule("RC")["id"], "RC");
        assert!(rule("RC").get("helpUri").is_none());
        let s0c7 = rule("S0C7");
        assert!(
            s0c7["shortDescription"]["text"]
                .as_str()
                .unwrap()
                .starts_with("Abend S0C7: "),
            "{s0c7}"
        );
        assert!(
            rule("IEF472I")["helpUri"]
                .as_str()
                .unwrap()
                .contains("ief472i")
        );
    }
}
