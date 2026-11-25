//! `jes-reader summary`: one block per job, its steps and its failures.

use std::fmt::Write as _;
use std::path::Path;

use jes_reader_core::{
    DdKind, Finding, FindingSeverity, JobEnd, Options, Step, StepStatus, analyze, duration_text,
};
use serde_json::{Value, json};

use crate::files::File;

/// What is known about a job from its files.
pub struct Job<'a> {
    pub name: Option<String>,
    pub id: Option<String>,
    pub end: Option<JobEnd>,
    pub files: Vec<&'a Path>,
    /// Steps of JESYSMSG (with abend reasons and CPU), with the elapsed time
    /// of the JESMSGLG step table when it is more precise; otherwise the
    /// JESMSGLG steps.
    pub steps: Vec<Step>,
    /// Errors and warnings, most severe first, each reported once even when
    /// several DDs show it (step table of JESMSGLG and JESYSMSG).
    pub findings: Vec<(&'a Path, Finding)>,
}

/// Groups the files by JES job ID (a file without one is a job of its own),
/// in the order of the files.
pub fn jobs<'a>(files: &'a [File], options: &Options) -> Vec<Job<'a>> {
    let mut groups: Vec<Vec<&File>> = Vec::new();
    for file in files {
        let same_job = file.dd.job_id.as_ref().and_then(|id| {
            groups
                .iter_mut()
                .find(|g| g[0].dd.job_id.as_ref() == Some(id))
        });
        match same_job {
            Some(group) => group.push(file),
            None => groups.push(vec![file]),
        }
    }
    groups.into_iter().map(|g| job(&g, options)).collect()
}

fn job<'a>(files: &[&'a File], options: &Options) -> Job<'a> {
    let of_kind = |kind| files.iter().find(|f| f.dd.kind == kind).map(|f| &f.dd);
    let sysmsg = of_kind(DdKind::JesYsMsg);
    let msglg = of_kind(DdKind::JesMsgLg);

    let mut steps = sysmsg
        .or(msglg)
        .map(|dd| dd.steps.clone())
        .unwrap_or_default();
    if let (Some(_), Some(msglg)) = (sysmsg, msglg) {
        let same_steps = steps.len() == msglg.steps.len()
            && steps
                .iter()
                .zip(&msglg.steps)
                .all(|(a, b)| a.display_name() == b.display_name());
        if same_steps {
            for (step, row) in steps.iter_mut().zip(&msglg.steps) {
                if step.elapsed_in_minutes && row.elapsed.is_some() {
                    step.elapsed = row.elapsed;
                    step.elapsed_in_minutes = false;
                }
            }
        }
    }

    // JESYSMSG first: its findings are the most detailed (abend reasons).
    let mut ordered = files.to_vec();
    ordered.sort_by_key(|f| match f.dd.kind {
        DdKind::JesYsMsg => 0,
        DdKind::JesMsgLg => 1,
        DdKind::JesJcl => 2,
        DdKind::Other => 3,
    });
    let mut findings: Vec<(&Path, Finding)> = Vec::new();
    for file in ordered {
        let path = file.path.as_path();
        for finding in analyze(&file.dd, options) {
            let reported = findings.iter().any(|(p, f)| {
                same_failure(f, &finding) && (finding.columns.is_none() || *p != path)
            });
            if finding.severity != FindingSeverity::Information && !reported {
                findings.push((path, finding));
            }
        }
    }
    findings.sort_by_key(|(_, f)| f.severity);

    Job {
        name: files.iter().find_map(|f| f.dd.job_name.clone()),
        id: files.iter().find_map(|f| f.dd.job_id.clone()),
        end: msglg.and_then(|dd| dd.job_end.clone()),
        files: {
            let mut paths: Vec<&Path> = files.iter().map(|f| f.path.as_path()).collect();
            paths.dedup();
            paths
        },
        steps,
        findings,
    }
}

/// True if two findings report the same failure: the same step outcome
/// (`Step COBRUN.GO: …` from JESYSMSG and from the JESMSGLG step table), or
/// the same message ID (`IEC141I` is in JESMSGLG, JESYSMSG and the Fault
/// Analyzer report). Messages are only compared across files.
fn same_failure(a: &Finding, b: &Finding) -> bool {
    fn step(f: &Finding) -> Option<&str> {
        f.columns
            .is_none()
            .then(|| f.message.split(':').next().unwrap_or(""))
    }
    a.code == b.code && step(a) == step(b)
}

fn end_text(end: Option<&JobEnd>) -> String {
    match end {
        Some(JobEnd::ReturnCode(rc)) => format!("ended, RC {rc}"),
        Some(JobEnd::Abend(code)) => format!("ended, abend {code}"),
        Some(JobEnd::JclError) => "JCL error".into(),
        Some(JobEnd::Ended) => "ended".into(),
        None => "end unknown".into(),
    }
}

