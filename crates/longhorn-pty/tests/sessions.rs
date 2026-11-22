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
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};

use longhorn_pty::{
    CLOSE_GRACE_PERIOD, CommandSpec, ExitResult, Failure, MAX_BUFFERED_OUTPUT_BYTES,
    OUTPUT_CHUNK_BYTES, OUTPUT_QUEUE_CAPACITY, PtyEvent, PtySession, PtySize,
};
use rustix::pty::OpenptFlags;

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
    match PtySession::open(spec) {
        Ok(opened) => opened,
        Err(failure) => panic!(
            "the real child starts on a PTY: {failure} ({failure:?}){}",
            open_failure_os_evidence()
        ),
    }
}

fn probe_openpt_flags() -> OpenptFlags {
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        OpenptFlags::RDWR | OpenptFlags::NOCTTY | OpenptFlags::CLOEXEC
    }
    #[cfg(any(target_os = "macos", target_os = "illumos"))]
    {
        OpenptFlags::RDWR
            | OpenptFlags::NOCTTY
            | OpenptFlags::from_bits_retain(rustix::fs::OFlags::CLOEXEC.bits())
    }
    #[cfg(not(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "macos",
        target_os = "illumos"
    )))]
    {
        OpenptFlags::RDWR | OpenptFlags::NOCTTY
    }
}

/// Reports the OS state at the moment an open failure surfaced.
///
/// `Failure` carries the raw OS error code of the failed step, and this
/// evidence-only probe uses the same master-open flags as production `open`
/// plus the current process descriptor count so a successful probe cannot mask
/// a flag mismatch. It runs once on the already-failed path, never retries
/// `PtySession::open`, and never gates success.
fn open_failure_os_evidence() -> String {
    let master_probe = match rustix::pty::openpt(probe_openpt_flags()) {
        Ok(descriptor) => {
            let descriptor_number = descriptor.as_raw_fd();
            drop(descriptor);
            format!(
                "a direct PTY master open succeeded (fd {descriptor_number}, closed): \
                 the host still had a free pseudo-terminal at evidence time"
            )
        }
        Err(errno) => format!(
            "a direct PTY master open failed (errno {errno}, raw {})",
            errno.raw_os_error()
        ),
    };
    let descriptor_directory = if Path::new("/dev/fd").exists() {
        "/dev/fd"
    } else {
        "/proc/self/fd"
    };
    let descriptor_line = match std::fs::read_dir(descriptor_directory) {
        Ok(entries) => format!(
            "process descriptors open now: {} (via {descriptor_directory})",
            entries.count()
        ),
        Err(error) => format!(
            "process descriptor count unavailable (reading {descriptor_directory} failed: {error})"
        ),
    };
    format!("\n  same-moment host evidence: {master_probe}; {descriptor_line}")
}

/// Drains remaining events after an unexpected operation failure so the panic
/// records any output, async failures, and the child's exit status.
fn drain_events_for_failure(receiver: &Receiver<PtyEvent>) -> String {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut output = Vec::new();
    let mut failures = Vec::new();
    let mut exit = None;
    let mut disconnected = false;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match receiver.recv_timeout(remaining) {
            Ok(PtyEvent::Output(bytes)) => output.extend(bytes),
            Ok(PtyEvent::Failure(failure)) => failures.push(failure),
            Ok(PtyEvent::Exit(result)) => {
                exit = Some(result);
                break;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                disconnected = true;
                break;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
        }
    }
    format!(
        "\n  drained events: exit={exit:?}, failures={failures:?}, disconnected={disconnected}, output={:?}",
        text(&output)
    )
}

fn write_or_diagnose(
    session: &PtySession,
    receiver: &Receiver<PtyEvent>,
    bytes: &[u8],
    context: &str,
) {
    if let Err(failure) = session.write(bytes) {
        panic!(
            "{context}: write failed with {failure} ({failure:?}){}",
            drain_events_for_failure(receiver)
        );
    }
}

fn resize_or_diagnose(
    session: &PtySession,
    receiver: &Receiver<PtyEvent>,
    size: PtySize,
    context: &str,
) {
    if let Err(failure) = session.resize(size) {
        panic!(
            "{context}: resize failed with {failure} ({failure:?}){}",
            drain_events_for_failure(receiver)
        );
    }
}

