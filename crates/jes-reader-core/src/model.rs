//! Modèle de données d'une sysout : Job → DD → step → message.
//!
//! Positions : les numéros de ligne sont 0-indexés (bornes hautes exclues)
//! et les colonnes sont des offsets en octets dans la ligne, caractère ASA
//! compris. Le texte d'une sysout est en pratique de l'ASCII ; la
//! conversion vers les colonnes UTF-16 du LSP se fait côté serveur.

use std::ops::Range;

/// Sévérité d'un message IBM, tirée du dernier caractère de son identifiant
/// (`IEF142I`, `IEC161W`, `IGD17101E`…).
///
/// Attention : beaucoup de messages d'erreur JES2 et MVS finissent par `I`
/// (`IEF453I` JCL ERROR, `IEF472I` abend). La gravité réelle d'un message
/// est décidée par [`crate::analysis`], pas par ce seul suffixe.
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

/// Nature d'une DD, qui détermine la façon de la lire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DdKind {
    /// Journal du job (`JESMSGLG`) : messages horodatés, tableau des steps.
    JesMsgLg,
    /// JCL développé (`JESJCL`) : instructions numérotées.
    JesJcl,
    /// Messages système (`JESYSMSG`) : allocations, fins de step.
    JesYsMsg,
    /// Toute autre DD (`SYSPRINT`, `SYSOUT`, `CEEDUMP`…).
    Other,
}

/// Message identifié dans une DD.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Identifiant (`IEF142I`).
    pub id: String,
    /// Sévérité tirée du suffixe, si l'identifiant en porte une.
    pub severity: Option<Severity>,
    /// Lignes occupées, lignes de suite comprises.
    pub lines: Range<usize>,
    /// Colonnes de l'identifiant dans la première ligne.
    pub id_columns: Range<usize>,
    /// Texte de la première ligne après l'identifiant, sans les espaces
    /// de bord.
    pub text: String,
    /// Numéro d'instruction JCL (colonne `STMT NO.` de JESYSMSG).
    pub statement: Option<u32>,
}

/// Issue d'un step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepStatus {
    /// Exécuté jusqu'au bout avec ce code retour.
    Completed {
        /// Code retour (`COND CODE`).
        return_code: u16,
    },
    /// Fin anormale.
    Abended {
        /// Code d'abend : `S0C7` (système) ou `U4038` (utilisateur).
        code: String,
        /// Code raison, s'il est donné.
        reason: Option<String>,
    },
    /// Non exécuté (condition, JCL ERROR, step précédent en abend).
    NotRun,
    /// Issue inconnue : step en cours ou messages de fin absents.
    Unknown,
}

/// Step d'un job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// Nom du step du job (`COBRUN`, `STEP010`).
    pub name: String,
    /// Nom du step dans la procédure appelée (`COBOL`, `GO`), le cas échéant.
    pub proc_step: Option<String>,
    /// Issue du step.
    pub status: StepStatus,
    /// Lignes occupées dans la DD.
    pub lines: Range<usize>,
    /// Ligne qui donne l'issue du step (`IEF142I`, `IEF472I`, ligne du
    /// tableau de JESMSGLG…), à défaut la première ligne du step.
    pub status_line: usize,
}

impl Step {
    /// Nom affiché : `COBRUN.COBOL` pour un step de procédure, sinon `STEP010`.
    pub fn display_name(&self) -> String {
        match &self.proc_step {
            Some(proc_step) => format!("{}.{}", self.name, proc_step),
            None => self.name.clone(),
        }
    }
}

/// Origine d'une instruction de JESJCL, d'après ses deux premiers caractères.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementOrigin {
    /// `//` : écrite dans le job.
    Job,
    /// `XX` ou `++` : issue d'une procédure (cataloguée ou in-stream).
    Procedure,
    /// `X/` ou `+/` : instruction de procédure surchargée par le job.
    Override,
}

/// Instruction JCL numérotée de JESJCL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    /// Numéro d'instruction (cible des liens depuis JESYSMSG).
    pub number: u32,
    /// Origine de l'instruction.
    pub origin: StatementOrigin,
    /// Nom (étiquette) : nom du job, du step ou de la DD.
    pub name: Option<String>,
    /// Opération : `JOB`, `EXEC`, `DD`, `PROC`, `SET`…
    pub operation: Option<String>,
    /// Lignes occupées, continuations et commentaires suivants compris.
    pub lines: Range<usize>,
}

/// Fin du job, lue dans `$HASP395` et `IEF453I` (JESMSGLG).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobEnd {
    /// Terminé avec ce code retour maximal.
    ReturnCode(u16),
    /// Terminé par un abend.
    Abend(String),
    /// Arrêté en JCL ERROR.
    JclError,
    /// Terminé sans précision.
    Ended,
}

/// DD de sortie (JESMSGLG, JESJCL, JESYSMSG, SYSPRINT…).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dd {
    /// Nom de la DD, s'il est connu (nom de fichier, URI ou contenu).
    pub name: Option<String>,
    /// Nature de la DD.
    pub kind: DdKind,
    /// Nom du job, s'il apparaît dans la DD.
    pub job_name: Option<String>,
    /// Identifiant JES (`JOB00823`, `TSU00812`), tiré du chemin ou du contenu.
    pub job_id: Option<String>,
    /// Vrai si la colonne 1 porte un caractère de contrôle ASA.
    pub has_asa: bool,
    /// Nombre de lignes de la DD.
    pub line_count: usize,
    /// Messages reconnus, dans l'ordre du texte.
    pub messages: Vec<Message>,
    /// Steps (JESYSMSG et tableau des steps de JESMSGLG).
    pub steps: Vec<Step>,
    /// Instructions JCL (JESJCL).
    pub statements: Vec<Statement>,
    /// Fin du job (JESMSGLG).
    pub job_end: Option<JobEnd>,
}

/// Sysout complète d'un job : agrégat des DD ouvertes d'un même identifiant JES.
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
}
