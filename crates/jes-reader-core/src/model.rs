//! Modèle de données d'une sysout : Job → DD → step → message.

use std::ops::Range;

/// Sévérité d'un message IBM, tirée du dernier caractère de son identifiant
/// (`IEF142I`, `IEC161W`, `IGD17101E`…).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// `I` : information.
    Info,
    /// `W` : avertissement.
    Warning,
    /// `E` : erreur.
    Error,
    /// `S` : erreur grave.
    Severe,
    /// `A` : action requise de l'opérateur.
    Action,
}

impl Severity {
    /// Déduit la sévérité du suffixe d'un identifiant de message.
    ///
    /// Renvoie `None` si le dernier caractère n'est pas une lettre de
    /// sévérité connue (par exemple `$HASP373`, qui n'a pas de suffixe).
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

/// Message identifié dans une DD.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Identifiant (`IEF142I`).
    pub id: String,
    /// Sévérité, si l'identifiant en porte une.
    pub severity: Option<Severity>,
    /// Lignes occupées dans le document (0-indexées, borne haute exclue).
    pub lines: Range<usize>,
}

/// Step d'un job, délimité par ses messages IEF de début et de fin.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Step {
    /// Nom du step (`STEP010`).
    pub name: String,
    /// Programme exécuté (`PGM=`), s'il est connu.
    pub program: Option<String>,
    /// Code retour, s'il est connu.
    pub return_code: Option<u16>,
    /// Code d'abend (`S0C7`, `U4038`), le cas échéant.
    pub abend: Option<String>,
    /// Lignes occupées dans le document.
    pub lines: Range<usize>,
}

/// DD de sortie (JESMSGLG, JESJCL, JESYSMSG, SYSPRINT…).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Dd {
    /// Nom de la DD.
    pub name: String,
    /// Steps rattachés à cette DD.
    pub steps: Vec<Step>,
    /// Messages reconnus dans cette DD.
    pub messages: Vec<Message>,
    /// Lignes occupées dans le document.
    pub lines: Range<usize>,
}

/// Sysout complète d'un job.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Job {
    /// Nom du job.
    pub name: Option<String>,
    /// Identifiant JES (`JOB01234`).
    pub id: Option<String>,
    /// DD dans l'ordre du spool.
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
}
