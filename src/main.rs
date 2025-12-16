mod eml;
mod mbox;

use crate::{eml::ConvertToMboxCommand, mbox::ConvertToEmlCommand};
use clap::Parser;
use std::path::PathBuf;

/// A simple and quick bidirectional converter between mbox and eml formats.
#[derive(Parser)]
#[clap(about, long_about, version, author)]
struct Arguments {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Parser)]
enum Commands {
    EmlToMbox(ConvertToMboxCommand),
    MboxToEml(ConvertToEmlCommand),
}

fn validate_output_file(s: &str) -> Result<PathBuf, String> {
    if s.ends_with('/') || s.ends_with('\\') {
        return Err(format!("'{}' appears to be a directory, not a file", s));
    }
    Ok(PathBuf::from(s))
}

fn main() -> anyhow::Result<()> {
    match Arguments::parse().command {
        Commands::EmlToMbox(cmd) => cmd.run(),
        Commands::MboxToEml(cmd) => cmd.run(),
    }
}
