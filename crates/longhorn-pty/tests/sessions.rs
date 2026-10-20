//! Public-API integration tests using real pseudo-terminal child processes.

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

use longhorn_pty::{
    CLOSE_GRACE_PERIOD, CommandSpec, ExitResult, Failure, MAX_BUFFERED_OUTPUT_BYTES,
    OUTPUT_CHUNK_BYTES, OUTPUT_QUEUE_CAPACITY, PtyEvent, PtySession, PtySize,
};

fn command(program: &str, arguments: &[&str], environment: &[(&str, &str)]) -> CommandSpec {
    CommandSpec {
        program: PathBuf::from(program),
        arguments: arguments.iter().map(OsString::from).collect(),
        environment: environment
            .iter()
            .map(|(name, value)| (OsString::from(name), OsString::from(value)))
            .collect::<BTreeMap<_, _>>(),
        working_directory: PathBuf::from("/"),
        size: PtySize {
            rows: 24,
            columns: 80,
        },
    }
}

fn shell(script: &str) -> CommandSpec {
    command("/bin/sh", &["-c", script], &[("PATH", "/usr/bin:/bin")])
}

fn open(spec: CommandSpec) -> (PtySession, Receiver<PtyEvent>) {
    PtySession::open(spec).expect("the real child starts on a PTY")
}

fn receive_until_text(receiver: &Receiver<PtyEvent>, text: &str) -> Vec<u8> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut output = Vec::new();
    loop {
        match receive(receiver, deadline) {
            PtyEvent::Output(bytes) => output.extend(bytes),
            PtyEvent::Failure(failure) => panic!("unexpected PTY failure: {failure:?}"),
            PtyEvent::Exit(exit) => panic!("child exited before readiness: {exit:?}"),
        }
        if String::from_utf8_lossy(&output).contains(text) {
            return output;
        }
    }
}

fn receive_until_exit(receiver: &Receiver<PtyEvent>) -> (ExitResult, Vec<u8>) {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut output = Vec::new();
    loop {
        match receive(receiver, deadline) {
            PtyEvent::Output(bytes) => output.extend(bytes),
            PtyEvent::Failure(failure) => panic!("unexpected PTY failure: {failure:?}"),
            PtyEvent::Exit(exit) => return (exit, output),
        }
    }
}

