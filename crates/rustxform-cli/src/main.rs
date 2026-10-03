//! Command-line interface for rustxform.
//!
//! Accepts a Markdown (`.md`), CSV (`.csv`) or spreadsheet (`.xlsx`/`.xls`)
//! XLSForm and writes the generated XForm, chosen by input extension.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Parser;

/// Compile an XLSForm into an XForm.
#[derive(Debug, Parser)]
#[command(name = "rustxform", version, about)]
struct Cli {
    /// Path to the XLSForm source (`.md`, `.csv`, `.xlsx` or `.xls`).
    input: PathBuf,
    /// Path to write the XForm to; defaults to the input path with `.xml`.
    output: Option<PathBuf>,
}

fn main() -> Result<()> {
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

    let xform = match extension.as_str() {
        "md" => rustxform::convert_markdown(&read_text(&cli.input)?),
        "csv" => rustxform::convert_csv(&read_text(&cli.input)?),
        "xlsx" | "xls" => {
            let bytes =
                fs::read(&cli.input).with_context(|| format!("reading {}", cli.input.display()))?;
            rustxform::convert_xlsx(&bytes)
        }
        other => bail!("unsupported input extension: .{other} (use .md, .csv, .xlsx or .xls)"),
    }
    .context("converting XLSForm to XForm")?;

    let output = cli
        .output
        .unwrap_or_else(|| cli.input.with_extension("xml"));
    fs::write(&output, xform).with_context(|| format!("writing {}", output.display()))?;

    println!("wrote {}", output.display());
    Ok(())
}
