//! Parsing d'une DD. Le parser ne plante jamais : ce qu'il ne reconnaît
//! pas reste du texte brut, sans message ni step.

mod jcl;
mod msglg;
mod other;
mod sysmsg;

use std::ops::Range;

use crate::model::{Dd, DdKind, Message, Severity};
use crate::source::{DdSource, jes_dd_name};

/// Analyse le contenu d'un document dont on connaît le chemin (ou l'URI).
pub fn parse_file(path: &str, text: &str) -> Dd {
    parse_dd(&DdSource::from_path(path), text)
}

/// Analyse une DD. Le nom de DD de `source` décide de la façon de la lire ;
/// à défaut, le contenu (en-têtes JES2).
pub fn parse_dd(source: &DdSource, text: &str) -> Dd {
    let by_name = source
        .dd_name
        .as_deref()
        .map_or(DdKind::Other, DdKind::from_dd_name);
    let kind = match by_name {
        DdKind::Other => DdKind::sniff(text).unwrap_or(DdKind::Other),
        kind => kind,
    };
    let lines: Vec<&str> = text.lines().collect();
    let has_asa = match kind {
        DdKind::Other => detect_asa(&lines),
        _ => true,
    };
    let mut dd = Dd {
        name: source
            .dd_name
            .clone()
            .or_else(|| jes_dd_name(kind).map(str::to_owned)),
        kind,
        job_name: source.job_name.clone(),
        job_id: source.job_id.clone(),
        has_asa,
        line_count: lines.len(),
        messages: Vec::new(),
        steps: Vec::new(),
        statements: Vec::new(),
        job_end: None,
    };
    match kind {
        DdKind::JesMsgLg => msglg::parse(&lines, &mut dd),
        DdKind::JesJcl => jcl::parse(&lines, &mut dd),
        DdKind::JesYsMsg => sysmsg::parse(&lines, &mut dd),
        DdKind::Other => other::parse(&lines, &mut dd),
    }
    dd
}

/// Une DD porte un caractère ASA si chaque ligne non vide commence par
/// l'un des caractères de contrôle ` `, `0`, `-`, `+` ou `1`.
fn detect_asa(lines: &[&str]) -> bool {
    let mut any = false;
    for line in lines.iter().filter(|l| !l.is_empty()) {
        if !matches!(line.as_bytes()[0], b' ' | b'0' | b'-' | b'+' | b'1') {
            return false;
        }
        any = true;
    }
    any
}

/// Accumule les messages d'une DD et gère leurs lignes de suite.
struct Messages {
    messages: Vec<Message>,
    open: bool,
}

impl Messages {
    fn new() -> Self {
        Self {
            messages: Vec::new(),
            open: false,
        }
    }

    /// Ouvre un message dont l'identifiant occupe `id_columns` en ligne `line_no`.
    fn start(
        &mut self,
        line_no: usize,
        line: &str,
        id_columns: Range<usize>,
        statement: Option<u32>,
    ) {
        let id = line[id_columns.clone()].to_owned();
        self.messages.push(Message {
            severity: Severity::from_message_id(&id),
            id,
            lines: line_no..line_no + 1,
            text: line[id_columns.end..].trim().to_owned(),
            id_columns,
            statement,
        });
        self.open = true;
    }

    /// Rattache la ligne `line_no` au message ouvert, s'il y en a un.
    fn extend(&mut self, line_no: usize) {
        if self.open
            && let Some(last) = self.messages.last_mut()
        {
            last.lines.end = line_no + 1;
        }
    }

    /// Ferme le message en cours : les lignes suivantes ne s'y rattachent plus.
    fn close(&mut self) {
        self.open = false;
    }

    fn finish(self) -> Vec<Message> {
        self.messages
    }
}

/// Noms d'un step dans un message `<job> [<procstep>] <step> - …`.
struct StepNames<'a> {
    job: &'a str,
    step: &'a str,
    proc_step: Option<&'a str>,
}

impl<'a> StepNames<'a> {
    /// Lit les noms avant le premier ` - `, qui doit être présent.
    fn parse(text: &'a str) -> Option<Self> {
        Self::from_names(text.split_once(" - ")?.0)
    }

    /// Deux noms : job et step ; trois : job, step de procédure, step du job.
    fn from_names(head: &'a str) -> Option<Self> {
        let names: Vec<&str> = head.split_whitespace().collect();
        match names[..] {
            [job, step] => Some(Self {
                job,
                step,
                proc_step: None,
            }),
            [job, proc_step, step] => Some(Self {
                job,
                step,
                proc_step: Some(proc_step),
            }),
            _ => None,
        }
    }
}

/// Valeur qui suit `key` dans `text`, jusqu'à la prochaine espace.
fn value_after<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let start = text.find(key)? + key.len();
    let rest = &text[start..];
    Some(rest.split_whitespace().next().unwrap_or(""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asa() {
        assert!(detect_asa(&["1READY", " %CMD", "", "0FIN"]));
        assert!(!detect_asa(&["         MACRO", "*  commentaire"]));
        assert!(!detect_asa(&["", ""]));
    }

    #[test]
    fn noms_de_step() {
        let n = StepNames::parse("JRS0C7 COBOL COBRUN - STEP WAS EXECUTED").unwrap();
        assert_eq!(
            (n.job, n.proc_step, n.step),
            ("JRS0C7", Some("COBOL"), "COBRUN")
        );
        let n = StepNames::parse("JRIDC8 STEP020 - STEP WAS NOT EXECUTED.").unwrap();
        assert_eq!((n.job, n.proc_step, n.step), ("JRIDC8", None, "STEP020"));
        assert!(StepNames::parse("PAS DE TIRET").is_none());
    }

    #[test]
    fn valeurs() {
        let t = "COMPLETION CODE - SYSTEM=0C7 USER=0000 REASON=00000007";
        assert_eq!(value_after(t, "SYSTEM="), Some("0C7"));
        assert_eq!(value_after(t, "REASON="), Some("00000007"));
        assert_eq!(value_after(t, "ABSENT="), None);
    }

    #[test]
    fn ne_plante_jamais() {
        for text in ["", "\r\n", "1", " ", "\u{0}\u{1}", "é è à", "        1 //"] {
            for name in ["JESMSGLG", "JESJCL", "JESYSMSG", "SYSPRINT"] {
                let source = DdSource {
                    dd_name: Some(name.into()),
                    ..DdSource::default()
                };
                parse_dd(&source, text);
            }
        }
    }
}