fn receive(receiver: &Receiver<PtyEvent>, deadline: Instant) -> PtyEvent {
    let remaining = deadline.saturating_duration_since(Instant::now());
    assert!(!remaining.is_zero(), "the PTY event deadline expired");
    receiver
        .recv_timeout(remaining)
        .expect("the PTY event arrives before its explicit deadline")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn output_arrives_and_the_exit_result_follows_it_once() {
    let (_session, receiver) = open(shell("printf 'hello-from-pty\\n'"));
    let (exit, output) = receive_until_exit(&receiver);
    assert_eq!(exit, ExitResult::Code(0));
    assert!(text(&output).contains("hello-from-pty"));
    assert!(receiver.try_recv().is_err(), "one exit event was delivered");
}

#[test]
fn stdin_round_trips_through_the_terminal() {
    let (session, receiver) = open(shell(
        "stty -echo; printf READY; read line; printf 'got:%s\\n' \"$line\"",
    ));
    let ready = receive_until_text(&receiver, "READY");
    session
        .write(b"round-trip-input\n")
        .expect("input is written");
    let (exit, output) = receive_until_exit(&receiver);
    assert_eq!(exit, ExitResult::Code(0));
    assert!(text(&[ready, output].concat()).contains("got:round-trip-input"));
}

#[test]
fn child_has_a_controlling_terminal() {
    let (_session, receiver) = open(shell("stty size </dev/tty >/dev/null && tty >/dev/null"));
    let (exit, _) = receive_until_exit(&receiver);
    assert_eq!(exit, ExitResult::Code(0));
}

#[test]
fn resize_is_visible_to_the_child() {
    let (session, receiver) = open(shell("stty -echo; printf READY; read line; stty size"));
    let ready = receive_until_text(&receiver, "READY");
    session
        .resize(PtySize {
            rows: 40,
            columns: 100,
        })
        .expect("the PTY and process group resize");
    session
        .write(b"go\n")
        .expect("child receives the gate input");
    let (exit, output) = receive_until_exit(&receiver);
    assert_eq!(exit, ExitResult::Code(0));
    assert!(text(&[ready, output].concat()).contains("40 100"));
}

#[test]
fn the_environment_is_exactly_the_supplied_map() {
    let (_session, receiver) = open(command("/usr/bin/env", &[], &[("ONLY_THIS", "retained")]));
    let (exit, output) = receive_until_exit(&receiver);
    assert_eq!(exit, ExitResult::Code(0));
    assert_eq!(text(&output).replace("\r\n", "\n"), "ONLY_THIS=retained\n");
}

#[test]
fn exit_codes_and_signals_are_typed_and_map_to_shell_codes() {
    let (_session, receiver) = open(shell("exit 7"));
    let (exit, _) = receive_until_exit(&receiver);
    assert_eq!(exit, ExitResult::Code(7));
    assert_eq!(exit.compatibility_code(), 7);

    let (_session, receiver) = open(shell("kill -TERM $$"));
    let (exit, _) = receive_until_exit(&receiver);
    assert_eq!(exit, ExitResult::Signal(15));
    assert_eq!(exit.compatibility_code(), 128 + 15);
}

#[test]
fn close_escalates_when_the_child_ignores_hangup() {
    let (session, receiver) = open(shell(
        "trap '' HUP; printf READY; while :; do sleep 1; done",
    ));
    let _ready = receive_until_text(&receiver, "READY");
    let started = Instant::now();
    session.close().expect("the process group closes");
    assert!(started.elapsed() >= CLOSE_GRACE_PERIOD);
    let (exit, _) = receive_until_exit(&receiver);
    assert_eq!(exit, ExitResult::Signal(9));
}

#[test]
fn close_escalates_to_a_grandchild_in_the_same_process_group() {
    let directory = tempfile::tempdir().expect("a unique test directory is available");
    let marker = directory.path().join("grandchild-survived");
    let script =
        "trap '' HUP; (trap '' HUP; sleep 1; printf alive > \"$MARKER\") & printf READY; wait";
    let mut spec = shell(script);
    spec.environment
        .insert(OsString::from("MARKER"), marker.as_os_str().to_owned());
    let (session, receiver) = open(spec);
    let _ready = receive_until_text(&receiver, "READY");
    session.close().expect("the child process group closes");
    let (exit, _) = receive_until_exit(&receiver);
    assert_eq!(exit, ExitResult::Signal(9));
    thread::sleep(Duration::from_millis(1_100));
    assert!(!marker.exists(), "the grandchild was killed with its group");
}

#[test]
fn operations_after_exit_are_classified_as_ended() {
    let (session, receiver) = open(shell("exit 0"));
    let (exit, _) = receive_until_exit(&receiver);
    assert_eq!(exit, ExitResult::Code(0));
    assert_eq!(session.write(b"late"), Err(Failure::Ended));
    assert_eq!(
        session.resize(PtySize {
            rows: 50,
            columns: 120,
        }),
        Err(Failure::Ended)
    );
    assert_eq!(session.close(), Err(Failure::Ended));
    assert!(receiver.try_recv().is_err(), "no second exit was sent");
}

#[test]
fn two_sessions_have_independent_input_and_output() {
    let (first, first_events) = open(shell(
        "stty -echo; printf READY1; read line; printf 'first:%s\\n' \"$line\"",
    ));
    let (second, second_events) = open(shell(
        "stty -echo; printf READY2; read line; printf 'second:%s\\n' \"$line\"",
    ));
    let first_ready = receive_until_text(&first_events, "READY1");
    let second_ready = receive_until_text(&second_events, "READY2");
    first
        .write(b"alpha\n")
        .expect("first session accepts input");
    second
        .write(b"beta\n")
        .expect("second session accepts input");
    let (first_exit, first_output) = receive_until_exit(&first_events);
    let (second_exit, second_output) = receive_until_exit(&second_events);
    assert_eq!(first_exit, ExitResult::Code(0));
    assert_eq!(second_exit, ExitResult::Code(0));
    assert!(text(&[first_ready, first_output].concat()).contains("first:alpha"));
    assert!(text(&[second_ready, second_output].concat()).contains("second:beta"));
}

#[test]
fn a_slow_consumer_applies_backpressure_without_growing_output_memory() {
    assert_eq!(MAX_BUFFERED_OUTPUT_BYTES, 9 * OUTPUT_CHUNK_BYTES);
    let (session, receiver) = open(shell("exec /usr/bin/head -c 1048576 /dev/zero"));
    let deadline = Instant::now() + Duration::from_secs(10);
    let first_chunk = match receive(&receiver, deadline) {
        PtyEvent::Output(bytes) => bytes,
        event => panic!("expected output before the child exits: {event:?}"),
    };
    assert!(first_chunk.len() <= OUTPUT_CHUNK_BYTES);
    thread::sleep(Duration::from_millis(250));
    let mut output_bytes = first_chunk.len();
    let mut queued_events = 0;
    for _ in 0..OUTPUT_QUEUE_CAPACITY {
        match receiver.try_recv() {
            Ok(PtyEvent::Output(bytes)) => {
                assert!(bytes.len() <= OUTPUT_CHUNK_BYTES);
                output_bytes += bytes.len();
                queued_events += 1;
            }
            Ok(event) => panic!("unexpected queued event before output drains: {event:?}"),
            Err(_) => break,
        }
    }
    assert!(queued_events <= OUTPUT_QUEUE_CAPACITY);
    assert!(output_bytes <= MAX_BUFFERED_OUTPUT_BYTES);
    loop {
        match receive(&receiver, deadline) {
            PtyEvent::Output(bytes) => {
                assert!(bytes.len() <= OUTPUT_CHUNK_BYTES);
                output_bytes += bytes.len();
            }
            PtyEvent::Failure(failure) => panic!("unexpected PTY failure: {failure:?}"),
            PtyEvent::Exit(exit) => {
                assert_eq!(exit, ExitResult::Code(0));
                break;
            }
        }
    }
    assert_eq!(output_bytes, 1_048_576);
    drop(session);
}

#[test]
fn a_relative_working_directory_is_applied() {
    let mut spec = shell("pwd");
    spec.working_directory = PathBuf::from("/usr");
    let (_session, receiver) = open(spec);
    let (exit, output) = receive_until_exit(&receiver);
    assert_eq!(exit, ExitResult::Code(0));
    assert!(text(&output).contains("/usr"));
}

fn check_stress_exit(id: usize, code: i32) {
    let marker = format!("marker-exit-{id}");
    let (_session, receiver) = open(shell(&format!("printf '{marker}'; exit {code}")));
    let (exit, output) = receive_until_exit(&receiver);
    let body = text(&output);
    assert!(
        body.contains(&marker),
        "an exit-{code} session lost its output: {body:?}"
    );
    assert_eq!(
        exit,
        ExitResult::Code(code),
        "an exit-{code} session reported {exit:?}"
    );
}

fn check_stress_signal(id: usize, signal: i32) {
    let marker = format!("marker-signal-{id}");
    let (_session, receiver) = open(shell(&format!("printf '{marker}'; kill -{signal} $$")));
    let (exit, output) = receive_until_exit(&receiver);
    let body = text(&output);
    assert!(
        body.contains(&marker),
        "a self-signal-{signal} session lost its output: {body:?}"
    );
    assert_eq!(
        exit,
        ExitResult::Signal(signal),
        "a self-signal-{signal} session reported {exit:?}"
    );
}

fn check_stress_close(id: usize) {
    let marker = format!("marker-close-{id}");
    let (session, receiver) = open(shell(&format!(
        "printf '{marker}'; trap '' HUP; while :; do sleep 1; done"
    )));
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

fn burst_round(base: usize, count: usize) {
    use std::sync::Barrier;
    let barrier = Arc::new(Barrier::new(count));
    let mut handles = Vec::new();
    for i in 0..count {
        let barrier = barrier.clone();
        handles.push(thread::spawn(move || {
            let id = base + i;
            let marker = format!("marker-burst-{id}");
            let code = (id % 124 + 1) as i32;
            barrier.wait();
            let (_session, receiver) = open(shell(&format!("printf '{marker}'; exit {code}")));
            let (exit, output) = receive_until_exit(&receiver);
            let body = text(&output);
            assert!(
                body.contains(&marker),
                "a burst session lost its output: {body:?}"
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

/// In-process independence under load: hundreds of sessions share this one
/// test process with the rest of the suite (mixed exits with codes,
/// self-signals, and closes that escalate to KILL) while fork-and-exec
/// storms stall thread startup. Each session asserts its own exit and its
/// own unique output marker. Live sessions stay few at a time (one churn
/// loop, one burst of three) so loaded hosts do not exhaust
/// pseudo-terminals.
#[test]
fn many_concurrent_sessions_share_one_process() {
    let stop = Arc::new(AtomicBool::new(false));
    let mut storms = Vec::new();
    for _ in 0..4 {
        let stop = stop.clone();
        storms.push(thread::spawn(move || storm_worker(stop)));
    }
    for round in 0..200 {
        let id = round;
        check_stress_exit(id, (id % 124 + 1) as i32);
        if round % 12 == 0 {
            check_stress_signal(1_000_000 + id, 15);
        }
        if round % 25 == 0 {
            check_stress_close(2_000_000 + round);
        }
    }
    for round in 0..30 {
        burst_round(100_000 + round * 3, 3);
    }
    stop.store(true, Ordering::Relaxed);
    for storm in storms {
        storm.join().expect("the storm thread finishes");
    }
}
