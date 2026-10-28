//! The Vagrantfile subset that Vagrant Studio can edit visually (ADR-0005).
//!
//! Vagrantfiles are Ruby; nothing here evaluates Ruby or claims to understand
//! a whole file. [`analyze`] accepts a file only when every statement belongs
//! to the supported subset (see docs/vagrantfile-editor.md), and otherwise
//! reports the first line it can't represent so the UI can fall back to raw
//! editing. [`generate`] writes a model back, marked with a checksum header
//! so later edits outside the visual editor are noticed.
//!
//! This crate does no I/O; `project-service` reads, backs up and saves files.

mod generate;
mod lexer;
mod model;
mod parse;
mod validate;

use serde::Serialize;
use sha2::{Digest, Sha256};

pub use generate::generate;
pub use model::*;
pub use validate::{FieldError, MAX_CPUS, MAX_MEMORY_MB, MIN_MEMORY_MB, validate};

/// Starts the header line holding the checksum of everything after it.
pub const MARKER_PREFIX: &str = "# vagrant-studio: sha256=";

/// A line the visual editor can't represent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[error("line {line}: {message}")]
pub struct Unsupported {
    pub line: usize,
    pub message: String,
}

impl Unsupported {
    pub(crate) fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
    /// The model, when the whole file is in the supported subset.
    pub model: Option<VagrantConfig>,
    /// Why the file can't be edited visually.
    pub unsupported: Option<Unsupported>,
    /// The file carries the visual editor's header.
    pub managed: bool,
    /// Managed, but changed since the visual editor wrote it.
    pub modified_outside: bool,
    /// Things a visual save would lose, such as comments.
    pub warnings: Vec<String>,
    /// Problems to fix before the model can be saved.
    pub errors: Vec<FieldError>,
    pub line_ending: LineEnding,
}

/// Reads a Vagrantfile into the model when it is entirely in the supported subset.
pub fn analyze(text: &str) -> Analysis {
    let line_ending = if text.contains("\r\n") {
        LineEnding::Crlf
    } else {
        LineEnding::Lf
    };
    let text = text.replace("\r\n", "\n");
    let (managed, modified_outside) = match marker(&text) {
        Some((sum, body)) => (true, checksum(body) != sum),
        None => (false, false),
    };
    let mut analysis = Analysis {
        model: None,
        unsupported: None,
        managed,
        modified_outside,
        warnings: Vec::new(),
        errors: Vec::new(),
        line_ending,
    };
    let result = check_characters(&text)
        .and_then(|()| lexer::lex(&text))
        .and_then(|lexed| Ok((parse::parse(&lexed.tokens)?, lexed.comments)));
    match result {
        Ok((model, comments)) => {
            let lost: Vec<usize> = comments
                .iter()
                .filter(|(_, c)| !ignorable_comment(c))
                .map(|(line, _)| *line)
                .collect();
            if !lost.is_empty() {
                analysis.warnings.push(format!(
                    "Saving from the visual editor removes {} comment{} (line {}).",
                    lost.len(),
                    if lost.len() == 1 { "" } else { "s" },
                    lost.iter()
                        .map(usize::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            analysis.errors = validate(&model);
            analysis.model = Some(model);
        }
        Err(u) => analysis.unsupported = Some(u),
    }
    analysis
}

/// Lowercase hex SHA-256 of `text`.
pub fn checksum(text: impl AsRef<[u8]>) -> String {
    Sha256::digest(text.as_ref())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The checksum in the marker line and the text after that line.
fn marker(text: &str) -> Option<(&str, &str)> {
    let start = if text.starts_with(MARKER_PREFIX) {
        0
    } else {
        text.find(&format!("\n{MARKER_PREFIX}"))? + 1
    };
    let rest = &text[start + MARKER_PREFIX.len()..];
    let (sum, body) = rest.split_once('\n').unwrap_or((rest, ""));
    (sum.len() == 64 && sum.bytes().all(|b| b.is_ascii_hexdigit())).then_some((sum, body))
}

fn check_characters(text: &str) -> Result<(), Unsupported> {
    if text.starts_with('\u{feff}') {
        return Err(Unsupported::new(
            1,
            "the file starts with a byte order mark",
        ));
    }
    for (i, line) in text.split('\n').enumerate() {
        if line.chars().any(|c| c.is_control() && c != '\t') {
            return Err(Unsupported::new(
                i + 1,
                "the line contains control characters (mixed line endings?)",
            ));
        }
    }
    Ok(())
}

/// Editor modelines and the visual editor's own header lines.
fn ignorable_comment(comment: &str) -> bool {
    let c = comment.trim_start_matches('#').trim();
    c.is_empty()
        || c.contains("-*- mode: ruby -*-")
        || c.starts_with("vi: set ft=ruby")
        || c.contains("Vagrant Studio")
        || comment.starts_with(MARKER_PREFIX)
        || c.starts_with("the editor re-reads the file")
}

#[cfg(test)]
mod tests;
