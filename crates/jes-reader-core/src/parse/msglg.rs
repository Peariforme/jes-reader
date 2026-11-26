//! JESMSGLG: timestamped messages, step table, end of job.
//!
//! ```text
//!  04.12.28 JOB00823  IRR010I  USERID Z99999   IS ASSIGNED TO THIS JOB.
//!  04.51.14 JOB00896  -COBRUN   GO       *S0C7    282     44 …
//! ```
//!
//! Time in columns 2-9, JES job ID in columns 11-18, message or step table
//! row from column 21 (offset 20).

use super::{Messages, minutes, value_after};
use crate::model::{Dd, JobEnd, Step, StepStatus};
use crate::msgid::message_id_at;
use crate::source::is_job_id;

/// Offset where the message starts in a timestamped line.
const BODY: usize = 20;

pub(super) fn parse(lines: &[&str], dd: &mut Dd) {
    let mut messages = Messages::new();
    // Columns of the step table being read, from its header line.
    let mut table: Option<Table> = None;

    for (i, line) in lines.iter().enumerate() {
        let Some(job_id) = timed_job_id(line) else {
            // Banner, JES2 statistics: not messages.
            messages.close();
            continue;
        };
        if dd.job_id.is_none() {
            dd.job_id = Some(job_id.to_owned());
        }

        // `+WLM644E`: message written to the console by the program itself.
        let id = message_id_at(line, BODY).or_else(|| {
            (line.as_bytes().get(BODY) == Some(&b'+'))
                .then(|| message_id_at(line, BODY + 1))
                .flatten()
        });
        if let Some(id) = id {
            messages.start(i, line, id, None);
            continue;
        }
        match line.as_bytes().get(BODY) {
            Some(b' ') => messages.extend(i, &line[BODY..]),
            Some(b'-') => {
                messages.close();
                let row = &line[BODY + 1..];
                if row.contains("STEPNAME") && row.contains("PROCSTEP") {
                    table = Table::from_header(line);
                } else if row.contains(" ENDED.  NAME-") {
                    table = None;
                } else if let Some(table) = &table
                    && let Some(step) = table.step(line, i)
                {
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

/// Column just after `label` in the step table header.
fn header_end(header: &str, label: &str) -> Option<usize> {
    let start = header
        .find(&format!("{label} "))
        .or_else(|| header.ends_with(label).then(|| header.len() - label.len()))?;
    Some(start + label.len())
}

/// Word of `line` that ends exactly at column `end` (right-aligned value).
fn value_ending_at(line: &str, end: usize) -> Option<&str> {
    let head = line.get(..end)?;
    if line.as_bytes().get(end).is_some_and(|c| *c != b' ') {
        return None;
    }
    head.split(' ').next_back().filter(|w| !w.is_empty())
}

/// Columns of a step table, read from its header. The layout belongs to
/// the installation (IEFACTRT exit):
///
/// ```text
/// -STEPNAME PROCSTEP    RC   EXCP   CONN       TCB       SRB  CLOCK …
/// -JOBNAME  STEPNAME PROCSTEP    RC   EXCP    CPU    SRB  CLOCK   SERV …
/// ```
///
/// Names are left-aligned under their header, numbers right-aligned.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Table {
    step: usize,
    proc_step: usize,
    /// Column just after the `RC` header.
    rc_end: usize,
    /// Column just after `TCB` (or `CPU`), and after `CLOCK`.
    cpu_end: Option<usize>,
    clock_end: Option<usize>,
}

impl Table {
    fn from_header(header: &str) -> Option<Self> {
        Some(Self {
            step: header.find("STEPNAME")?,
            proc_step: header.find("PROCSTEP")?,
            rc_end: header_end(header, " RC")?,
            cpu_end: header_end(header, " TCB").or_else(|| header_end(header, " CPU")),
            clock_end: header_end(header, " CLOCK"),
        })
    }

    /// Step of a table row. The return code is a right-aligned word:
    /// `08`, `FLUSH` for a step that did not run, `*S0C7` for a system
    /// abend (leading asterisk), `U4038` for a user abend.
    fn step(&self, line: &str, line_no: usize) -> Option<Step> {
        let rc = value_ending_at(line, self.rc_end)?;
        let rc_start = self.rc_end - rc.len();
        // A name stops where the return code starts (`FLUSH` is written
        // under PROCSTEP in some tables).
        let field = |start: usize| -> &str {
            line.get(start..(start + 8).min(rc_start))
                .map_or("", str::trim)
        };
        // An unnamed step (`// EXEC` without a label) has a blank STEPNAME.
        let mut name = field(self.step);
        let mut proc_step = Some(field(self.proc_step)).filter(|s| !s.is_empty());
        // A name under PROCSTEP alone is the step name: there is no
        // procedure step without a job step.
        if name.is_empty()
            && let Some(alone) = proc_step.take()
        {
            name = alone;
        }
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
        let time =
            |end: Option<usize>| end.and_then(|e| value_ending_at(line, e)).and_then(minutes);
        Some(Step {
            name: name.to_owned(),
            proc_step: proc_step.map(str::to_owned),
            status,
            lines: line_no..line_no + 1,
            status_line: line_no,
            program: None,
            cpu: time(self.cpu_end),
            elapsed: time(self.clock_end),
            elapsed_in_minutes: false,
        })
    }
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
    fn program_messages() {
        let lines = [
            " 04.42.18 JOB00868  +WLM644E ******* PRODUCT 14 DAY GRACE EXPIRED",
            " 04.42.18 JOB00868  IEF403I JRIDC8 - STARTED - TIME=04.42.18",
        ];
        let mut dd = crate::parse_file("JOB00868/JES2/JESMSGLG.txt", "");
        parse(&lines, &mut dd);
        let ids: Vec<_> = dd
            .messages
            .iter()
            .map(|m| (m.id.as_str(), m.id_columns.clone()))
            .collect();
        assert_eq!(ids, [("WLM644E", 21..28), ("IEF403I", 20..27)]);
    }

    #[test]
    fn step_table_timings() {
        let text = [
            " 04.51.14 JOB00896  -STEPNAME PROCSTEP    RC   EXCP   CONN       TCB       SRB  CLOCK          SERV  WORKLOAD  PAGE  SWAP   VIO SWAPS",
            " 04.51.14 JOB00896  -COBRUN   COBOL       00  16874    281       .01       .00     .3            68  BATCH        1     0     0     0",
            " 04.42.18 JOB00868  -STEP020           FLUSH      0      0       .00       .00     .0                BATCH        0     0     0     0",
        ];
        let mut dd = crate::parse_file("JOB00896/JES2/JESMSGLG.txt", "");
        parse(&text, &mut dd);
        let timings: Vec<_> = dd.steps.iter().map(|s| (s.cpu, s.elapsed)).collect();
        use std::time::Duration;
        assert_eq!(
            timings,
            [
                (
                    Some(Duration::from_millis(600)),
                    Some(Duration::from_secs(18))
                ),
                (Some(Duration::ZERO), Some(Duration::ZERO)),
            ]
        );
    }

    /// Layout seen at a client site: JOBNAME first, CPU instead of TCB, the
    /// step name of a plain step under PROCSTEP. Values from the corpus.
    #[test]
    fn step_table_with_job_name() {
        let text = [
            " 04.42.18 JOB00868  -                                         --TIMINGS (MINS.)--            ----PAGING COUNTS---",
            " 04.42.18 JOB00868  -JOBNAME  STEPNAME PROCSTEP    RC   EXCP    CPU    SRB  CLOCK   SERV  PG   PAGE   SWAP    VIO SWAPS STEPNO",
            " 04.42.18 JOB00868  -JRIDC8            STEP005     04     32    .01    .00    .02     5   0      0      0      0     0      1",
            " 04.42.18 JOB00868  -JRIDC8            STEP010     08     12    .00    .00    .00     1   0      0      0      0     0      2",
            " 04.42.18 JOB00868  IEF404I JRIDC8 - ENDED - TIME=04.42.18",
            " 04.42.18 JOB00868  -JRIDC8   COBRUN   GO       *S0C7    282    .00    .00    .00   101   0      0      0      0     0      3",
            " 04.42.18 JOB00868  -JRIDC8   ENDED.  NAME-JES-READER IDCAMS    TOTAL CPU TIME=   .01  TOTAL ELAPSED TIME=   .02",
        ];
        let mut dd = crate::parse_file("JOB00868/JES2/JESMSGLG.txt", "");
        parse(&text, &mut dd);
        let steps: Vec<_> = dd
            .steps
            .iter()
            .map(|s| (s.display_name(), s.status.clone(), s.cpu, s.elapsed))
            .collect();
        use std::time::Duration;
        assert_eq!(
            steps,
            [
                (
                    "STEP005".to_owned(),
                    StepStatus::Completed { return_code: 4 },
                    Some(Duration::from_millis(600)),
                    Some(Duration::from_millis(1200)),
                ),
                (
                    "STEP010".to_owned(),
                    StepStatus::Completed { return_code: 8 },
                    Some(Duration::ZERO),
                    Some(Duration::ZERO),
                ),
                (
                    "COBRUN.GO".to_owned(),
                    StepStatus::Abended {
                        code: "S0C7".into(),
                        reason: None
                    },
                    Some(Duration::ZERO),
                    Some(Duration::ZERO),
                ),
            ]
        );
        assert!(dd.messages.iter().any(|m| m.id == "IEF404I"));
    }

    #[test]
    fn step_table() {
        let table = Table::from_header(
            " 04.51.14 JOB00896  -STEPNAME PROCSTEP    RC   EXCP   CONN       TCB       SRB  CLOCK          SERV",
        )
        .unwrap();
        let row = |s: &str| {
            table
                .step(s, 0)
                .map(|step| (step.display_name(), step.status))
        };
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
