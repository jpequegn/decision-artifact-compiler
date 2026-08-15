use std::{fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use artifact_core::{ValidationError, artifact_json_schema, parse_artifact, validate_artifact};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "decision-artifact",
    version,
    about = "Compile approved Markdown decisions into authorized task graphs"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the supported artifact format version.
    FormatVersion,
    /// Validate an artifact without compiling it.
    Validate { artifact: PathBuf },
    /// Compile an artifact to canonical JSON.
    Compile {
        artifact: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Print the artifact JSON Schema.
    Schema {
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::FormatVersion => println!("{}", artifact_core::artifact_format_version()),
        Command::Validate { artifact } => {
            let parsed = load(&artifact)?;
            validate(&parsed)?;
            println!(
                "valid artifact: {} ({} tasks)",
                parsed.id,
                parsed.tasks.len()
            );
        }
        Command::Compile { artifact, output } => {
            let parsed = load(&artifact)?;
            validate(&parsed)?;
            write(output.as_ref(), &serde_json::to_string_pretty(&parsed)?)?;
        }
        Command::Schema { output } => {
            write(output.as_ref(), &artifact_json_schema()?)?;
        }
    }
    Ok(())
}

fn load(path: &PathBuf) -> Result<artifact_core::DecisionArtifact> {
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    parse_artifact(&source).map_err(Into::into)
}

fn validate(artifact: &artifact_core::DecisionArtifact) -> Result<()> {
    match validate_artifact(artifact) {
        Ok(()) => Ok(()),
        Err(ValidationError::Invalid { diagnostics }) => {
            eprintln!("{}", serde_json::to_string_pretty(&diagnostics)?);
            bail!("artifact failed validation")
        }
        Err(error) => Err(error.into()),
    }
}

fn write(output: Option<&PathBuf>, content: &str) -> Result<()> {
    if let Some(path) = output {
        fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))?;
    } else {
        println!("{content}");
    }
    Ok(())
}
