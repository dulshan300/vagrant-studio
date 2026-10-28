//! Operation log files: human-readable text, one output line per line,
//! prefixed `[out]`/`[err]`; `#` lines are metadata written by Vagrant Studio.

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use serde::Serialize;
use vagrant_studio_domain::{Operation, OperationStatus};
use vagrant_studio_vagrant_adapter::Stream;

pub(crate) struct LogWriter {
    file: Option<BufWriter<File>>,
}

impl LogWriter {
    /// Logging problems never fail an operation; they are reported in the app log.
    pub(crate) fn create(path: &Path, op: &Operation) -> Self {
        let file = File::create(path)
            .map_err(|e| tracing::warn!(path = %path.display(), error = %e, "cannot create operation log"))
            .ok()
            .map(BufWriter::new);
        let mut writer = Self { file };
        writer.raw(&format!("# Vagrant Studio operation {}", op.id));
        writer.raw(&format!("# target: {}", op.target_label));
        writer.raw(&format!("# command: {}", op.command_line));
        if let Some(dir) = &op.working_dir {
            writer.raw(&format!("# working directory: {dir}"));
        }
        if let Some(started) = op.started_at {
            writer.raw(&format!("# started: {started}"));
        }
        writer
    }

    pub(crate) fn line(&mut self, stream: Stream, text: &str) {
        let prefix = match stream {
            Stream::Stdout => "[out]",
            Stream::Stderr => "[err]",
        };
        self.raw(&format!("{prefix} {text}"));
    }

    /// A note from Vagrant Studio between command outputs.
    pub(crate) fn meta(&mut self, text: &str) {
        self.raw(&format!("# {text}"));
    }

    pub(crate) fn footer(
        &mut self,
        status: OperationStatus,
        exit_code: Option<i32>,
        summary: Option<&str>,
    ) {
        self.raw(&format!(
            "# finished: {} (exit code {})",
            status.as_str(),
            exit_code.map_or_else(|| "none".to_owned(), |c| c.to_string())
        ));
        if let Some(summary) = summary {
            for line in summary.lines() {
                self.raw(&format!("# {line}"));
            }
        }
    }

    fn raw(&mut self, line: &str) {
        if let Some(file) = &mut self.file
            && writeln!(file, "{line}")
                .and_then(|()| file.flush())
                .is_err()
        {
            self.file = None;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLineKind {
    Out,
    Err,
    Meta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogLine {
    pub kind: LogLineKind,
    pub text: String,
}

/// Reads a log file back for display, keeping at most `limit` trailing lines.
pub fn read_log(path: &Path, limit: usize) -> std::io::Result<Vec<LogLine>> {
    let reader = BufReader::new(File::open(path)?);
    let mut lines: std::collections::VecDeque<LogLine> = std::collections::VecDeque::new();
    for line in reader.lines() {
        let line = line?;
        let parsed = if let Some(t) = line.strip_prefix("[out] ") {
            LogLine {
                kind: LogLineKind::Out,
                text: t.to_owned(),
            }
        } else if let Some(t) = line.strip_prefix("[err] ") {
            LogLine {
                kind: LogLineKind::Err,
                text: t.to_owned(),
            }
        } else if let Some(t) = line.strip_prefix("# ") {
            LogLine {
                kind: LogLineKind::Meta,
                text: t.to_owned(),
            }
        } else {
            LogLine {
                kind: LogLineKind::Out,
                text: line,
            }
        };
        lines.push_back(parsed);
        if lines.len() > limit {
            lines.pop_front();
        }
    }
    Ok(lines.into())
}
