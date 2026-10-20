//! In-process concurrency stress for `longhorn-pty`.
//!
//! The sessions opened here all live in this one test process at the same
//! time, so a signal or reap that reaches past its own session corrupts a
//! neighbour and fails the run. Every session also prints a unique marker
//! before it ends, so unread output lost to a closed pseudo-terminal fails
//! the run as well. This file must stay green under `cargo test` (threads in
//! one process), not only under nextest (one process per test).
//!
//! Peak live sessions stay small (churn and close workers plus one burst at
//! a time) so the test cannot exhaust the host's pseudo-terminals; pressure
//! comes from churn (hundreds of sessions while close escalations are in
//! flight), synchronized open bursts, and fork-and-exec storms that hold no
//! pseudo-terminal themselves.

#![cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "illumos"
))]

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};

use longhorn_pty::{CommandSpec, ExitResult, PtyEvent, PtySession, PtySize};

fn shell(script: &str) -> CommandSpec {
    CommandSpec {
        program: PathBuf::from("/bin/sh"),
        arguments: vec![OsString::from("-c"), OsString::from(script)],
        environment: [(OsString::from("PATH"), OsString::from("/usr/bin:/bin"))]
            .into_iter()
            .collect::<BTreeMap<_, _>>(),
        working_directory: PathBuf::from("/"),
        size: PtySize {
            rows: 24,
            columns: 80,
        },
    }
}

fn receive_until_exit(receiver: &Receiver<PtyEvent>) -> (ExitResult, Vec<u8>) {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut output = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "the PTY event deadline expired");
        match receiver
            .recv_timeout(remaining)
            .expect("the PTY event arrives before its explicit deadline")
        {
            PtyEvent::Output(bytes) => output.extend(bytes),
            PtyEvent::Failure(failure) => panic!("unexpected PTY failure: {failure:?}"),
            PtyEvent::Exit(exit) => return (exit, output),
        }
    }
}

fn receive_until_text(receiver: &Receiver<PtyEvent>, text: &str) {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut output = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "the PTY event deadline expired");
        match receiver
            .recv_timeout(remaining)
            .expect("the PTY event arrives before its explicit deadline")
        {
            PtyEvent::Output(bytes) => output.extend(bytes),
            PtyEvent::Failure(failure) => panic!("unexpected PTY failure: {failure:?}"),
            PtyEvent::Exit(exit) => panic!("child exited before readiness: {exit:?}"),
        }
        if String::from_utf8_lossy(&output).contains(text) {
            return;
        }
    }
}

fn check_exit(id: usize, code: i32) {
    let marker = format!("marker-exit-{id}");
    let (_session, receiver) = PtySession::open(shell(&format!("printf '{marker}'; exit {code}")))
        .expect("the exit child starts on a PTY");
    let (exit, output) = receive_until_exit(&receiver);
    let text = String::from_utf8_lossy(&output);
    assert!(
        text.contains(&marker),
        "an exit-{code} session lost its output: {text:?}"
    );
    assert_eq!(
        exit,
        ExitResult::Code(code),
        "an exit-{code} session reported {exit:?}"
    );
}

fn check_signal(id: usize, signal: i32) {
    let marker = format!("marker-signal-{id}");
    let (_session, receiver) =
        PtySession::open(shell(&format!("printf '{marker}'; kill -{signal} $$")))
            .expect("the signal child starts on a PTY");
    let (exit, output) = receive_until_exit(&receiver);
    let text = String::from_utf8_lossy(&output);
    assert!(
        text.contains(&marker),
        "a self-signal-{signal} session lost its output: {text:?}"
    );
    assert_eq!(
        exit,
        ExitResult::Signal(signal),
        "a self-signal-{signal} session reported {exit:?}"
    );
}

fn check_close(id: usize) {
    let marker = format!("marker-close-{id}");
    let (session, receiver) = PtySession::open(shell(&format!(
        "printf '{marker}'; trap '' HUP; while :; do sleep 1; done"
    )))
    .expect("the close child starts on a PTY");
    // The marker arriving here proves the session was alive and streaming
    // before close; nothing is written after it, so the post-close output
    // is legitimately empty and only the exit is asserted below.
    receive_until_text(&receiver, &marker);
    session.close().expect("the process group closes");
    let (exit, _) = receive_until_exit(&receiver);
    assert_eq!(
        exit,
        ExitResult::Signal(9),
        "a closed session reported {exit:?}"
    );
}

/// One synchronized burst: `count` threads open their sessions at the same
/// instant, so dozens of children fork and dozens of reader threads spawn
/// in the same millisecond. A reader whose first run is delayed past its
/// child's write and death finds an empty master and the marker is lost
/// unless the parent kept the slave open past the final drain.
fn burst_round(base: usize, count: usize) {
    use std::sync::{Arc, Barrier};
    let barrier = Arc::new(Barrier::new(count));
    let mut handles = Vec::new();
    for i in 0..count {
        let barrier = barrier.clone();
        handles.push(thread::spawn(move || {
            let id = base + i;
            let marker = format!("marker-burst-{id}");
            let code = (id % 124 + 1) as i32;
            barrier.wait();
            let (_session, receiver) =
                PtySession::open(shell(&format!("printf '{marker}'; exit {code}")))
                    .expect("the burst child starts on a PTY");
            let (exit, output) = receive_until_exit(&receiver);
            let text = String::from_utf8_lossy(&output);
            assert!(
                text.contains(&marker),
                "a burst session lost its output: {text:?}"
            );
            assert_eq!(
                exit,
                ExitResult::Code(code),
                "a burst session reported {exit:?}"
            );
        }));
    }
    for handle in handles {
        handle.join().expect("the burst thread finishes");
    }
}

/// Scheduler pressure without pseudo-terminal pressure: tight fork-and-exec
/// churn in plain child processes (no PTY) stalls thread startup across the
/// process, including session reader threads, while holding no PTY itself.
fn storm_worker(stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Relaxed) {
        let mut child = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg("exit 0")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("the storm child starts");
        let _ = child.wait();
    }
}

#[test]
fn many_concurrent_sessions_report_their_own_exits() {
    let mut handles = Vec::new();
    for worker in 0..3 {
        handles.push(thread::spawn(move || {
            for round in 0..50 {
                let id = worker * 50 + round;
                check_exit(id, (id % 124 + 1) as i32);
                if round % 10 == 0 {
                    check_signal(id, 15);
                }
            }
        }));
    }
    for id in 0..2 {
        handles.push(thread::spawn(move || {
            for _ in 0..4 {
                check_close(id);
            }
        }));
    }
    let stop = Arc::new(AtomicBool::new(false));
    let mut storms = Vec::new();
    for _ in 0..4 {
        let stop = stop.clone();
        storms.push(thread::spawn(move || storm_worker(stop)));
    }
    for round in 0..40 {
        burst_round(100_000 + round * 4, 4);
    }
    stop.store(true, Ordering::Relaxed);
    for storm in storms {
        storm.join().expect("the storm thread finishes");
    }
    for handle in handles {
        handle.join().expect("the worker thread finishes");
    }
}
