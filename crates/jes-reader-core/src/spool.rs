//! Whole spool: every DD of a job in one file, one after the other, as
//! saved by BMC AMI DevX JES Explorer for example.
//!
//! The file is cut into DDs, each parsed as if it had been opened on its
//! own; line numbers still count from the top of the file
//! ([`Dd::first_line`]).
//!
//! JES2 writes the system DDs first, in this order, and each one is
//! recognized by its content:
//!
//! - JESMSGLG: its banner, `J E S 2  J O B  L O G`;
//! - JESJCL: its first statement, `        1 //NAME     JOB …`;
//! - JESYSMSG: its header, `STMT NO. MESSAGE`, or else the first line after
//!   JESJCL that is not JCL; it ends after `IEF033I` (end of job) and the
//!   lines that continue it.
//!
//! What follows is the output of the steps (`SYSPRINT`, `SYSOUT`…). Those DDs
//! have no signature and their names are not in the text: they are kept as
//! one DD of kind [`DdKind::Other`], without a name.
//!
//! BMC AMI DevX saves the DDs with their ASA column and no separator
//! between them (`<jobname>_<jobid>.jes`). A file without the ASA column
//! is read too, with a blank column added in front of each line.
//!
//! _To be confirmed on a real file_: separators or banners that the saving
//! tool may insert between DDs. Lines it adds before the next system DD stay
//! in the previous DD, which tolerates them as plain text.

use std::ops::Range;

use crate::model::{Dd, DdKind};
use crate::msgid::message_id_at;
use crate::parse::{parse_dd, parse_file, remove_first_column};
use crate::source::{DdSource, jes_dd_name};

/// DD found in a whole spool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    /// Kind of DD.
    pub kind: DdKind,
    /// Lines of the file it covers.
    pub lines: Range<usize>,
}

/// DDs of a document: the DDs of a whole spool, otherwise the document as
/// one DD.
pub fn parse_document(path: &str, text: &str) -> Vec<Dd> {
    parse_spool(path, text).unwrap_or_else(|| vec![parse_file(path, text)])
}

/// DDs of a whole spool, or `None` if the text is not one: a whole spool
/// holds at least two JES2 system DDs.
pub fn parse_spool(path: &str, text: &str) -> Option<Vec<Dd>> {
    let system = |parts: &[Part]| parts.iter().filter(|p| p.kind != DdKind::Other).count();
    let original: Vec<&str> = text.lines().collect();
    let mut parts = split(&original);
    // Saved without the ASA column: try again with a blank
    // column added in front of each line, and move the columns back after.
    let padded: Vec<String>;
    let mut lines = original;
    let mut without_asa = false;
    if system(&parts) < 2 {
        padded = lines.iter().map(|l| format!(" {l}")).collect();
        let padded_lines: Vec<&str> = padded.iter().map(String::as_str).collect();
        let padded_parts = split(&padded_lines);
        if system(&padded_parts) < 2 {
            return None;
        }
        (lines, parts, without_asa) = (padded_lines, padded_parts, true);
    }
    let source = DdSource::from_path(path);
    let dds = parts
        .into_iter()
        .map(|part| {
            let source = DdSource {
                dd_name: jes_dd_name(part.kind).map(str::to_owned),
                ..source.clone()
            };
            let mut dd = parse_dd(&source, &lines[part.lines.clone()].join("\n"));
            // The content decides, whatever the name of the file says.
            dd.name = source.dd_name;
            if without_asa {
                remove_first_column(&mut dd);
            }
            shift(&mut dd, part.lines.start);
            dd
        })
        .collect::<Vec<_>>();
    // The job ID is on the lines of JESMSGLG and the JOB card of JESJCL:
    // give it to the DDs that do not show it.
    let job_id = dds.iter().find_map(|dd| dd.job_id.clone());
    let job_name = dds.iter().find_map(|dd| dd.job_name.clone());
    Some(
        dds.into_iter()
            .map(|mut dd| {
                if dd.job_id.is_none() {
                    dd.job_id.clone_from(&job_id);
                }
                if dd.job_name.is_none() {
                    dd.job_name.clone_from(&job_name);
                }
                dd
            })
            .collect(),
    )
}