fn close_or_diagnose(session: &PtySession, receiver: &Receiver<PtyEvent>, context: &str) {
    if let Err(failure) = session.close() {
        panic!(
            "{context}: close failed with {failure} ({failure:?}){}",
            drain_events_for_failure(receiver)
        );
    }
}

fn receive_until_text(receiver: &Receiver<PtyEvent>, text_marker: &str) -> Vec<u8> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut output = Vec::new();
    loop {
        match receive(receiver, deadline) {
            PtyEvent::Output(bytes) => output.extend(bytes),
            PtyEvent::Failure(failure) => panic!(
                "unexpected PTY failure before {text_marker:?}: {failure} ({failure:?}); output={:?}{}",
                text(&output),
                drain_events_for_failure(receiver)
            ),
            PtyEvent::Exit(exit) => panic!(
                "child exited before readiness {text_marker:?}: {exit:?}; output={:?}",
                text(&output)
            ),
        }
        if String::from_utf8_lossy(&output).contains(text_marker) {
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
            PtyEvent::Failure(failure) => panic!(
                "unexpected PTY failure before exit: {failure} ({failure:?}); output={:?}{}",
                text(&output),
                drain_events_for_failure(receiver)
            ),
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
    write_or_diagnose(
        &session,
        &receiver,
        b"round-trip-input\n",
        "stdin_round_trips_through_the_terminal",
    );
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
    resize_or_diagnose(
        &session,
        &receiver,
        PtySize {
            rows: 40,
            columns: 100,
        },
        "resize_is_visible_to_the_child",
    );
    write_or_diagnose(
        &session,
        &receiver,
        b"go\n",
        "resize_is_visible_to_the_child",
    );
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
    let parent_pid = std::process::id();
    let (session, receiver) = open(shell(&format!(
        "trap '' HUP; printf READY; while kill -0 {parent_pid} 2>/dev/null; do sleep 1; done"
    )));
    let _ready = receive_until_text(&receiver, "READY");
    let started = Instant::now();
    close_or_diagnose(
        &session,
        &receiver,
        "close_escalates_when_the_child_ignores_hangup",
    );
    assert!(started.elapsed() >= CLOSE_GRACE_PERIOD);
    let (exit, _) = receive_until_exit(&receiver);
    assert_eq!(exit, ExitResult::Signal(9));
}

#[test]
fn close_escalates_to_a_grandchild_in_the_same_process_group() {
    let parent_pid = std::process::id();
    let directory = tempfile::tempdir().expect("a unique test directory is available");
    let marker = directory.path().join("grandchild-survived");
    // Self-terminates after 1s even if the test process is killed mid-run.
    let script = format!(
        "trap '' HUP; (trap '' HUP; sleep 1; if kill -0 {parent_pid} 2>/dev/null; then printf alive > \"$MARKER\"; fi) & printf READY; wait"
    );
    let mut spec = shell(&script);
    spec.environment
        .insert(OsString::from("MARKER"), marker.as_os_str().to_owned());
    let (session, receiver) = open(spec);
    let _ready = receive_until_text(&receiver, "READY");
    close_or_diagnose(
        &session,
        &receiver,
        "close_escalates_to_a_grandchild_in_the_same_process_group",
    );
    let (exit, _) = receive_until_exit(&receiver);
    assert_eq!(exit, ExitResult::Signal(9));
    thread::sleep(Duration::from_millis(1_100));
    assert!(!marker.exists(), "the grandchild was killed with its group");
}

/// HUP ends the shell itself, so the group is only a zombie when close probes
/// it after the grace period. Close must count that group as closed.
#[test]
fn close_succeeds_when_hangup_ends_the_group() {
    let parent_pid = std::process::id();
    let (session, receiver) = open(shell(&format!(
        "printf READY; while kill -0 {parent_pid} 2>/dev/null; do sleep 1; done"
    )));
    let _ready = receive_until_text(&receiver, "READY");
    close_or_diagnose(
        &session,
        &receiver,
        "close_succeeds_when_hangup_ends_the_group",
    );
    let (exit, _) = receive_until_exit(&receiver);
    assert_eq!(exit, ExitResult::Signal(1));
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
    write_or_diagnose(
        &first,
        &first_events,
        b"alpha\n",
        "two_sessions_have_independent_input_and_output (first)",
    );
    write_or_diagnose(
        &second,
        &second_events,
        b"beta\n",
        "two_sessions_have_independent_input_and_output (second)",
    );
    let (first_exit, first_output) = receive_until_exit(&first_events);
    let (second_exit, second_output) = receive_until_exit(&second_events);
    assert_eq!(first_exit, ExitResult::Code(0));
    assert_eq!(second_exit, ExitResult::Code(0));
    assert!(text(&[first_ready, first_output].concat()).contains("first:alpha"));
    assert!(text(&[second_ready, second_output].concat()).contains("second:beta"));
}

#[test]
fn a_slow_consumer_applies_backpressure_without_growing_output_memory() {
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
            PtyEvent::Failure(failure) => panic!("unexpected PTY failure: {failure} ({failure:?})"),
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
fn close_wakes_a_blocked_reader_when_the_consumer_stops_reading() {
    let (session, receiver) = open(shell("exec /usr/bin/head -c 1048576 /dev/zero"));
    // Let the child fill the bounded output queue and stall the reader in send.
    thread::sleep(Duration::from_millis(200));
    close_or_diagnose(
        &session,
        &receiver,
        "close_wakes_a_blocked_reader_when_the_consumer_stops_reading",
    );
    // Give the woken reader a brief moment to drop its PTY descriptors and
    // queue the terminal exit event before we touch the receiver.
    thread::sleep(Duration::from_millis(50));

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut output_events = 0;
    let exit = loop {
        match receive(&receiver, deadline) {
            PtyEvent::Output(_) => output_events += 1,
            PtyEvent::Failure(failure) => panic!("unexpected PTY failure: {failure} ({failure:?})"),
            PtyEvent::Exit(exit) => break exit,
        }
    };
    assert!(
        output_events <= OUTPUT_QUEUE_CAPACITY,
        "close woke the blocked reader before draining further chunks (got {output_events} chunks, capacity {OUTPUT_QUEUE_CAPACITY})"
    );
    assert!(
        matches!(exit, ExitResult::Signal(1 | 9)),
        "closed child exited via HUP or KILL, got {exit:?}"
    );
}

#[test]
fn setup_failures_carry_raw_os_error_without_leaking_inputs() {
    let secret_program = "/nonexistent/longhorn-pty-secret-program-name";
    let mut spec = command(secret_program, &[], &[("SECRET_TOKEN", "hunter2")]);
    spec.working_directory = PathBuf::from("/");
    let failure = PtySession::open(spec).expect_err("missing program fails to spawn");
    let raw = failure
        .raw_os_error()
        .expect("spawn failure carries raw_os_error");
    assert_eq!(
        failure,
        Failure::Spawn {
            raw_os_error: Some(raw),
        }
    );
    assert_eq!(raw, rustix::io::Errno::NOENT.raw_os_error());
    let rendered = failure.to_string();
    assert!(
        rendered.contains(&format!("(os error {raw})")),
        "Display includes numeric os error: {rendered:?}"
    );
    assert!(
        !rendered.contains(secret_program) && !rendered.contains("hunter2"),
        "Display does not leak paths or environment values: {rendered:?}"
    );
}

#[test]
fn a_relative_working_directory_is_applied() {
    let relative = PathBuf::from("src");
    assert!(
        !relative.is_absolute(),
        "test fixture must pass a relative working directory"
    );
    let expected = std::fs::canonicalize(&relative).expect("crate src directory exists");
    let mut spec = shell("pwd -P");
    spec.working_directory = relative;
    let (_session, receiver) = open(spec);
    let (exit, output) = receive_until_exit(&receiver);
    assert_eq!(exit, ExitResult::Code(0));
    assert_eq!(text(&output).trim(), expected.to_string_lossy());
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
    let parent_pid = std::process::id();
    let marker = format!("marker-close-{id}");
    // The trap comes before the marker. Otherwise a hangup that lands between
    // them ends the shell itself and close sees a zombie-only group, which is
    // a different case from the escalation this check covers.
    let (session, receiver) = open(shell(&format!(
        "trap '' HUP; printf '{marker}'; while kill -0 {parent_pid} 2>/dev/null; do sleep 1; done"
    )));
    // The marker arriving here proves the session was alive, streaming, and
    // ignoring hangup before close; nothing is written after it, so the
    // post-close output is legitimately empty and only the exit is asserted
    // below.
    receive_until_text(&receiver, &marker);
    close_or_diagnose(&session, &receiver, &format!("check_stress_close({id})"));
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
