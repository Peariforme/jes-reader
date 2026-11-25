//! Règles qui transforment une DD analysée en constats : ce que le LSP
//! affiche dans Problems et ce que la CLI signale.
//!
//! La gravité ne se lit pas sur le seul suffixe des messages : `IEF453I`
//! (JCL ERROR) ou `IEFA107I` (dataset introuvable) finissent par `I`. Les
//! règles portent donc d'abord sur les steps, puis sur une liste de messages
//! connus, et en dernier sur le suffixe (`E`, `S`, `W`).

use std::ops::Range;

use crate::model::{Dd, Message, StepStatus};

/// Gravité d'un constat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FindingSeverity {
    /// Le job ou un step a échoué.
    Error,
    /// À vérifier : code retour élevé, message d'avertissement.
    Warning,
    /// Pour information : step non exécuté.
    Information,
}

/// Constat sur une DD.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Gravité.
    pub severity: FindingSeverity,
    /// Code court : identifiant du message, code d'abend ou `RC`.
    pub code: String,
    /// Texte affiché.
    pub message: String,
    /// Ligne concernée.
    pub line: usize,
    /// Colonnes concernées ; `None` pour toute la ligne.
    pub columns: Option<Range<usize>>,
}

/// Réglages des règles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Un code retour strictement supérieur à ce seuil est un avertissement.
    pub max_return_code: u16,
}

impl Default for Options {
    fn default() -> Self {
        Self { max_return_code: 4 }
    }
}

/// Messages en erreur quel que soit leur suffixe.
const ERROR_MESSAGES: &[&str] = &[
    "IEF453I",   // JOB FAILED - JCL ERROR
    "IEFA107I",  // DATA SET … NOT FOUND (allocation)
    "IEF212I",   // DATA SET NOT FOUND
    "IGD17101I", // DUPLICATE NAME EXISTS IN CATALOG
    "CSV003I",   // REQUESTED MODULE … NOT FOUND
    "IEC141I",   // 013-xx : échec d'OPEN
];

/// Messages en avertissement quel que soit leur suffixe.
const WARNING_MESSAGES: &[&str] = &[
    "IEF287I", // NOT CATLGD
];

/// Messages de conversion numérotés purement informatifs.
const INFO_STATEMENT_MESSAGES: &[&str] = &[
    "IEFC001I", // PROCEDURE … WAS EXPANDED
    "IEFC002I", // INCLUDE GROUP … WAS EXPANDED
];

/// Constats d'une DD, du plus grave au moins grave puis par ligne.
pub fn analyze(dd: &Dd, options: &Options) -> Vec<Finding> {
    let mut findings = Vec::new();

    for step in &dd.steps {
        let name = step.display_name();
        let (severity, code, message) = match &step.status {
            StepStatus::Abended { code, reason } => (
                FindingSeverity::Error,
                code.clone(),
                match reason {
                    Some(reason) => format!("Step {name} : abend {code}, raison {reason}"),
                    None => format!("Step {name} : abend {code}"),
                },
            ),
            StepStatus::Completed { return_code } if *return_code > options.max_return_code => (
                FindingSeverity::Warning,
                "RC".to_owned(),
                format!("Step {name} : code retour {return_code}"),
            ),
            StepStatus::NotRun => (
                FindingSeverity::Information,
                "FLUSH".to_owned(),
                format!("Step {name} non exécuté"),
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
    // `IEF450I` (abend, JESMSGLG) double la ligne du tableau des steps ; il
    // ne compte que si l'installation n'imprime pas ce tableau.
    if id == "IEF450I" {
        return dd.steps.is_empty().then_some(FindingSeverity::Error);
    }
    // Message du convertisseur JCL (`IEF…`, `IEFC…`) rattaché à une
    // instruction : le JCL a été corrigé ou complété. Les autres messages
    // numérotés (`IGD01007I` des routines ACS) suivent la règle générale.
    if m.statement.is_some() && id.starts_with("IEF") && !INFO_STATEMENT_MESSAGES.contains(&id) {
        return Some(FindingSeverity::Warning);
    }
    match id.as_bytes().last() {
        Some(b'E' | b'S') if !id.starts_with('$') => Some(FindingSeverity::Error),
        Some(b'W') => Some(FindingSeverity::Warning),
        _ => None,
    }
}