fn outcome(status: &StepStatus) -> String {
    match status {
        StepStatus::Completed { return_code } => format!("RC {return_code}"),
        StepStatus::Abended {
            code,
            reason: Some(reason),
        } => format!("abend {code} reason {}", reason.trim_start_matches('0')),
        StepStatus::Abended { code, reason: None } => format!("abend {code}"),
        StepStatus::NotRun => "not run".into(),
        StepStatus::Unknown => "unknown".into(),
    }
}

/// One block per job: header line, step table, findings.
pub fn text(jobs: &[Job<'_>]) -> String {
    let mut out = String::new();
    for (i, job) in jobs.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let name = job.name.as_deref().unwrap_or("?");
        let id = job.id.as_deref().unwrap_or("?");
        writeln!(out, "{name} {id}: {}", end_text(job.end.as_ref())).unwrap();

        let rows: Vec<[String; 5]> = job
            .steps
            .iter()
            .map(|s| {
                [
                    s.display_name(),
                    s.program.clone().unwrap_or_else(|| "-".into()),
                    outcome(&s.status),
                    s.cpu.map_or_else(|| "-".into(), duration_text),
                    s.elapsed_text().unwrap_or_else(|| "-".into()),
                ]
            })
            .collect();
        if !rows.is_empty() {
            let header = ["STEP", "PROGRAM", "OUTCOME", "CPU", "ELAPSED"].map(str::to_owned);
            let mut widths = [0; 5];
            for row in std::iter::once(&header).chain(&rows) {
                for (w, cell) in widths.iter_mut().zip(row) {
                    *w = (*w).max(cell.chars().count());
                }
            }
            for row in std::iter::once(&header).chain(&rows) {
                let mut line = String::from(" ");
                for (w, cell) in widths.iter().zip(row) {
                    write!(line, " {cell:<w$} ").unwrap();
                }
                writeln!(out, "{}", line.trim_end()).unwrap();
            }
        }
        for (path, f) in &job.findings {
            let label = match f.severity {
                FindingSeverity::Error => "error",
                _ => "warning",
            };
            writeln!(
                out,
                "  {label}: {} ({}:{})",
                shorten(&f.message),
                path.display(),
                f.line + 1
            )
            .unwrap();
        }
    }
    out
}

/// Longest finding message in the text summary.
const MESSAGE_WIDTH: usize = 120;

fn shorten(message: &str) -> String {
    match message.char_indices().nth(MESSAGE_WIDTH) {
        Some((end, _)) => format!("{}…", message[..end].trim_end()),
        None => message.to_owned(),
    }
}

/// Every job, as JSON. Durations in seconds; lines counted from 1.
pub fn json(jobs: &[Job<'_>]) -> Value {
    let jobs: Vec<Value> = jobs
        .iter()
        .map(|job| {
            let end = match &job.end {
                Some(JobEnd::ReturnCode(rc)) => json!({ "status": "ended", "returnCode": rc }),
                Some(JobEnd::Abend(code)) => json!({ "status": "abend", "abend": code }),
                Some(JobEnd::JclError) => json!({ "status": "jclError" }),
                Some(JobEnd::Ended) => json!({ "status": "ended" }),
                None => Value::Null,
            };
            let steps: Vec<Value> = job.steps.iter().map(step_json).collect();
            let count = |severity| {
                job.findings
                    .iter()
                    .filter(|(_, f)| f.severity == severity)
                    .count()
            };
            let findings: Vec<Value> = job
                .findings
                .iter()
                .map(|(path, f)| {
                    json!({
                        "file": path.display().to_string(),
                        "line": f.line + 1,
                        "severity": match f.severity {
                            FindingSeverity::Error => "error",
                            _ => "warning",
                        },
                        "code": f.code,
                        "message": f.message,
                    })
                })
                .collect();
            json!({
                "name": job.name,
                "id": job.id,
                "end": end,
                "files": job.files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
                "steps": steps,
                "errors": count(FindingSeverity::Error),
                "warnings": count(FindingSeverity::Warning),
                "findings": findings,
            })
        })
        .collect();
    json!({ "jobs": jobs })
}

fn step_json(step: &Step) -> Value {
    let mut value = json!({
        "name": step.display_name(),
        "step": step.name,
        "procStep": step.proc_step,
        "program": step.program,
        "cpuSeconds": step.cpu.map(|d| d.as_secs_f64()),
        "elapsedSeconds": step.elapsed.map(|d| d.as_secs_f64()),
        "elapsedToTheMinute": step.elapsed.is_some() && step.elapsed_in_minutes,
    });
    match &step.status {
        StepStatus::Completed { return_code } => {
            value["status"] = json!("completed");
            value["returnCode"] = json!(return_code);
        }
        StepStatus::Abended { code, reason } => {
            value["status"] = json!("abended");
            value["abend"] = json!(code);
            value["reason"] = json!(reason);
        }
        StepStatus::NotRun => value["status"] = json!("notRun"),
        StepStatus::Unknown => value["status"] = json!("unknown"),
    }
    value
}
