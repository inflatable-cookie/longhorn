//! Dedicated-process integration test proving PTY descriptors are released on
//! `close()` even when the child exited naturally while the receiver was idle
//! and remains held.
//!
//! This lives in its own integration test binary so `/dev/fd` (or
//! `/proc/self/fd`) counts are not perturbed by concurrent sessions in
//! `tests/sessions.rs`.

#![cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "illumos"
))]

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use longhorn_pty::{
    CommandSpec, ExitResult, Failure, OUTPUT_QUEUE_CAPACITY, PtyEvent, PtySession, PtySize,
};

fn open_descriptor_count() -> usize {
    let directory = if Path::new("/dev/fd").exists() {
        "/dev/fd"
    } else {
        "/proc/self/fd"
    };
    std::fs::read_dir(directory)
        .expect("process descriptor directory is readable")
        .count()
}

#[test]
fn close_releases_all_pty_descriptors_after_natural_exit_with_held_receiver() {
    let directory = tempfile::tempdir().expect("a unique test directory is available");
    let done_marker = directory.path().join("child-exited");
    let baseline_fds = open_descriptor_count();

    // Emit exactly OUTPUT_QUEUE_CAPACITY + 1 separate small writes so the
    // reader fills the 8-event queue and blocks in `send_output` on the 9th
    // chunk while leaving the PTY kernel buffer empty (on macOS, unread bytes
    // in the slave buffer would block `exit(0)` in `tty_drain`).
    let script = "i=0; while [ $i -lt 9 ]; do printf 'chunk-%02d\\n' \"$i\"; sleep 0.02; i=$((i + 1)); done; sleep 0.05; printf done > \"$DONE_MARKER\"; exit 0";
    let spec = CommandSpec {
        program: PathBuf::from("/bin/sh"),
        arguments: vec![OsString::from("-c"), OsString::from(script)],
        environment: [
            (OsString::from("PATH"), OsString::from("/usr/bin:/bin")),
            (
                OsString::from("DONE_MARKER"),
                done_marker.as_os_str().to_owned(),
            ),
        ]
        .into_iter()
        .collect::<BTreeMap<_, _>>(),
        working_directory: PathBuf::from("/"),
        size: PtySize {
            rows: 24,
            columns: 80,
        },
    };

    let (session, receiver) = PtySession::open(spec).expect("the child starts on a PTY");

    // Wait for the child to finish writing and exit naturally while `receiver`
    // stays completely unread.
    let marker_deadline = Instant::now() + Duration::from_secs(10);
    while !done_marker.exists() {
        assert!(
            Instant::now() < marker_deadline,
            "child did not reach its exit marker before deadline"
        );
        thread::sleep(Duration::from_millis(10));
    }
    // Wait until the reaper has observed the natural exit while `receiver`
    // stays completely unread. An empty write polls exit state without writing
    // bytes or closing the session.
    let ended_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match session.write(b"") {
            Ok(()) => {
                assert!(
                    Instant::now() < ended_deadline,
                    "reaper did not observe natural child exit before deadline"
                );
                thread::sleep(Duration::from_millis(10));
            }
            Err(Failure::Ended) => break,
            Err(other) => {
                panic!("unexpected write failure while waiting for exit: {other} ({other:?})")
            }
        }
    }

    // Before `close()`, the session still holds `control_master`, and the
    // back-pressured reader still holds `reader_master` and `retained_slave`.
    let held_fds = open_descriptor_count();
    assert!(
        held_fds >= baseline_fds + 3,
        "expected three PTY descriptors held before close(): baseline={baseline_fds}, held={held_fds}"
    );

    assert_eq!(
        session.close(),
        Err(Failure::Ended),
        "close() preserves Ended when the child already exited"
    );

    // Without reading or dropping `receiver`, all three PTY descriptors
    // (`control_master`, `reader_master`, `retained_slave`) must close and
    // return the process descriptor count to its baseline.
    let fd_deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let current_fds = open_descriptor_count();
        if current_fds <= baseline_fds {
            break;
        }
        assert!(
            Instant::now() < fd_deadline,
            "PTY descriptors stayed open after close() returned Ended while receiver was held: baseline={baseline_fds}, current={current_fds}"
        );
        thread::sleep(Duration::from_millis(10));
    }

    // Dropping the session keeps the descriptor count at baseline, and
    // draining the held receiver still delivers the buffered chunks + Exit(0).
    drop(session);
    assert!(
        open_descriptor_count() <= baseline_fds,
        "descriptor count remained at or below baseline after dropping session"
    );

    let drain_deadline = Instant::now() + Duration::from_secs(5);
    let mut output_events = 0;
    let exit = loop {
        let remaining = drain_deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "timed out draining receiver");
        match receiver
            .recv_timeout(remaining)
            .expect("queued events and exit arrive")
        {
            PtyEvent::Output(_) => output_events += 1,
            PtyEvent::Failure(failure) => {
                panic!("unexpected PTY failure: {failure} ({failure:?})")
            }
            PtyEvent::Exit(exit) => break exit,
        }
    };
    assert!(
        output_events <= OUTPUT_QUEUE_CAPACITY,
        "at most OUTPUT_QUEUE_CAPACITY queued chunks remain after close wake (got {output_events})"
    );
    assert_eq!(exit, ExitResult::Code(0));
}
