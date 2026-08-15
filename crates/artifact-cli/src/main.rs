use std::{fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use artifact_core::{
    ValidationError, artifact_json_schema, compile_artifact, compile_report, export_plan,
    parse_artifact, validate_artifact,
};
use clap::{Parser, Subcommand, ValueEnum};

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
        #[arg(long, value_enum, default_value_t = CompileFormat::Ir)]
        format: CompileFormat,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Print the artifact JSON Schema.
    Schema {
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CompileFormat {
    Ir,
    Plan,
    Report,
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
        Command::Compile {
            artifact,
            format,
            output,
        } => {
            let parsed = load(&artifact)?;
            validate(&parsed)?;
            let compiled = compile_artifact(&parsed)?;
            let content = match format {
                CompileFormat::Ir => serde_json::to_string_pretty(&compiled)?,
                CompileFormat::Plan => serde_json::to_string_pretty(&export_plan(&compiled))?,
                CompileFormat::Report => compile_report(&compiled),
            };
            write(output.as_ref(), &content)?;
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
