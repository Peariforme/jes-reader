//! JESYSMSG : messages de conversion (numérotés), d'allocation et de fin
//! de step.
//!
//! ```text
//!   STMT NO. MESSAGE
//!          2 IEFC001I PROCEDURE ZXP$$CHK WAS EXPANDED USING SYSTEM LIBRARY …
//!  IEF142I @VSC1 VALIDATE - STEP WAS EXECUTED - COND CODE 0000
//!          CPU:     0 HR  00 MIN  00.01 SEC    SRB:     0 HR  00 MIN …
//! ```
//!
//! Un message commence en col. 2 (offset 1), ou en col. 12 (offset 11)
//! derrière un numéro d'instruction cadré en col. 2-10. Toute autre ligne
//! prolonge le message précédent, y compris les suites qui commencent en
//! col. 2 (`IGD17101I`, `IEC141I`).

use super::{Messages, StepNames, value_after};
use crate::model::{Dd, Message, Step, StepStatus};
use crate::msgid::message_id_at;

pub(super) fn parse(lines: &[&str], dd: &mut Dd) {
    let mut messages = Messages::new();
    for (i, line) in lines.iter().enumerate() {
        if line.trim_start().starts_with("STMT NO. MESSAGE") || line.trim().is_empty() {
            messages.close();
        } else if let Some(id) = message_id_at(line, 1) {
            messages.start(i, line, id, None);
        } else if let Some((statement, id)) = numbered_message(line) {
            messages.start(i, line, id, Some(statement));
        } else {
            messages.extend(i);
        }
    }
    dd.messages = messages.finish();
    dd.steps = steps(&dd.messages, &mut dd.job_name);
}

/// `         2 IEFC001I …` : numéro d'instruction en col. 2-10, identifiant
/// en col. 12.
fn numbered_message(line: &str) -> Option<(u32, std::ops::Range<usize>)> {
    let number = line.get(1..10)?.trim_start().parse().ok()?;
    if line.as_bytes().get(10) != Some(&b' ') {
        return None;
    }
    Some((number, message_id_at(line, 11)?))
}

/// Messages qui précèdent les steps ou les suivent, sans appartenir à aucun.
fn is_job_level(m: &Message) -> bool {
    m.statement.is_some()
        || matches!(
            m.id.as_str(),
            "ICH70001I" | "IEFA111I" | "IEF375I" | "IEF033I"
        )
}

/// Ce qu'on sait d'un step en cours de lecture.
#[derive(Default)]
struct Pending {
    start: Option<usize>,
    name: Option<(String, Option<String>)>,
    /// Vrai si le nom vient d'un message de fin (`IEF142I`…) et non d'un
    /// message d'allocation ou d'horodatage.
    name_is_final: bool,
    status: Option<(StepStatus, usize)>,
}

impl Pending {
    fn set_name(&mut self, names: &StepNames<'_>, is_final: bool) {
        if self.name.is_none() || (is_final && !self.name_is_final) {
            self.name = Some((names.step.to_owned(), names.proc_step.map(str::to_owned)));
            self.name_is_final = is_final;
        }
    }

    fn set_status(&mut self, status: StepStatus, line: usize) {
        // Un premier statut définitif l'emporte : `IEF202I` puis `IEF272I`
        // décrivent le même step non exécuté.
        if self.status.is_none() {
            self.status = Some((status, line));
        }
    }

    fn into_step(self, end: usize) -> Option<Step> {
        let start = self.start?;
        let (name, proc_step) = self.name?;
        let (status, status_line) = self.status.unwrap_or((StepStatus::Unknown, start));
        Some(Step {
            name,
            proc_step,
            status,
            lines: start..end,
            status_line,
        })
    }
}