/// Cuts a whole spool into DDs. Lines before the first recognized DD form a
/// DD of kind `Other`.
pub fn split(lines: &[&str]) -> Vec<Part> {
    let mut parts: Vec<Part> = Vec::new();
    // JESYSMSG has reached `IEF033I`: only its continuation lines follow.
    let mut job_ended = false;

    for (i, line) in lines.iter().enumerate() {
        let current = parts.last().map(|p| p.kind);
        let jesysmsg_starts = (is_sysmsg_header(line) && current != Some(DdKind::JesYsMsg))
            // Without its header, JESYSMSG starts where the JCL stops.
            || (current == Some(DdKind::JesJcl) && !line.trim().is_empty() && !is_jcl_line(line));
        let next = if is_msglg_banner(line) {
            Some(DdKind::JesMsgLg)
        } else if is_job_card(line) && current != Some(DdKind::JesJcl) {
            Some(DdKind::JesJcl)
        } else if jesysmsg_starts {
            Some(DdKind::JesYsMsg)
        } else if current.is_none()
            || (current == Some(DdKind::JesYsMsg) && job_ended && !is_continuation(line))
        {
            Some(DdKind::Other)
        } else {
            None
        };
        if let Some(kind) = next {
            parts.push(Part { kind, lines: i..i });
            job_ended = false;
        }
        if parts.last().map(|p| p.kind) == Some(DdKind::JesYsMsg)
            && message_id_at(line, 1).is_some_and(|id| &line[id] == "IEF033I")
        {
            job_ended = true;
        }
        if let Some(part) = parts.last_mut() {
            part.lines.end = i + 1;
        }
    }
    parts
}

/// `1                   J E S 2  J O B  L O G  --  S Y S T E M …`
fn is_msglg_banner(line: &str) -> bool {
    line.contains("J E S 2  J O B  L O G")
}

/// `        1 //NAME     JOB …`: first statement of JESJCL. Column 1 (ASA)
/// may hold a page eject.
fn is_job_card(line: &str) -> bool {
    line.get(1..9).map(str::trim) == Some("1")
        && line.get(9..12) == Some(" //")
        && line.contains(" JOB ")
}

/// `  STMT NO. MESSAGE`
fn is_sysmsg_header(line: &str) -> bool {
    line.get(1..)
        .is_some_and(|l| l.trim_start().starts_with("STMT NO. MESSAGE"))
}

/// Line of JESJCL: text from column 11, after a statement number or a
/// blank number field (`        4 XXCOBOL …`, `          //  REGION=0M`,
/// `          IEFC653I SUBSTITUTION JCL - …` and its continuation lines).
/// JESYSMSG lines differ: a message from column 2, or a statement number
/// that reaches column 10 (`         2 IEFC001I …`).
fn is_jcl_line(line: &str) -> bool {
    let number = line.get(1..9).map(str::trim);
    number.is_some_and(|n| n.bytes().all(|c| c.is_ascii_digit()))
        && line.get(9..10) == Some(" ")
        && line.as_bytes().get(10).is_some_and(|c| *c != b' ')
}

/// Continuation of a JESYSMSG message: indented, after the ASA column
/// (`         CPU:     0 HR  00 MIN …`).
fn is_continuation(line: &str) -> bool {
    line.len() > 10 && line.get(1..9).is_some_and(|l| l.trim().is_empty())
}

