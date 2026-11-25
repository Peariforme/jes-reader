use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use jes_reader_core::{FindingSeverity, Options, analyze, parse_file};

/// Analyse de sysouts JES2 z/OS.
#[derive(Parser)]
#[command(name = "jes-reader", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Vérifie des sysouts et signale les erreurs (code de sortie 1 en cas d'erreur).
    Check {
        /// Fichiers ou dossiers (une DD par fichier, comme les télécharge Zowe Explorer).
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// Code retour au-delà duquel un step est signalé.
        #[arg(long, default_value_t = 4)]
        max_rc: u16,
    },
}

fn main() -> Result<ExitCode> {
    match Cli::parse().command {
        Command::Check { paths, max_rc } => check(
            &paths,
            &Options {
                max_return_code: max_rc,
            },
        ),
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
        let bytes =
            std::fs::read(file).with_context(|| format!("lecture de {}", file.display()))?;
        // Une sysout peut contenir des octets mal convertis depuis l'EBCDIC : on les tolère.
        let text = String::from_utf8_lossy(&bytes);
        let dd = parse_file(&file.to_string_lossy(), &text);
        for finding in analyze(&dd, options) {
            let label = match finding.severity {
                FindingSeverity::Error => {
                    errors += 1;
                    "erreur"
                }
                FindingSeverity::Warning => {
                    warnings += 1;
                    "avertissement"
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
        "{} fichier(s), {errors} erreur(s), {warnings} avertissement(s)",
        files.len()
    );
    Ok(if errors > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

/// Un fichier, ou tous les fichiers d'un dossier et de ses sous-dossiers.
fn collect(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_dir() {
        let entries =
            std::fs::read_dir(path).with_context(|| format!("lecture de {}", path.display()))?;
        for entry in entries {
            collect(&entry?.path(), files)?;
        }
    } else {
        files.push(path.to_owned());
    }
    Ok(())
}
