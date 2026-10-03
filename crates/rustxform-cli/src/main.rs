//! Command-line interface for rustxform.
//!
//! Accepts a Markdown (`.md`), CSV (`.csv`) or spreadsheet (`.xlsx`/`.xls`)
//! XLSForm, validates it, and writes the generated XForm, chosen by input
//! extension. Invalid forms are rejected; warnings are reported but non-fatal.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::{Context, Result, bail};
use clap::Parser;
use rustxform::{BuildError, Built};
use serde_json::json;

/// Compile an XLSForm into an XForm.
#[derive(Debug, Parser)]
#[command(name = "rustxform", version, about)]
struct Cli {
    /// Path to the XLSForm source (`.md`, `.csv`, `.xlsx` or `.xls`).
    input: PathBuf,
    /// Path to write the XForm to; defaults to the input path with `.xml`.
    output: Option<PathBuf>,
    /// Report warnings and errors as JSON on stdout.
    #[arg(long)]
    json: bool,
    /// Run ODK Validate on the output: `java -jar <JAR> <xform>` (needs Java).
    #[arg(long, value_name = "JAR")]
    odk_validate: Option<PathBuf>,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let extension = cli
        .input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let read_text = |path: &Path| {
        fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
    };

    let result = match extension.as_str() {
        "md" => rustxform::build_markdown(&read_text(&cli.input)?),
        "csv" => rustxform::build_csv(&read_text(&cli.input)?),
        "xlsx" | "xls" => {
            let bytes =
                fs::read(&cli.input).with_context(|| format!("reading {}", cli.input.display()))?;
            rustxform::build_xlsx(&bytes)
        }
        other => bail!("unsupported input extension: .{other} (use .md, .csv, .xlsx or .xls)"),
    };

    match result {
        Ok(built) => write_output(&cli, &built),
        Err(BuildError::Invalid(errors)) => {
            report_errors(cli.json, &errors);
            Ok(ExitCode::FAILURE)
        }
        Err(error) => Err(anyhow::Error::new(error).context("converting XLSForm to XForm")),
    }
}

fn write_output(cli: &Cli, built: &Built) -> Result<ExitCode> {
    let output = cli
        .output
        .clone()
        .unwrap_or_else(|| cli.input.with_extension("xml"));
    fs::write(&output, &built.xform).with_context(|| format!("writing {}", output.display()))?;

    if let Some(jar) = &cli.odk_validate {
        if !odk_validate(jar, &output)? {
            if cli.json {
                let report =
                    json!({ "status": "invalid-xform", "output": output.display().to_string() });
                println!("{report}");
            } else {
                eprintln!("ODK Validate rejected {}", output.display());
            }
            return Ok(ExitCode::FAILURE);
        }
    }

    let warnings: Vec<String> = built.warnings.iter().map(ToString::to_string).collect();
    if cli.json {
        let report = json!({ "status": "ok", "output": output.display().to_string(), "warnings": warnings, "errors": [] });
        println!("{report}");
    } else {
        for warning in &warnings {
            eprintln!("warning: {warning}");
        }
        println!("wrote {}", output.display());
    }
    Ok(ExitCode::SUCCESS)
}

/// Run ODK Validate (`java -jar <jar> <xform>`) and relay its output; returns
/// whether the XForm was accepted.
fn odk_validate(jar: &Path, xform: &Path) -> Result<bool> {
    let output = Command::new("java")
        .arg("-jar")
        .arg(jar)
        .arg(xform)
        .output()
        .with_context(|| "running ODK Validate (is `java` installed and on PATH?)")?;

    let report = String::from_utf8_lossy(&output.stderr);
    if !report.trim().is_empty() {
        eprint!("{report}");
    }
    Ok(output.status.success())
}

fn report_errors(as_json: bool, errors: &[rustxform::ValidationError]) {
    let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
    if as_json {
        let report = json!({ "status": "error", "warnings": [], "errors": messages });
        println!("{report}");
    } else {
        for message in &messages {
            eprintln!("error: {message}");
        }
        eprintln!("{} validation error(s)", messages.len());
    }
}
