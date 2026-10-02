//! Command-line interface for rustxform.
//!
//! Phase 0 accepts a Markdown XLSForm and writes the generated XForm. XLSX,
//! XLS and CSV inputs are added later.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

/// Compile an XLSForm into an XForm.
#[derive(Debug, Parser)]
#[command(name = "rustxform", version, about)]
struct Cli {
    /// Path to the XLSForm source (Markdown `.md` in Phase 0).
    input: PathBuf,
    /// Path to write the XForm to; defaults to the input path with `.xml`.
    output: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let src = fs::read_to_string(&cli.input)
        .with_context(|| format!("reading {}", cli.input.display()))?;

    let xform = rustxform::convert_markdown(&src).context("converting XLSForm to XForm")?;

    let output = cli
        .output
        .unwrap_or_else(|| cli.input.with_extension("xml"));
    fs::write(&output, xform).with_context(|| format!("writing {}", output.display()))?;

    println!("wrote {}", output.display());
    Ok(())
}
