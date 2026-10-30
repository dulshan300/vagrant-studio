//! Interactive programs in a pseudo-terminal, shown in the app's terminal
//! panel. Used for `vagrant ssh`, which needs a real terminal and so can't run
//! through the adapter's `Runner` (ADR-0009).
//!
//! The program runs with an argv vector (never a shell) and an explicit
//! working directory. On Windows the pseudo-terminal is ConPTY; closing it
//! ends every process attached to it (Vagrant's Ruby and `ssh` children
//! included), which `TerminateProcess` on `vagrant.exe` alone would not. On
//! Linux, closing the master sends SIGHUP to the session.
//!
//! Sessions belong to the user: they are not operations, take no locks and
//! nothing they print is stored.

mod utf8;

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use portable_pty::{Child, ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::{Deserialize, Serialize};

use crate::utf8::Utf8Stream;

/// Open sessions allowed at once.
pub const MAX_SESSIONS: usize = 8;
const READ_BUFFER: usize = 16 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum TerminalError {
    #[error("At most {MAX_SESSIONS} terminals can be open at once. Close one first.")]
    TooMany,
    #[error("This terminal session has ended.")]
    NotFound,
    #[error("The terminal couldn't be started: {0}")]
    Spawn(String),
    #[error("The terminal didn't accept the change: {0}")]
    Io(String),
}

/// Terminal size in character cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    pub cols: u16,
    pub rows: u16,
}

impl Size {
    fn pty(self) -> PtySize {
        PtySize {
            rows: self.rows.clamp(2, 1000),
            cols: self.cols.clamp(10, 1000),
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

/// What to run. `env` is added to the app's own environment.
#[derive(Debug, Clone)]
pub struct SpawnRequest {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
    pub size: Size,
}

/// Receives a session's output, on a background thread.
pub trait TerminalSink: Send + Sync + 'static {
    /// Output as text; characters are never split between calls.
    fn output(&self, data: String);
    /// The program ended (or was closed). Called once, after all output.
    /// `code` is `None` when the program was closed from the app.
    fn exited(&self, code: Option<u32>);
}

struct Session {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}

#[derive(Default)]
struct Inner {
    sessions: HashMap<String, Session>,
    /// Sessions closed from the app, so their exit isn't reported as the
    /// program's own.
    closing: Vec<String>,
}

/// The open terminal sessions. Cheap to clone.
#[derive(Clone, Default)]
pub struct Terminals {
    inner: Arc<Mutex<Inner>>,
}

impl Terminals {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts the program and returns the session id. Output and the exit
    /// go to `sink`.
    pub fn spawn(
        &self,
        req: SpawnRequest,
        sink: Arc<dyn TerminalSink>,
    ) -> Result<String, TerminalError> {
        if self.lock().sessions.len() >= MAX_SESSIONS {
            return Err(TerminalError::TooMany);
        }
        let spawn_err = |e: &dyn std::fmt::Display| TerminalError::Spawn(e.to_string());
        let pair = native_pty_system()
            .openpty(req.size.pty())
            .map_err(|e| spawn_err(&e))?;
        let mut command = CommandBuilder::new(&req.program);
        command.args(&req.args);
        command.cwd(&req.cwd);
        for (key, value) in &req.env {
            command.env(key, value);
        }
        let mut child = pair
            .slave
            .spawn_command(command)
            .map_err(|e| spawn_err(&e))?;
        // The parent's copy of the slave must go, or Linux never reports EOF.
        drop(pair.slave);
        let reader = pair.master.try_clone_reader().map_err(|e| spawn_err(&e))?;
        let writer = pair.master.take_writer().map_err(|e| spawn_err(&e))?;
        let killer = child.clone_killer();

        let id = uuid::Uuid::new_v4().to_string();
        self.lock().sessions.insert(
            id.clone(),
            Session {
                master: pair.master,
                writer,
                killer,
            },
        );
        tracing::info!(session = %id, program = %req.program.display(), "terminal started");

        // ConPTY keeps the output pipe open after the program exits until the
        // pseudo-console is closed, so a waiter closes it; the reader then
        // drains what is left and reports the exit.
        let terminals = self.clone();
        let waiter_id = id.clone();
        let waiter = std::thread::Builder::new()
            .name("terminal-wait".into())
            .spawn(move || {
                let code = wait(&mut *child);
                // Closing a pseudo-console can block briefly: not under the lock.
                let session = terminals.lock().sessions.remove(&waiter_id);
                drop(session);
                code
            })
            .map_err(|e| spawn_err(&e))?;
        let terminals = self.clone();
        let reader_id = id.clone();
        std::thread::Builder::new()
            .name("terminal-read".into())
            .spawn(move || terminals.pump(&reader_id, reader, waiter, &*sink))
            .map_err(|e| spawn_err(&e))?;
        Ok(id)
    }

    /// Sends keyboard input to the program.
    pub fn write(&self, id: &str, data: &[u8]) -> Result<(), TerminalError> {
        let mut inner = self.lock();
        let session = inner.sessions.get_mut(id).ok_or(TerminalError::NotFound)?;
        session
            .writer
            .write_all(data)
            .and_then(|()| session.writer.flush())
            .map_err(|e| TerminalError::Io(e.to_string()))
    }

    pub fn resize(&self, id: &str, size: Size) -> Result<(), TerminalError> {
        let inner = self.lock();
        let session = inner.sessions.get(id).ok_or(TerminalError::NotFound)?;
        session
            .master
            .resize(size.pty())
            .map_err(|e| TerminalError::Io(e.to_string()))
    }

    /// Ends the program and closes the pseudo-terminal. Closing a session
    /// that already ended is not an error.
    pub fn close(&self, id: &str) {
        let session = {
            let mut inner = self.lock();
            let session = inner.sessions.remove(id);
            if session.is_some() {
                inner.closing.push(id.to_owned());
            }
            session
        };
        if let Some(mut session) = session {
            if let Err(e) = session.killer.kill() {
                tracing::debug!(session = %id, error = %e, "terminal program already gone");
            }
            // Dropping the master closes the pseudo-terminal.
            drop(session);
            tracing::info!(session = %id, "terminal closed");
        }
    }

    /// Closes every session (the app is quitting).
    pub fn close_all(&self) {
        let ids: Vec<String> = self.lock().sessions.keys().cloned().collect();
        for id in ids {
            self.close(&id);
        }
    }

    pub fn open_sessions(&self) -> usize {
        self.lock().sessions.len()
    }

    fn pump(
        &self,
        id: &str,
        mut reader: Box<dyn Read + Send>,
        waiter: JoinHandle<Option<u32>>,
        sink: &dyn TerminalSink,
    ) {
        let mut text = Utf8Stream::default();
        let mut buf = vec![0u8; READ_BUFFER];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = text.push(&buf[..n]);
                    if !chunk.is_empty() {
                        sink.output(chunk);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                // Linux reports EIO once the program and its children are gone.
                Err(_) => break,
            }
        }
        let rest = text.finish();
        if !rest.is_empty() {
            sink.output(rest);
        }
        let code = waiter.join().ok().flatten();
        let closed = {
            let mut inner = self.lock();
            let before = inner.closing.len();
            inner.closing.retain(|c| c != id);
            inner.closing.len() != before
        };
        tracing::info!(session = %id, code = ?code, closed, "terminal ended");
        sink.exited(if closed { None } else { code });
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn wait(child: &mut (dyn Child + Send + Sync)) -> Option<u32> {
    match child.wait() {
        Ok(status) => Some(status.exit_code()),
        Err(e) => {
            tracing::warn!(error = %e, "waiting for the terminal program failed");
            None
        }
    }
}
