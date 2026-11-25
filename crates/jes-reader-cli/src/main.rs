use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use jes_reader_core::rules::{self, Rules};
use jes_reader_core::{FindingSeverity, Options, analyze, parse_file};

/// Analysis of z/OS JES2 sysouts.
#[derive(Parser)]
#[command(name = "jes-reader", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Checks sysouts and reports failures (exit code 1 if there is an error).
    Check {
        /// Files or directories (one DD per file, as Zowe Explorer downloads them).
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// Return code above which a step is reported (default 4, or the
        /// rules file's max_return_code). Overrides the rules file.
        #[arg(long)]
        max_rc: Option<u16>,
        /// House rules file. Default: .jesreader.toml in the current
        /// directory, then in the home directory.
        #[arg(long)]
        rules: Option<PathBuf>,
    },
}

fn main() -> Result<ExitCode> {
    match Cli::parse().command {
        Command::Check {
            paths,
            max_rc,
            rules,
        } => {
            let rules_file = rules.or_else(|| rules::find(&[PathBuf::from(".")]));
            let mut rules = match &rules_file {
                Some(path) => Rules::load(path).map_err(anyhow::Error::msg)?,
                None => Rules::default(),
            };
            if max_rc.is_some() {
                rules.max_return_code = max_rc;
            }
            if let Some(path) = &rules_file {
                eprintln!("rules: {}", path.display());
            }
            check(
                &paths,
                &Options {
                    rules,
                    ..Options::default()
                },
            )
        }
    }
}

fn check(paths: &[PathBuf], options: &Options) -> Result<ExitCode> {
    let mut files = Vec::new();
    for path in paths {
        collect(path, &mut files)?;
    }
    files.sort();

    let (mut errors, mut warnings) = (0, 0);
    for file in &files {
        let bytes = std::fs::read(file).with_context(|| format!("reading {}", file.display()))?;
        // A sysout may contain bytes badly converted from EBCDIC: tolerate them.
        let text = String::from_utf8_lossy(&bytes);
        let dd = parse_file(&file.to_string_lossy(), &text);
        for finding in analyze(&dd, options) {
            let label = match finding.severity {
                FindingSeverity::Error => {
                    errors += 1;
                    "error"
                }
                FindingSeverity::Warning => {
                    warnings += 1;
                    "warning"
                }
                FindingSeverity::Information => "info",
            };
            let column = finding.columns.map_or(1, |c| c.start + 1);
            println!(
                "{}:{}:{column}: {label}[{}] {}",
                file.display(),
                finding.line + 1,
                finding.code,
                finding.message
            );
        }
    }
    println!(
        "{} file(s), {errors} error(s), {warnings} warning(s)",
        files.len()
    );
    Ok(if errors > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

/// A file, or every file in a directory and its subdirectories.
fn collect(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_dir() {
        let entries =
            std::fs::read_dir(path).with_context(|| format!("reading {}", path.display()))?;
        for entry in entries {
            collect(&entry?.path(), files)?;
        }
    } else {
        files.push(path.to_owned());
    }
    Ok(())
}
