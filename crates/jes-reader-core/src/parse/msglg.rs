//! JESMSGLG: timestamped messages, step table, end of job.
//!
//! ```text
//!  04.12.28 JOB00823  IRR010I  USERID Z99999   IS ASSIGNED TO THIS JOB.
//!  04.51.14 JOB00896  -COBRUN   GO       *S0C7    282     44 …
//! ```
//!
//! Time in columns 2-9, JES job ID in columns 11-18, message or step table
//! row from column 21 (offset 20).

use super::{Messages, value_after};
use crate::model::{Dd, JobEnd, Step, StepStatus};
use crate::msgid::message_id_at;
use crate::source::is_job_id;

/// Offset where the message starts in a timestamped line.
const BODY: usize = 20;

pub(super) fn parse(lines: &[&str], dd: &mut Dd) {
    let mut messages = Messages::new();
    let mut in_step_table = false;

    for (i, line) in lines.iter().enumerate() {
        let Some(job_id) = timed_job_id(line) else {
            // Banner, JES2 statistics: not messages.
            messages.close();
            continue;
        };
        if dd.job_id.is_none() {
            dd.job_id = Some(job_id.to_owned());
        }

        if let Some(id) = message_id_at(line, BODY) {
            messages.start(i, line, id, None);
            continue;
        }
        match line.as_bytes().get(BODY) {
            Some(b' ') => messages.extend(i, &line[BODY..]),
            Some(b'-') => {
                messages.close();
                let row = &line[BODY + 1..];
                if row.starts_with("STEPNAME ") {
                    in_step_table = true;
                } else if row.contains(" ENDED.  NAME-") {
                    in_step_table = false;
                } else if in_step_table && let Some(step) = step_row(line, i) {
                    dd.steps.push(step);
                }
            }
            _ => messages.close(),
        }
    }

    dd.messages = messages.finish();
    read_job(dd);
}

/// JES job ID of a timestamped line (` hh.mm.ss JOBnnnnn  …`).
fn timed_job_id(line: &str) -> Option<&str> {
    let b = line.as_bytes();
    let time = b.get(1..9)?;
    let is_time = time.iter().enumerate().all(|(k, c)| {
        if k % 3 == 2 {
            *c == b'.'
        } else {
            c.is_ascii_digit()
        }
    });
    if !is_time || b.get(9) != Some(&b' ') {
        return None;
    }
    let job_id = line.get(10..18)?;
    is_job_id(job_id).then_some(job_id)
}

/// Step table row:
/// `-STEPNAME PROCSTEP    RC` in columns 22-29, 31-38 and 39-44 (offsets 21, 30, 38).
fn step_row(line: &str, line_no: usize) -> Option<Step> {
    // An unnamed step (`// EXEC` without a label) has a blank STEPNAME.
    let name = line.get(21..29)?.trim();
    let proc_step = line.get(30..38).map(str::trim).filter(|s| !s.is_empty());
    let rc = line.get(38..44)?.trim();
    if rc.is_empty() {
        return None;
    }
    // System abend with a leading asterisk (`*S0C7`), user abend without
    // (`U4038`).
    let abend = rc.strip_prefix('*').unwrap_or(rc);
    let status = if rc == "FLUSH" {
        StepStatus::NotRun
    } else if abend.starts_with(['S', 'U']) && abend.len() > 1 {
        StepStatus::Abended {
            code: abend.to_owned(),
            reason: None,
        }
    } else {
        StepStatus::Completed {
            return_code: rc.parse().ok()?,
        }
    };
    Some(Step {
        name: name.to_owned(),
        proc_step: proc_step.map(str::to_owned),
        status,
        lines: line_no..line_no + 1,
        status_line: line_no,
    })
}

/// Job name and end of job, from `$HASP373`, `$HASP395` and `IEF453I`.
fn read_job(dd: &mut Dd) {
    let mut jcl_error = false;
    let mut hasp395 = None;
    for m in &dd.messages {
        match m.id.as_str() {
            "$HASP373" | "$HASP395" => {
                if dd.job_name.is_none() {
                    dd.job_name = m.text.split_whitespace().next().map(str::to_owned);
                }
                if m.id == "$HASP395" {
                    hasp395 = Some(m.text.as_str());
                }
            }
            "IEF453I" => jcl_error = true,
            _ => {}
        }
    }
    dd.job_end = match hasp395 {
        _ if jcl_error => Some(JobEnd::JclError),
        Some(text) => Some(
            if let Some(rc) = value_after(text, "RC=").and_then(|v| v.parse().ok()) {
                JobEnd::ReturnCode(rc)
            } else if let Some(code) = value_after(text, "ABEND=") {
                JobEnd::Abend(code.to_owned())
            } else {
                JobEnd::Ended
            },
        ),
        None => None,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamped_line() {
        assert_eq!(
            timed_job_id(" 04.12.28 JOB00823  IRR010I  USERID"),
            Some("JOB00823")
        );
        assert_eq!(
            timed_job_id(" 04.08.22 TSU00812  $HASP373"),
            Some("TSU00812")
        );
        assert_eq!(timed_job_id("0------ JES2 JOB STATISTICS ------"), None);
        assert_eq!(timed_job_id(" 04:12:28 JOB00823"), None);
    }

    #[test]
    fn step_table() {
        let row = |s: &str| step_row(s, 0).map(|step| (step.display_name(), step.status));
        assert_eq!(
            row(" 04.51.14 JOB00896  -COBRUN   GO       *S0C7    282     44"),
            Some((
                "COBRUN.GO".into(),
                StepStatus::Abended {
                    code: "S0C7".into(),
                    reason: None
                }
            ))
        );
        assert_eq!(
            row(" 04.42.18 JOB00868  -STEP010              08     12      1"),
            Some(("STEP010".into(), StepStatus::Completed { return_code: 8 }))
        );
        assert_eq!(
            row(" 04.42.18 JOB00868  -STEP020           FLUSH      0      0"),
            Some(("STEP020".into(), StepStatus::NotRun))
        );
        assert_eq!(
            row(" 04.49.02 JOB00885  -COBRUN   GO       U4038    292     50"),
            Some((
                "COBRUN.GO".into(),
                StepStatus::Abended {
                    code: "U4038".into(),
                    reason: None
                }
            ))
        );
        assert_eq!(row(" 04.42.18 JOB00868  -"), None);
        // Unnamed step: blank STEPNAME column.
        assert_eq!(
            row(" 10.23.01 JOB01967  -                     00      6      0"),
            Some((
                "(unnamed step)".into(),
                StepStatus::Completed { return_code: 0 }
            ))
        );
    }
}
