use crate::validate_output_file;
use anyhow::{Context, Result, bail};
use chrono::DateTime;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

/// Convert a directory of .eml files to a single .mbox file.
#[derive(Parser)]
pub struct ConvertToMboxCommand {
    input_directory: PathBuf,

    #[arg(value_parser = validate_output_file)]
    output_file: PathBuf,

    #[clap(long = "overwrite")]
    overwrite: bool,
}

impl ConvertToMboxCommand {
    pub fn run(&self) -> Result<()> {
        Self::eml_to_mbox(&self.input_directory, &self.output_file, self.overwrite)
    }

    fn get_header_value<'a>(content: &'a str, header_name: &str) -> Option<&'a str> {
        let prefix = format!("{}:", header_name.to_lowercase());
        content
            .lines()
            .find(|line| line.to_lowercase().starts_with(&prefix))
            .map(|line| line[prefix.len()..].trim())
    }

    fn eml_to_mbox(input_dir: &Path, output_file: &Path, overwrite: bool) -> Result<()> {
        if output_file.exists() && !overwrite {
            bail!(
                "File already exists at {:?}. Use the --overwrite flag to replace it.",
                output_file
            );
        }

        let eml_files = {
            let mut eml_files = Vec::new();
            Self::find_eml_files(input_dir, &mut eml_files)?;
            if eml_files.is_empty() {
                bail!("Did not find any .eml files inside of {:?}", input_dir);
            }
            eml_files.sort();
            eml_files
        };

        let (converted, errors) = {
            let (mut converted, mut errors) = (0, 0);
            let mut output = File::create(output_file)?;
            let pb = ProgressBar::new(eml_files.len() as u64);
            pb.enable_steady_tick(Duration::from_millis(100));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template(
                        "[{elapsed_precise}] {bar:40.cyan/blue} {human_pos:>7}/{human_len:7} {msg}",
                    )
                    .unwrap()
                    .progress_chars("#>-"),
            );
            for eml_file in &eml_files {
                match Self::process_eml_file(eml_file, &mut output) {
                    Ok(()) => converted += 1,
                    Err(e) => {
                        pb.println(format!("Error processing {:?}: {}", eml_file, e));
                        errors += 1;
                    }
                }
                pb.inc(1);
            }
            pb.finish_and_clear();
            (converted, errors)
        };

        println!(
            "Conversion of {converted} eml files completed with {errors} errors. Output saved to {:?}",
            output_file
        );

        Ok(())
    }

    fn find_eml_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in
            fs::read_dir(dir).with_context(|| format!("failed to read directory at {dir:?}"))?
        {
            let path = entry?.path();
            if path.is_dir() {
                Self::find_eml_files(&path, files)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("eml") {
                files.push(path);
            }
        }
        Ok(())
    }

    fn process_eml_file(eml_file: &Path, output: &mut File) -> Result<()> {
        let content = fs::read_to_string(eml_file)
            .with_context(|| format!("failed to read eml file at {eml_file:?}"))?;

        let from_addr = Self::get_header_value(&content, "from")
            .and_then(|value| {
                if let Some(start) = value.find('<') {
                    value.find('>').map(|end| &value[start + 1..end])
                } else {
                    Some(value)
                }
            })
            .unwrap_or("unknown@example.com");

        let date_str = Self::get_header_value(&content, "date")
            .and_then(|value| {
                DateTime::parse_from_rfc2822(value)
                    .or_else(|_| DateTime::parse_from_rfc3339(value))
                    .ok()
                    .map(|dt| dt.format("%a %b %d %H:%M:%S %Y").to_string())
            })
            .unwrap_or_else(|| "Mon Jan 01 00:00:00 2024".to_string());

        writeln!(output, "From {} {}", from_addr, date_str)
            .context("failed to write from line to mbox output file")?;
        write!(output, "{}", content).context("failed to write content to mbox output file")?;

        match content.as_bytes() {
            b if b.ends_with(b"\n\n") => {}
            b if b.ends_with(b"\n") => writeln!(output)?,
            _ => {
                writeln!(output)?;
                writeln!(output)?;
            }
        }

        output.flush()?;
        Ok(())
    }
}
