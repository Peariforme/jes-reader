//! Identification d'une DD : par son chemin ou son nom, à défaut par son
//! contenu.
//!
//! Formes de chemin reconnues (voir `docs/FORMAT.md`) :
//!
//! - téléchargement Zowe Explorer : `…/<jobid>/JES2/<DD>.txt`,
//!   `…/<jobid>/<step>/<DD>.txt`, `…/<jobid>/<procstep>/<step>/<DD>.txt`,
//!   avec parfois un suffixe `(1)` avant l'extension ;
//! - document ouvert par Zowe sans téléchargement :
//!   `<jobname>.<jobid>.<step>.<DD>.<n>`.

use crate::model::DdKind;

/// Ce que le chemin d'un document apprend sur la DD qu'il contient.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DdSource {
    /// Identifiant JES (`JOB00823`).
    pub job_id: Option<String>,
    /// Nom du job (forme `<jobname>.<jobid>…` uniquement).
    pub job_name: Option<String>,
    /// Nom de la DD.
    pub dd_name: Option<String>,
}

impl DdSource {
    /// Analyse un chemin ou une URI. Ne renvoie jamais d'erreur : ce qui
    /// n'est pas reconnu reste à `None`.
    pub fn from_path(path: &str) -> Self {
        let components: Vec<&str> = path.split(['/', '\\']).filter(|c| !c.is_empty()).collect();
        let Some(file_name) = components.last() else {
            return Self::default();
        };

        // Forme Zowe sans téléchargement : JRIDC12.JOB00922.JES2.JESMSGLG.2
        let segments: Vec<&str> = file_name.split('.').collect();
        if let Some(pos) = segments.iter().position(|s| is_job_id(s)) {
            let rest = &segments[pos + 1..];
            // <step>… <DD> <n> : la DD est l'avant-dernier segment.
            if rest.len() >= 3 && rest.last().is_some_and(|n| is_number(n)) {
                return Self {
                    job_id: Some(segments[pos].to_owned()),
                    job_name: (pos > 0).then(|| segments[pos - 1].to_owned()),
                    dd_name: Some(rest[rest.len() - 2].to_owned()),
                };
            }
        }

        // Forme téléchargement : …/JOB00823/JES2/JESMSGLG.txt
        let job_id = components[..components.len() - 1]
            .iter()
            .rev()
            .find(|c| is_job_id(c))
            .map(|c| (*c).to_owned());
        let stem = file_name
            .rsplit_once('.')
            .map_or(*file_name, |(stem, _)| stem);
        let stem = strip_copy_suffix(stem);
        let dd_name = is_dd_name(stem).then(|| stem.to_owned());
        Self {
            job_id,
            job_name: None,
            dd_name,
        }
    }
}

impl DdKind {
    /// Nature d'une DD d'après son nom.
    pub fn from_dd_name(name: &str) -> Self {
        match name {
            "JESMSGLG" => Self::JesMsgLg,
            "JESJCL" => Self::JesJcl,
            "JESYSMSG" => Self::JesYsMsg,
            _ => Self::Other,
        }
    }

    /// Reconnaît une DD système JES2 à sa première ligne non vide.
    pub fn sniff(text: &str) -> Option<Self> {
        let first = text.lines().find(|l| !l.trim().is_empty())?;
        if first.contains("J E S 2  J O B  L O G") {
            return Some(Self::JesMsgLg);
        }
        if first.trim_start().starts_with("STMT NO. MESSAGE") {
            return Some(Self::JesYsMsg);
        }
        // `        1 //NOM      JOB …` : numéro en col. 9, `//` en col. 11.
        let number = first.get(1..9).map(str::trim);
        if number == Some("1") && first.get(10..12) == Some("//") && first.contains(" JOB ") {
            return Some(Self::JesJcl);
        }
        None
    }
}

/// Nom de la DD standard d'une DD système JES2.
pub(crate) fn jes_dd_name(kind: DdKind) -> Option<&'static str> {
    match kind {
        DdKind::JesMsgLg => Some("JESMSGLG"),
        DdKind::JesJcl => Some("JESJCL"),
        DdKind::JesYsMsg => Some("JESYSMSG"),
        DdKind::Other => None,
    }
}

