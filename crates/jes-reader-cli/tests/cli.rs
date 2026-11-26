//! The `jes-reader` binary on corpus jobs.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/sysout")
}

/// Runs `jes-reader` in the corpus directory, without any rules file from
/// the home directory. Path separators are turned into `/` (Windows).
fn run(args: &[&str]) -> (bool, String) {
    let home = std::env::temp_dir().join(format!("jes-reader-cli-home-{}", std::process::id()));
    std::fs::create_dir_all(&home).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_jes-reader"))
        .args(args)
        .current_dir(corpus())
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("error:"), "{stderr}");
    let stdout = String::from_utf8(output.stdout)
        .unwrap()
        .replace("\\\\", "/")
        .replace('\\', "/");
    (output.status.success(), stdout)
}

fn json(args: &[&str]) -> Value {
    serde_json::from_str(&run(args).1).unwrap()
}

#[test]
fn check_text() {
    let (success, out) = run(&["check", "JOB00868"]);
    assert!(success, "only warnings: {out}");
    assert!(
        out.contains("JOB00868/JES2/JESYSMSG.txt:19:1: warning[RC] Step STEP010: return code 8"),
        "{out}"
    );
    assert!(
        out.ends_with("5 file(s), 0 error(s), 2 warning(s)\n"),
        "{out}"
    );

    let (success, _) = run(&["check", "JOB00896"]);
    assert!(!success, "S0C7 is an error");
}

#[test]
fn check_json() {
    let report = json(&["check", "--format", "json", "JOB00896/JES2/JESYSMSG.txt"]);
    assert_eq!(report["files"], 1);
    let first = &report["findings"][0];
    assert_eq!(first["severity"], "error");
    assert_eq!(first["code"], "S0C7");
    assert_eq!(first["file"], "JOB00896/JES2/JESYSMSG.txt");
    assert_eq!(first["line"], 131);
    assert_eq!(first["column"], Value::Null);
}

#[test]
fn check_sarif() {
    let sarif = json(&["check", "--format", "sarif", "JOB00896"]);
    assert_eq!(sarif["version"], "2.1.0");
    let run = &sarif["runs"][0];
    assert_eq!(run["tool"]["driver"]["name"], "jes-reader");
    let rules = run["tool"]["driver"]["rules"].as_array().unwrap();
    let results = run["results"].as_array().unwrap();
    assert!(!results.is_empty());
    for result in results {
        let index = result["ruleIndex"].as_u64().unwrap() as usize;
        assert_eq!(rules[index]["id"], result["ruleId"]);
        let location = &result["locations"][0]["physicalLocation"];
        assert!(
            location["artifactLocation"]["uri"]
                .as_str()
                .unwrap()
                .starts_with("JOB00896/"),
            "{location}"
        );
        assert!(location["region"]["startLine"].as_u64().unwrap() >= 1);
    }
    let abend = results
        .iter()
        .find(|r| {
            r["ruleId"] == "S0C7"
                && r["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
                    == "JOB00896/JES2/JESYSMSG.txt"
        })
        .unwrap();
    assert_eq!(abend["level"], "error");
}

#[test]
fn summary_text() {
    let (success, out) = run(&["summary", "JOB00868", "JOB00896"]);
    assert!(success);
    let expected = "\
JRIDC8 JOB00868: ended, RC 8
  STEP     PROGRAM  OUTCOME  CPU     ELAPSED
  STEP005  IDCAMS   RC 4     0.00 s  0.00 s
  STEP010  IDCAMS   RC 8     0.00 s  0.00 s
  STEP020  IEFBR14  not run  0.00 s  0.00 s
  warning: Step STEP010: return code 8 (JOB00868/JES2/JESYSMSG.txt:19)
";
    assert!(out.starts_with(expected), "{out}");
    assert!(
        out.contains("\n\nJRS0C7 JOB00896: ended, abend S0C7\n"),
        "{out}"
    );
    assert!(
        out.contains("  COBRUN.GO     *.LKED.SYSLMOD  abend S0C7 reason 7"),
        "{out}"
    );
}

#[test]
fn summary_json() {
    let summary = json(&["summary", "--format", "json", "JOB00868"]);
    let job = &summary["jobs"][0];
    assert_eq!(job["name"], "JRIDC8");
    assert_eq!(job["id"], "JOB00868");
    assert_eq!(job["end"]["returnCode"], 8);
    assert_eq!(job["warnings"], 1);
    let step = &job["steps"][1];
    assert_eq!(step["name"], "STEP010");
    assert_eq!(step["program"], "IDCAMS");
    assert_eq!(step["status"], "completed");
    assert_eq!(step["returnCode"], 8);
    assert_eq!(step["elapsedToTheMinute"], false);
    assert_eq!(job["steps"][2]["status"], "notRun");
}

/// The phase 3 exit criterion: a short rules file adapts the tool to an
/// application.
#[test]
fn rules_by_program() {
    let dir = std::env::temp_dir().join(format!("jes-reader-cli-rules-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let rules = dir.join("team.toml");
    std::fs::write(
        &rules,
        r#"
# IDCAMS: RC 8 is "nothing to delete" for this application.
[[steps]]
program = "IDCAMS"
max_return_code = 8

# Fault Analyzer cannot write its history file: not about the job.
[[messages]]
id = "IDI*"
severity = "ignore"
"#,
    )
    .unwrap();
    let rules = rules.to_string_lossy();

    let (_, out) = run(&["check", "--rules", &rules, "JOB00868"]);
    assert!(
        out.ends_with("5 file(s), 0 error(s), 0 warning(s)\n"),
        "{out}"
    );
    let (_, out) = run(&["check", "--rules", &rules, "JOB00901"]);
    assert!(!out.contains("IDI0078E"), "{out}");

    std::fs::remove_dir_all(&dir).ok();
}

/// A whole spool: every DD of the job in one file.
#[test]
fn whole_spool() {
    let dir = std::env::temp_dir().join(format!("jes-reader-cli-spool-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let job = corpus().join("JOB00868");
    let mut text = String::new();
    for dd in [
        "JES2/JESMSGLG.txt",
        "JES2/JESJCL.txt",
        "JES2/JESYSMSG.txt",
        "STEP005/SYSPRINT.txt",
        "STEP010/SYSPRINT.txt",
    ] {
        text.push_str(&String::from_utf8_lossy(
            &std::fs::read(job.join(dd)).unwrap(),
        ));
    }
    let spool = dir.join("JRIDC8.txt");
    std::fs::write(&spool, text).unwrap();
    let spool = spool.to_string_lossy();

    let (_, out) = run(&["check", &spool]);
    assert!(
        out.ends_with("1 file(s), 0 error(s), 2 warning(s)\n"),
        "{out}"
    );
    let summary = json(&["summary", "--format", "json", &spool]);
    let job = &summary["jobs"][0];
    assert_eq!(job["id"], "JOB00868");
    assert_eq!(job["files"].as_array().unwrap().len(), 1);
    assert_eq!(job["steps"][1]["program"], "IDCAMS");

    std::fs::remove_dir_all(&dir).ok();
}
