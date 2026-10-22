//! Contract 028 (`longhorn-pty`): a live `/bin/sh` session with output, a
//! stdin round trip, a resize the child sees, the exit code, and a close that
//! escalates.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use longhorn_pty::{CLOSE_GRACE_PERIOD, CommandSpec, ExitResult, PtyEvent, PtySession, PtySize};

/// How long one session may take before the proof calls it a hang.
const DEADLINE: Duration = Duration::from_secs(20);

fn shell(directory: &Path, script: &str) -> CommandSpec {
    CommandSpec {
        program: Path::new("/bin/sh").to_path_buf(),
        arguments: vec![OsString::from("-c"), OsString::from(script)],
        environment: [
            (OsString::from("PATH"), OsString::from("/usr/bin:/bin")),
            (OsString::from("TERM"), OsString::from("xterm-256color")),
        ]
        .into_iter()
        .collect::<BTreeMap<_, _>>(),
        working_directory: directory.to_path_buf(),
        size: PtySize {
            rows: 24,
            columns: 80,
        },
    }
}

fn next(events: &Receiver<PtyEvent>, deadline: Instant) -> PtyEvent {
    let remaining = deadline.saturating_duration_since(Instant::now());
    assert!(!remaining.is_zero(), "the PTY event deadline expired");
    events
        .recv_timeout(remaining)
        .expect("a PTY event arrives before the deadline")
}

/// Collects output until `text` appears in it.
fn until_text(events: &Receiver<PtyEvent>, text: &str, deadline: Instant) -> Vec<u8> {
    let mut output = Vec::new();
    loop {
        match next(events, deadline) {
            PtyEvent::Output(bytes) => output.extend(bytes),
            PtyEvent::Failure(failure) => panic!("unexpected PTY failure: {failure}"),
            PtyEvent::Exit(exit) => panic!("the child exited before {text:?}: {exit:?}"),
        }
        if String::from_utf8_lossy(&output).contains(text) {
            return output;
        }
    }
}

/// Collects output until the child's one exit event.
fn until_exit(events: &Receiver<PtyEvent>, deadline: Instant) -> (ExitResult, Vec<u8>) {
    let mut output = Vec::new();
    loop {
        match next(events, deadline) {
            PtyEvent::Output(bytes) => output.extend(bytes),
            PtyEvent::Failure(failure) => panic!("unexpected PTY failure: {failure}"),
            PtyEvent::Exit(exit) => return (exit, output),
        }
    }
}

pub(crate) fn run() -> String {
    let directory = tempfile::tempdir().expect("a working directory");
    let deadline = Instant::now() + DEADLINE;

    // Output, a stdin round trip, a resize the child reads back, and the exit
    // code of a real child on a real pseudo-terminal.
    let (session, events) = PtySession::open(shell(
        directory.path(),
        "stty -echo; printf READY; read line; printf 'got:%s\\n' \"$line\"; stty size",
    ))
    .expect("the shell starts on a pseudo-terminal");
    let ready = until_text(&events, "READY", deadline);
    session
        .resize(PtySize {
            rows: 40,
            columns: 100,
        })
        .expect("the terminal and process group resize");
    session
        .write(b"round-trip\n")
        .expect("input reaches the child");
    let (exit, output) = until_exit(&events, deadline);
    assert_eq!(exit, ExitResult::Code(0), "the child's exit code");
    let text = String::from_utf8_lossy(&[ready, output].concat()).into_owned();
    assert!(
        text.contains("READY"),
        "the ready marker came first: {text:?}"
    );
    assert!(
        text.contains("got:round-trip"),
        "the child received the written line: {text:?}"
    );
    assert!(
        text.contains("40 100"),
        "the child saw the requested size: {text:?}"
    );
    assert_eq!(
        exit.compatibility_code(),
        0,
        "the shell-style code agrees with the typed exit"
    );
    drop(session);

    // A close escalates to SIGKILL after the bounded grace period when the
    // child ignores SIGHUP.
    let (escalating, events) = PtySession::open(shell(
        directory.path(),
        "trap '' HUP; printf READY; while :; do sleep 1; done",
    ))
    .expect("a second shell starts on a pseudo-terminal");
    let _ready = until_text(&events, "READY", deadline);
    let started = Instant::now();
    escalating.close().expect("the process group closes");
    assert!(
        started.elapsed() >= CLOSE_GRACE_PERIOD,
        "close waited out its grace period: {:?}",
        started.elapsed()
    );
    let (exit, _) = until_exit(&events, deadline);
    assert_eq!(
        exit,
        ExitResult::Signal(9),
        "an unresponsive child is killed after the grace period"
    );
    assert_eq!(
        escalating.write(b"late"),
        Err(longhorn_pty::Failure::Ended),
        "a closed session accepts no input"
    );

    format!(
        "live /bin/sh session: output, stdin round trip, a resize the child reads back, exit 0, and a close that escalates to signal 9 after {CLOSE_GRACE_PERIOD:?}"
    )
}