/// Moves every line number of `dd` down by `offset`.
fn shift(dd: &mut Dd, offset: usize) {
    let range = |r: &mut Range<usize>| *r = r.start + offset..r.end + offset;
    dd.first_line = offset;
    for m in &mut dd.messages {
        range(&mut m.lines);
    }
    for step in &mut dd.steps {
        range(&mut step.lines);
        step.status_line += offset;
    }
    for st in &mut dd.statements {
        range(&mut st.lines);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Options, analyze};

    fn corpus(path: &str) -> String {
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus/sysout")
            .join(path);
        String::from_utf8_lossy(&std::fs::read(file).unwrap()).into_owned()
    }

    /// A whole spool made of corpus DDs, in spool order, and the line where
    /// each one starts.
    fn spool(job: &str, dds: &[&str]) -> (String, Vec<usize>) {
        let mut text = String::new();
        let mut starts = Vec::new();
        for dd in dds {
            starts.push(text.lines().count());
            text.push_str(&corpus(&format!("{job}/{dd}")));
            if !text.ends_with('\n') {
                text.push('\n');
            }
        }
        (text, starts)
    }

    #[test]
    fn splits_system_and_step_output() {
        let dds = [
            "JES2/JESMSGLG.txt",
            "JES2/JESJCL.txt",
            "JES2/JESYSMSG.txt",
            "STEP010/IDIREPRT.txt",
            "STEP010/SYSPRINT.txt",
        ];
        let (text, starts) = spool("JOB00901", &dds);
        let lines: Vec<&str> = text.lines().collect();
        let parts = split(&lines);
        let kinds: Vec<_> = parts.iter().map(|p| (p.kind, p.lines.start)).collect();
        // JESYSMSG starts without its header (no conversion message), the
        // step output stays one DD.
        assert_eq!(
            kinds,
            [
                (DdKind::JesMsgLg, starts[0]),
                (DdKind::JesJcl, starts[1]),
                (DdKind::JesYsMsg, starts[2]),
                (DdKind::Other, starts[3]),
            ]
        );
        assert_eq!(parts[3].lines.end, lines.len());
    }

    #[test]
    fn spool_reads_like_separate_dds() {
        let dds = ["JES2/JESMSGLG.txt", "JES2/JESJCL.txt", "JES2/JESYSMSG.txt"];
        let (text, starts) = spool("JOB00896", &dds);
        let parsed = parse_spool("JRS0C7.spool.txt", &text).unwrap();
        assert_eq!(parsed.len(), 3);
        for ((dd, path), start) in parsed.iter().zip(dds).zip(&starts) {
            let alone = parse_file(
                &format!("JOB00896/{path}"),
                &corpus(&format!("JOB00896/{path}")),
            );
            assert_eq!(dd.kind, alone.kind);
            assert_eq!(dd.first_line, *start);
            assert_eq!(dd.job_id.as_deref(), Some("JOB00896"));
            assert_eq!(dd.job_name.as_deref(), Some("JRS0C7"));
            let lines = |d: &Dd, offset: usize| -> Vec<(String, usize)> {
                analyze(d, &Options::default())
                    .into_iter()
                    .map(|f| (f.code, f.line - offset))
                    .collect()
            };
            assert_eq!(lines(dd, *start), lines(&alone, 0), "{path}");
            assert_eq!(dd.steps.len(), alone.steps.len());
        }
        // The JCL of the spool resolves the steps of the spool.
        let (sysmsg, jcl) = (&parsed[2], &parsed[1]);
        assert!(jcl.is_jcl_of(sysmsg));
        let exec = jcl.exec_statement(&sysmsg.steps[2]).unwrap();
        assert_eq!(exec.name.as_deref(), Some("GO"));
        assert!(exec.lines.start >= starts[1] && exec.lines.start < starts[2]);
    }

    /// Every corpus job, its system DDs then its step output (in path order).
    #[test]
    fn every_corpus_job() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/sysout");
        let mut jobs: Vec<_> = std::fs::read_dir(&root)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.is_dir())
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        jobs.sort();
        for job in jobs {
            let mut dds = Vec::new();
            let mut expected = Vec::new();
            for (name, kind) in [
                ("JESMSGLG", DdKind::JesMsgLg),
                ("JESJCL", DdKind::JesJcl),
                ("JESYSMSG", DdKind::JesYsMsg),
            ] {
                let path = format!("JES2/{name}.txt");
                if root.join(&job).join(&path).is_file() {
                    dds.push(path);
                    expected.push(kind);
                }
            }
            let mut others = Vec::new();
            collect(&root.join(&job), &root.join(&job), &mut others);
            others.sort();
            if !others.is_empty() {
                expected.push(DdKind::Other);
            }
            dds.extend(others);
            let refs: Vec<&str> = dds.iter().map(String::as_str).collect();
            let (text, starts) = spool(&job, &refs);
            let lines: Vec<&str> = text.lines().collect();
            let parts = split(&lines);
            let kinds: Vec<_> = parts.iter().map(|p| p.kind).collect();
            assert_eq!(kinds, expected, "{job}");
            for (part, start) in parts.iter().zip(&starts) {
                assert_eq!(part.lines.start, *start, "{job} {:?}", part.kind);
            }
        }
    }

    fn collect(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect(root, &path, out);
            } else if !path.starts_with(root.join("JES2")) {
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push(relative);
            }
        }
    }

    /// Without the ASA column: every line one column to the left, as BMC
    /// AMI DevX saves a whole spool.
    #[test]
    fn spool_without_asa() {
        let dds = [
            "JES2/JESMSGLG.txt",
            "JES2/JESJCL.txt",
            "JES2/JESYSMSG.txt",
            "STEP010/SYSPRINT.txt",
        ];
        let (text, starts) = spool("JOB00901", &dds);
        let stripped: String = text
            .lines()
            .map(|l| format!("{}\n", l.get(1..).unwrap_or("")))
            .collect();
        let with_asa = parse_spool("x.txt", &text).unwrap();
        let without = parse_spool("x.txt", &stripped).unwrap();
        assert_eq!(without.len(), with_asa.len());
        for (a, b) in with_asa.iter().zip(&without) {
            assert_eq!(a.kind, b.kind);
            assert_eq!(a.first_line, b.first_line);
            assert!(!b.has_asa);
            assert_eq!(a.steps, b.steps);
            assert_eq!(a.statements, b.statements);
            let columns = |dd: &Dd| -> Vec<_> {
                dd.messages
                    .iter()
                    .map(|m| (m.id.clone(), m.lines.clone(), m.id_columns.clone()))
                    .collect()
            };
            let shifted: Vec<_> = columns(a)
                .into_iter()
                .map(|(id, l, c)| (id, l, c.start - 1..c.end - 1))
                .collect();
            assert_eq!(columns(b), shifted);
        }
        assert_eq!(without[2].first_line, starts[2]);
        let codes: Vec<String> = analyze(&without[2], &Options::default())
            .into_iter()
            .map(|f| f.code)
            .collect();
        assert!(codes.contains(&"S013".to_owned()), "{codes:?}");
    }

    /// Layout of a `.jes` file saved by BMC AMI DevX at a client site: every
    /// DD concatenated, ASA column kept, program messages (`+WLM644E`), a
    /// non-ASCII character in the JOB card.
    #[test]
    fn devx_jes_file() {
        let msglg: String = corpus("JOB00868/JES2/JESMSGLG.txt")
            .lines()
            .flat_map(|l| {
                let extra = l.contains("$HASP373").then_some(
                    " 04.42.18 JOB00868  +WLM644E ******* PRODUCT GRACE PERIOD EXPIRED\n",
                );
                [format!("{l}\n"), extra.unwrap_or_default().to_owned()]
            })
            .collect();
        let jcl = corpus("JOB00868/JES2/JESJCL.txt").replacen("JOB (", "JOB (£", 1);
        let mut text = msglg + &jcl;
        for dd in [
            "JES2/JESYSMSG.txt",
            "STEP005/SYSPRINT.txt",
            "STEP010/SYSPRINT.txt",
        ] {
            text.push_str(&corpus(&format!("JOB00868/{dd}")));
        }
        let dds = parse_spool(r"c:\tmp\JRIDC8_JOB00868.jes", &text).unwrap();
        let kinds: Vec<_> = dds.iter().map(|d| d.kind).collect();
        assert_eq!(
            kinds,
            [
                DdKind::JesMsgLg,
                DdKind::JesJcl,
                DdKind::JesYsMsg,
                DdKind::Other
            ]
        );
        assert_eq!(dds[0].steps.len(), 3, "step table of JESMSGLG");
        assert!(dds[0].messages.iter().any(|m| m.id == "WLM644E"));
        assert!(
            dds[1].statements.len() >= 8,
            "JCL with £: {:?}",
            dds[1].statements.len()
        );
        let mut sysmsg = dds[2].clone();
        assert_eq!(sysmsg.steps.len(), 3);
        sysmsg.attach_programs(&dds[1]);
        assert_eq!(sysmsg.steps[1].program.as_deref(), Some("IDCAMS"));
        assert!(dds.iter().all(|d| d.job_id.as_deref() == Some("JOB00868")));
    }

    #[test]
    fn header_and_running_job() {
        // JESYSMSG with its header; TSO session still running (no IEF033I).
        let (text, starts) = spool(
            "TSU00812",
            &["JES2/JESMSGLG.txt", "JES2/JESJCL.txt", "JES2/JESYSMSG.txt"],
        );
        let lines: Vec<&str> = text.lines().collect();
        let kinds: Vec<_> = split(&lines)
            .iter()
            .map(|p| (p.kind, p.lines.start))
            .collect();
        assert_eq!(
            kinds,
            [
                (DdKind::JesMsgLg, starts[0]),
                (DdKind::JesJcl, starts[1]),
                (DdKind::JesYsMsg, starts[2]),
            ]
        );
    }

    #[test]
    fn single_dds_are_not_spools() {
        for path in [
            "JOB00901/JES2/JESMSGLG.txt",
            "JOB00901/JES2/JESJCL.txt",
            "JOB00901/JES2/JESYSMSG.txt",
            "JOB00901/STEP010/SYSPRINT.txt",
        ] {
            assert!(parse_spool(path, &corpus(path)).is_none(), "{path}");
            assert_eq!(parse_document(path, &corpus(path)).len(), 1);
        }
        assert!(parse_spool("x", "").is_none());
    }
}