/// `JOB00823`, `TSU00812`, `STC01234`, ou forme à 7 chiffres `J0123456`.
pub fn is_job_id(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 8 {
        return false;
    }
    let five_digits =
        matches!(&b[..3], b"JOB" | b"TSU" | b"STC") && b[3..].iter().all(u8::is_ascii_digit);
    let seven_digits = matches!(b[0], b'J' | b'T' | b'S') && b[1..].iter().all(u8::is_ascii_digit);
    five_digits || seven_digits
}

fn is_number(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit())
}

/// Retire le suffixe `(1)` que Zowe ajoute parfois au nom de fichier.
fn strip_copy_suffix(stem: &str) -> &str {
    if let Some(open) = stem.rfind('(')
        && stem.ends_with(')')
        && is_number(&stem[open + 1..stem.len() - 1])
    {
        return &stem[..open];
    }
    stem
}

/// Nom de DD valide : 1 à 8 caractères, lettre ou national en tête.
fn is_dd_name(s: &str) -> bool {
    let b = s.as_bytes();
    (1..=8).contains(&b.len())
        && (b[0].is_ascii_uppercase() || matches!(b[0], b'$' | b'#' | b'@'))
        && b.iter().all(|c| {
            c.is_ascii_uppercase() || c.is_ascii_digit() || matches!(c, b'$' | b'#' | b'@')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telechargement_zowe() {
        let src = DdSource::from_path(r"C:\corpus\sysout\JOB00823\JES2\JESMSGLG.txt");
        assert_eq!(src.job_id.as_deref(), Some("JOB00823"));
        assert_eq!(src.dd_name.as_deref(), Some("JESMSGLG"));

        let src = DdSource::from_path("corpus/sysout/JOB00896/LKED/COBRUN/SYSPRINT(1).txt");
        assert_eq!(src.job_id.as_deref(), Some("JOB00896"));
        assert_eq!(src.dd_name.as_deref(), Some("SYSPRINT"));
    }

    #[test]
    fn document_zowe() {
        let src = DdSource::from_path("/zosmf/JOB00922/JRIDC12.JOB00922.JES2.JESMSGLG.2");
        assert_eq!(src.job_id.as_deref(), Some("JOB00922"));
        assert_eq!(src.job_name.as_deref(), Some("JRIDC12"));
        assert_eq!(src.dd_name.as_deref(), Some("JESMSGLG"));

        let src = DdSource::from_path("Z99999.TSU00812.JES2.JESYSMSG.4");
        assert_eq!(src.job_id.as_deref(), Some("TSU00812"));
        assert_eq!(src.dd_name.as_deref(), Some("JESYSMSG"));
    }

    #[test]
    fn chemin_inconnu() {
        let src = DdSource::from_path("rapport final.txt");
        assert_eq!(src, DdSource::default());
    }

    #[test]
    fn identifiants_jes() {
        for id in ["JOB00823", "TSU00812", "STC01234", "J0123456"] {
            assert!(is_job_id(id), "{id}");
        }
        for id in ["JOB0082", "STEP0100", "JES2", "JOBABCDE"] {
            assert!(!is_job_id(id), "{id}");
        }
    }

    #[test]
    fn detection_par_contenu() {
        let msglg = "1                   J E S 2  J O B  L O G  --  S Y S T E M  S0W1\r\n";
        assert_eq!(DdKind::sniff(msglg), Some(DdKind::JesMsgLg));
        assert_eq!(
            DdKind::sniff("  STMT NO. MESSAGE\n"),
            Some(DdKind::JesYsMsg)
        );
        assert_eq!(
            DdKind::sniff("        1 //@VSC1     JOB ,MSGLEVEL=(0,0)\n"),
            Some(DdKind::JesJcl)
        );
        assert_eq!(DdKind::sniff("1READY\n"), None);
    }
}
