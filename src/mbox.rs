use crate::validate_output_file;
use anyhow::{Context, Result, bail};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    fs::{self, File},
    io::{self, BufRead, BufReader, BufWriter, Write},
    iter::Peekable,
    path::{Path, PathBuf},
    time::Duration,
};

/// Convert a single .mbox file to an extracted directory of .eml files.
#[derive(Parser)]
pub struct ConvertToEmlCommand {
    #[arg(value_parser = validate_output_file)]
    input_file: PathBuf,

    output_directory: PathBuf,

    /// Replace any existing eml files in the given directory with new ones if they overlap.
    #[clap(long = "overwrite")]
    overwrite: bool,
}

impl ConvertToEmlCommand {
    pub fn run(&self) -> Result<()> {
        Self::mbox_to_eml(&self.input_file, &self.output_directory, self.overwrite)
    }

    fn mbox_to_eml(input_file: &Path, output_dir: &Path, overwrite: bool) -> Result<()> {
        if !input_file.exists() {
            bail!("Mbox file at {:?} does not exist", input_file);
        }
        if output_dir.exists() && !overwrite {
            bail!(
                "Directory already exists at {:?}. Use the --overwrite flag to replace overlapping files inside of it.",
                output_dir
            );
        }
        fs::create_dir_all(output_dir)
            .with_context(|| format!("failed to create output directory at {output_dir:?}"))?;

        let (converted, errors) = {
            let reader = BufReader::new(
                File::open(input_file)
                    .with_context(|| format!("failed to open mbox file at {input_file:?}"))?,
            );
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("[{elapsed_precise}] {spinner} {human_pos} emails processed {msg}")
                    .unwrap(),
            );
            pb.enable_steady_tick(Duration::from_millis(100));

            let mut parser = MboxParser::new(reader.lines());
            let (mut converted, mut errors) = (0, 0);
            while let Some(email_result) = parser.next_message() {
                match email_result {
                    Ok(email) => {
                        let subject = Self::extract_subject(&email);
                        match Self::save_eml_file(output_dir, converted, subject, &email) {
                            Ok(()) => converted += 1,
                            Err(e) => {
                                pb.println(format!("Error saving email {}: {}", converted, e));
                                errors += 1;
                            }
                        }
                    }
                    Err(e) => {
                        pb.println(format!("Error reading email {}: {}", converted, e));
                        errors += 1;
                    }
                }
                pb.inc(1);
            }

            pb.finish_and_clear();
            (converted, errors)
        };

        println!(
            "Conversion of {} emails completed with {} errors. Output saved to {:?}",
            converted, errors, output_dir
        );

        Ok(())
    }

    fn extract_subject(content: &[String]) -> Option<String> {
        for line in content {
            if line.to_lowercase().starts_with("subject:") {
                let subject = line[8..].trim();
                if subject.is_empty() {
                    return None;
                }
                return Some(sanitize_filename::sanitize(subject));
            }
        }
        None
    }

    fn save_eml_file(
        output_dir: &Path,
        index: usize,
        subject: Option<String>,
        content: &[String],
    ) -> Result<()> {
        let filename = if let Some(subject) = subject {
            format!("{:04}_{}.eml", index, subject)
        } else {
            format!("{:04}.eml", index)
        };
        let filepath = output_dir.join(filename);
        let mut file = BufWriter::new(
            File::create(&filepath)
                .with_context(|| format!("failed to create eml file at {filepath:?}"))?,
        );
        for line in content {
            writeln!(file, "{}", line)?;
        }
        file.flush()?;
        Ok(())
    }
}

struct MboxParser<I: Iterator<Item = io::Result<String>>> {
    lines: Peekable<I>,
    finished: bool,
}

impl<I: Iterator<Item = io::Result<String>>> MboxParser<I> {
    fn new(lines: I) -> Self {
        Self {
            lines: lines.peekable(),
            finished: false,
        }
    }

    fn next_message(&mut self) -> Option<Result<Vec<String>>> {
        if self.finished {
            return None;
        }

        while let Some(Ok(line)) = self.lines.peek() {
            if line.starts_with("From ") {
                self.lines.next();
                break;
            }
            self.lines.next();
        }

        let mut email_data = Vec::new();
        while let Some(line_result) = self.lines.peek() {
            match line_result {
                Ok(line) => {
                    if line.starts_with("From ") {
                        return Some(Ok(email_data));
                    }
                    if let Some(Ok(line)) = self.lines.next() {
                        email_data.push(line);
                    }
                }
                Err(_) => {
                    self.finished = true;
                    return self
                        .lines
                        .next()
                        .map(|r| r.map(|_| email_data).map_err(Into::into));
                }
            }
        }
        self.finished = true;
        if !email_data.is_empty() {
            Some(Ok(email_data))
        } else {
            None
        }
    }
}
