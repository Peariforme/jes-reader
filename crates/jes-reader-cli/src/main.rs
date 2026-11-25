use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

/// Analyse de sysouts JES2 z/OS.
#[derive(Parser)]
#[command(name = "jes-reader", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Vérifie une sysout et signale les erreurs (code de sortie non nul en cas d'erreur).
    Check {
        /// Fichier sysout à analyser.
        file: PathBuf,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Check { file } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("lecture de {}", file.display()))?;
            // Une sysout peut contenir des octets mal convertis depuis l'EBCDIC : on les tolère.
            let text = String::from_utf8_lossy(&bytes);
            println!("{} : {} lignes", file.display(), text.lines().count());
        }
    }
    Ok(())
}