/// Découpe les messages en steps. Un step va du premier message qui suit le
/// step précédent jusqu'à son `IEF032I` (fin de step, avec ses lignes CPU et
/// mémoire). Son nom et son issue viennent de `IEF142I`, `IEF472I`,
/// `IEF202I`, `IEF272I`, à défaut de `IEF236I` (allocation) ou `IEF373I`.
fn steps(messages: &[Message], job_name: &mut Option<String>) -> Vec<Step> {
    let mut steps = Vec::new();
    let mut pending = Pending::default();
    let mut last_end = 0;

    for m in messages {
        if is_job_level(m) {
            continue;
        }
        pending.start.get_or_insert(m.lines.start);
        last_end = m.lines.end;
        let line = m.lines.start;

        match m.id.as_str() {
            "IEF142I" | "IEF472I" | "IEF202I" | "IEF272I" => {
                let Some(names) = StepNames::parse(&m.text) else {
                    continue;
                };
                if job_name.is_none() {
                    *job_name = Some(names.job.to_owned());
                }
                pending.set_name(&names, true);
                let status = match m.id.as_str() {
                    "IEF142I" => value_after(&m.text, "COND CODE ")
                        .and_then(|rc| rc.parse().ok())
                        .map(|return_code| StepStatus::Completed { return_code }),
                    "IEF472I" => abend(&m.text),
                    _ => Some(StepStatus::NotRun),
                };
                if let Some(status) = status {
                    pending.set_status(status, line);
                }
            }
            "IEF236I" => {
                // `ALLOC. FOR JRIGZ COBOL COBRUN` : pas de ` - `.
                if let Some(names) = m
                    .text
                    .strip_prefix("ALLOC. FOR ")
                    .and_then(StepNames::from_names)
                {
                    if job_name.is_none() {
                        *job_name = Some(names.job.to_owned());
                    }
                    pending.set_name(&names, false);
                }
            }
            "IEF373I" => {
                // `STEP/GO      /START …` : le nom le plus interne seulement.
                if pending.name.is_none()
                    && let Some(name) = m
                        .text
                        .strip_prefix("STEP/")
                        .and_then(|t| t.split('/').next())
                {
                    pending.name = Some((name.trim().to_owned(), None));
                }
            }
            "IEF032I" => {
                steps.extend(std::mem::take(&mut pending).into_step(m.lines.end));
            }
            _ => {}
        }
    }
    // Step en cours (job actif) ou fin de step absente.
    steps.extend(pending.into_step(last_end));
    steps
}

/// `COMPLETION CODE - SYSTEM=0C7 USER=0000 REASON=00000007` : abend système
/// si `SYSTEM` est non nul, abend utilisateur sinon.
fn abend(text: &str) -> Option<StepStatus> {
    let system = value_after(text, "SYSTEM=")?;
    let user = value_after(text, "USER=")?;
    let code = if system.bytes().all(|c| c == b'0') {
        format!("U{user}")
    } else {
        format!("S{system}")
    };
    // Une raison nulle (`REASON=00000000`, S322) n'apporte rien.
    let reason = value_after(text, "REASON=").filter(|r| !r.bytes().all(|c| c == b'0'));
    Some(StepStatus::Abended {
        code,
        reason: reason.map(str::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_numerote() {
        let line = "         2 IEFC001I PROCEDURE ZXP$$CHK WAS EXPANDED";
        assert_eq!(numbered_message(line), Some((2, 11..19)));
        assert_eq!(numbered_message(" IEF142I @VSC1"), None);
    }

    #[test]
    fn codes_abend() {
        let abend = |t| match abend(t) {
            Some(StepStatus::Abended { code, reason }) => (code, reason),
            other => panic!("{other:?}"),
        };
        assert_eq!(
            abend("COMPLETION CODE - SYSTEM=0C7 USER=0000 REASON=00000007"),
            ("S0C7".into(), Some("00000007".into()))
        );
        assert_eq!(
            abend("COMPLETION CODE - SYSTEM=000 USER=4038 REASON=00000001"),
            ("U4038".into(), Some("00000001".into()))
        );
    }
}
