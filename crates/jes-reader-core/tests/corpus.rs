//! Un snapshot par DD du corpus (`corpus/sysout`) : steps, instructions,
//! messages et constats.
//!
//! Mise à jour après un changement voulu du parser :
//! `INSTA_UPDATE=always cargo test -p jes-reader-core --test corpus`,
//! puis relire le diff des fichiers `tests/snapshots/*.snap`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use jes_reader_core::{Dd, FindingSeverity, Options, analyze, parse_file};

#[test]
fn corpus() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/sysout");
    let mut files = Vec::new();
    collect(&root, &mut files);
    files.sort();
    assert!(
        files.len() >= 40,
        "corpus incomplet : {} fichiers",
        files.len()
    );

    for file in files {
        let relative = file
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let bytes = std::fs::read(&file).unwrap();
        let text = anonymize(&String::from_utf8_lossy(&bytes));
        let dd = parse_file(&relative, &text);
        let name: String = relative
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        insta::assert_snapshot!(name, render(&relative, &dd));
    }
}

fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(&path, files);
        } else if path.extension().is_some_and(|e| e == "txt") {
            files.push(path);
        }
    }
}

/// Même règle que `scripts/anonymize.sh` : le corpus local garde le vrai
/// userid, la CI voit `Z99999`. Les snapshots doivent être identiques.
fn anonymize(text: &str) -> String {
    let b = text.as_bytes();
    let mut out = b.to_vec();
    for i in 0..b.len().saturating_sub(5) {
        let before_ok = i == 0 || !b[i - 1].is_ascii_alphanumeric();
        let digits = &b[i + 1..i + 6];
        let after_ok = b.get(i + 6).is_none_or(|c| !c.is_ascii_digit());
        if b[i] == b'Z' && before_ok && after_ok && digits.iter().all(u8::is_ascii_digit) {
            out[i + 1..i + 6].copy_from_slice(b"99999");
        }
    }
    String::from_utf8(out).unwrap()
}

/// Rendu lisible d'une DD ; lignes numérotées à partir de 1 comme dans
/// l'éditeur.
fn render(path: &str, dd: &Dd) -> String {
    let mut s = String::new();
    let opt = |v: &Option<String>| v.clone().unwrap_or_else(|| "?".into());
    writeln!(s, "fichier : {path}").unwrap();
    writeln!(
        s,
        "DD {} ({:?}), ASA : {}, {} lignes",
        opt(&dd.name),
        dd.kind,
        if dd.has_asa { "oui" } else { "non" },
        dd.line_count
    )
    .unwrap();
    writeln!(s, "job : {} {}", opt(&dd.job_name), opt(&dd.job_id)).unwrap();
    if let Some(end) = &dd.job_end {
        writeln!(s, "fin du job : {end:?}").unwrap();
    }

    if !dd.steps.is_empty() {
        writeln!(s, "\nsteps :").unwrap();
        for step in &dd.steps {
            writeln!(
                s,
                "  L{}-{} {} : {:?} (L{})",
                step.lines.start + 1,
                step.lines.end,
                step.display_name(),
                step.status,
                step.status_line + 1
            )
            .unwrap();
        }
    }

    if !dd.statements.is_empty() {
        writeln!(s, "\ninstructions :").unwrap();
        for st in &dd.statements {
            writeln!(
                s,
                "  {:>3} L{}-{} {:?} {} {}",
                st.number,
                st.lines.start + 1,
                st.lines.end,
                st.origin,
                opt(&st.name),
                opt(&st.operation)
            )
            .unwrap();
        }
    }

    if !dd.messages.is_empty() {
        writeln!(s, "\nmessages :").unwrap();
        for m in &dd.messages {
            let lines = if m.lines.len() == 1 {
                format!("L{}", m.lines.start + 1)
            } else {
                format!("L{}-{}", m.lines.start + 1, m.lines.end)
            };
            let statement = m
                .statement
                .map(|n| format!(" [instr. {n}]"))
                .unwrap_or_default();
            writeln!(
                s,
                "  {lines} col.{}-{} {}{statement} {}",
                m.id_columns.start + 1,
                m.id_columns.end,
                m.id,
                m.text
            )
            .unwrap();
        }
    }

    let findings = analyze(dd, &Options::default());
    if !findings.is_empty() {
        writeln!(s, "\nconstats :").unwrap();
        for f in findings {
            let severity = match f.severity {
                FindingSeverity::Error => "erreur",
                FindingSeverity::Warning => "avertissement",
                FindingSeverity::Information => "info",
            };
            writeln!(s, "  L{} {severity} [{}] {}", f.line + 1, f.code, f.message).unwrap();
        }
    }
    s
}
