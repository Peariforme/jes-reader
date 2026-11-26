mod check;
mod files;
mod summary;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use jes_reader_core::Options;
use jes_reader_core::rules::{self, Rules};

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
        #[command(flatten)]
        input: Input,
        /// Output format.
        #[arg(long, value_enum, default_value_t = CheckFormat::Text)]
        format: CheckFormat,
    },
    /// Summarizes each job: end, then each step with its program, outcome,
    /// CPU and elapsed time, then its errors and warnings.
    Summary {
        #[command(flatten)]
        input: Input,
        /// Output format.
        #[arg(long, value_enum, default_value_t = SummaryFormat::Text)]
        format: SummaryFormat,
    },
}

/// Files to read and rules to apply, common to all commands.
#[derive(clap::Args)]
struct Input {
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
}

#[derive(Clone, Copy, ValueEnum)]
enum CheckFormat {
    /// `file:line:column: severity[code] message`, one finding per line.
    Text,
    /// A JSON document with every finding.
    Json,
    /// SARIF 2.1.0, for code scanning tools (GitHub, Azure DevOps…).
    Sarif,
}

#[derive(Clone, Copy, ValueEnum)]
enum SummaryFormat {
    /// One block per job.
    Text,
    /// A JSON document with every job.
    Json,
}

impl Input {
    /// Analysis options: the rules file, then `--max-rc`.
    fn options(&self) -> Result<Options> {
        let rules_file = self
            .rules
            .clone()
            .or_else(|| rules::find(&[PathBuf::from(".")]));
        let mut rules = match &rules_file {
            Some(path) => Rules::load(path).map_err(anyhow::Error::msg)?,
            None => Rules::default(),
        };
        if self.max_rc.is_some() {
            rules.max_return_code = self.max_rc;
        }
        if let Some(path) = &rules_file {
            eprintln!("rules: {}", path.display());
        }
        Ok(Options {
            rules,
            ..Options::default()
        })
    }
}

fn main() -> Result<ExitCode> {
    match Cli::parse().command {
        Command::Check { input, format } => {
            let options = input.options()?;
            let files = files::load(&input.paths)?;
            let report = check::Report::new(&files, &options);
            let out = match format {
                CheckFormat::Text => report.text(),
                CheckFormat::Json => report.json().to_string(),
                CheckFormat::Sarif => report.sarif().to_string(),
            };
            println!("{}", out.trim_end());
            Ok(if report.errors() > 0 {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            })
        }
        Command::Summary { input, format } => {
            let options = input.options()?;
            let files = files::load(&input.paths)?;
            let jobs = summary::jobs(&files, &options);
            let out = match format {
                SummaryFormat::Text => summary::text(&jobs),
                SummaryFormat::Json => summary::json(&jobs).to_string(),
            };
            println!("{}", out.trim_end());
            Ok(ExitCode::SUCCESS)
        }
    }
}
