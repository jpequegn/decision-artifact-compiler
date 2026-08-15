use std::{env, fs, path::PathBuf};

use artifact_eval::{render_csv, render_markdown, run_evaluation};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = env::args().skip(1).collect();
    let check = arguments.iter().any(|argument| argument == "--check");
    let output = arguments
        .windows(2)
        .find(|pair| pair[0] == "--output-dir")
        .map_or_else(|| PathBuf::from("reports"), |pair| PathBuf::from(&pair[1]));
    let report = run_evaluation()?;
    let artifacts = [
        (
            "evaluation.json",
            format!("{}\n", serde_json::to_string_pretty(&report)?),
        ),
        ("evaluation.csv", render_csv(&report)),
        ("evaluation.md", render_markdown(&report)),
    ];
    if check {
        for (name, expected) in artifacts {
            let actual = fs::read_to_string(output.join(name))?;
            if actual != expected {
                return Err(format!("generated report is stale: {name}").into());
            }
        }
    } else {
        fs::create_dir_all(&output)?;
        for (name, content) in artifacts {
            fs::write(output.join(name), content)?;
        }
    }
    println!(
        "evaluation: {:.1}% correctness",
        report.compile_correctness_pct
    );
    Ok(())
}
