//! Real pseudo-terminals: `/bin/sh` on Linux, `cmd.exe` under ConPTY on Windows.

use std::ffi::OsString;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use vagrant_studio_terminal::{Size, SpawnRequest, TerminalSink, Terminals};

#[derive(Default)]
struct Collect {
    output: Mutex<String>,
    exit: Mutex<Option<Sender<Option<u32>>>>,
}

impl TerminalSink for Collect {
    fn output(&self, data: String) {
        self.output.lock().unwrap().push_str(&data);
    }
    fn exited(&self, code: Option<u32>) {
        if let Some(tx) = self.exit.lock().unwrap().take() {
            tx.send(code).unwrap();
        }
    }
}

fn sink() -> (Arc<Collect>, Receiver<Option<u32>>) {
    let (tx, rx) = channel();
    let sink = Arc::new(Collect::default());
    *sink.exit.lock().unwrap() = Some(tx);
    (sink, rx)
}

/// A command line for the host's shell.
fn shell(script: &str) -> SpawnRequest {
    let (program, args): (&str, Vec<&str>) = if cfg!(windows) {
        ("cmd.exe", vec!["/d", "/v:on", "/c", script])
    } else {
        ("/bin/sh", vec!["-c", script])
    };
    SpawnRequest {
        program: program.into(),
        args: args.into_iter().map(OsString::from).collect(),
        cwd: std::env::temp_dir(),
        env: vec![("VAGRANT_STUDIO_TERMINAL_TEST".into(), "yes".into())],
        size: Size {
            cols: 100,
            rows: 30,
        },
    }
}

fn wait_for(sink: &Collect, needle: &str) {
    let start = Instant::now();
    while !sink.output.lock().unwrap().contains(needle) {
        assert!(
            start.elapsed() < Duration::from_secs(20),
            "{needle:?} not seen in {:?}",
            sink.output.lock().unwrap()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// ConPTY starts by asking for the cursor position (`ESC[6n`) and waits for
/// the answer; xterm.js gives it in the app, the tests answer here.
fn start(terminals: &Terminals, req: SpawnRequest, sink: &Arc<Collect>) -> String {
    let id = terminals.spawn(req, sink.clone()).unwrap();
    if cfg!(windows) {
        wait_for(sink, "\x1b[6n");
        terminals.write(&id, b"\x1b[1;1R").unwrap();
    }
    id
}

const EXIT_WAIT: Duration = Duration::from_secs(20);

#[test]
fn runs_a_program_with_input_environment_and_exit_code() {
    let terminals = Terminals::new();
    let (sink, exit) = sink();
    let script = if cfg!(windows) {
        "echo ready %VAGRANT_STUDIO_TERMINAL_TEST%& set /p line=& echo got !line!& exit /b 3"
    } else {
        "echo ready $VAGRANT_STUDIO_TERMINAL_TEST; read line; echo got $line; exit 3"
    };
    let id = start(&terminals, shell(script), &sink);
    wait_for(&sink, "ready yes");
    terminals.write(&id, b"abc\r").unwrap();
    assert_eq!(exit.recv_timeout(EXIT_WAIT).unwrap(), Some(3));
    assert!(sink.output.lock().unwrap().contains("got abc"));
    assert_eq!(terminals.open_sessions(), 0);
    assert!(terminals.write(&id, b"x").is_err());
}

#[test]
fn closing_ends_the_program() {
    let terminals = Terminals::new();
    let (sink, exit) = sink();
    let script = if cfg!(windows) {
        "echo started& ping -n 60 127.0.0.1 >nul"
    } else {
        "echo started; sleep 60"
    };
    let id = start(&terminals, shell(script), &sink);
    wait_for(&sink, "started");
    let start = Instant::now();
    terminals.close(&id);
    assert_eq!(exit.recv_timeout(EXIT_WAIT).unwrap(), None);
    assert!(start.elapsed() < Duration::from_secs(10));
    assert_eq!(terminals.open_sessions(), 0);
    // Closing again is harmless.
    terminals.close(&id);
}

#[test]
fn resizing_reaches_the_program() {
    let terminals = Terminals::new();
    let (sink, exit) = sink();
    let script = if cfg!(windows) {
        "echo waiting& set /p line=& mode con"
    } else {
        "echo waiting; read line; stty size"
    };
    let id = start(&terminals, shell(script), &sink);
    wait_for(&sink, "waiting");
    terminals
        .resize(
            &id,
            Size {
                cols: 120,
                rows: 40,
            },
        )
        .unwrap();
    terminals.write(&id, b"\r").unwrap();
    exit.recv_timeout(EXIT_WAIT).unwrap();
    let out = sink.output.lock().unwrap().clone();
    if cfg!(windows) {
        assert!(out.contains("40") && out.contains("120"), "{out:?}");
    } else {
        assert!(out.contains("40 120"), "{out:?}");
    }
}

#[test]
fn limits_open_sessions() {
    let terminals = Terminals::new();
    let script = if cfg!(windows) {
        "ping -n 60 127.0.0.1 >nul"
    } else {
        "sleep 60"
    };
    let ids: Vec<String> = (0..vagrant_studio_terminal::MAX_SESSIONS)
        .map(|_| terminals.spawn(shell(script), sink().0).unwrap())
        .collect();
    assert!(terminals.spawn(shell(script), sink().0).is_err());
    terminals.close_all();
    assert_eq!(terminals.open_sessions(), 0);
    assert_eq!(ids.len(), vagrant_studio_terminal::MAX_SESSIONS);
}

#[test]
fn a_missing_program_is_an_error() {
    let terminals = Terminals::new();
    let mut req = shell("");
    req.program = std::env::temp_dir().join("no-such-program-vagrant-studio");
    assert!(terminals.spawn(req, sink().0).is_err());
}
