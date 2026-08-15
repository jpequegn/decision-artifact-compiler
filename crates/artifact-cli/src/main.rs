use std::path::PathBuf;

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

fn main() {
    match Cli::parse().command {
        Command::FormatVersion => println!("{}", artifact_core::artifact_format_version()),
        Command::Validate { artifact } => {
            println!("validation is not implemented: {}", artifact.display());
        }
        Command::Compile { artifact, output } => {
            println!(
                "compilation is not implemented: {} -> {}",
                artifact.display(),
                output
                    .as_ref()
                    .map_or("stdout".to_owned(), |path| path.display().to_string())
            );
        }
        Command::Schema { output } => {
            println!(
                "schema generation is not implemented: {}",
                output
                    .as_ref()
                    .map_or("stdout".to_owned(), |path| path.display().to_string())
            );
        }
    }
}
