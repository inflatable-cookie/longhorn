use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::fd::OwnedFd;
use rustix::fs::{Mode, OFlags, fcntl_getfl, fcntl_setfl, open as open_file};
use rustix::io::{Errno, fcntl_dupfd_cloexec};
#[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "macos")))]
use rustix::io::{FdFlags, fcntl_setfd};
use rustix::process::{
    Pid, Signal, WaitId, WaitIdOptions, WaitOptions, WaitStatus, ioctl_tiocsctty,
    kill_process_group, setsid, test_kill_process_group, waitid, waitpid,
};
use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};
use rustix::termios::{Winsize, tcsetwinsize};

use crate::api::{
    CLOSE_GRACE_PERIOD, CommandSpec, ExitResult, Failure, OUTPUT_CHUNK_BYTES,
    OUTPUT_QUEUE_CAPACITY, PtyEvent, PtySession, PtySize,
};

const REAPER_POLL_INTERVAL: Duration = Duration::from_millis(10);
const SEND_BACKPRESSURE_INTERVAL: Duration = Duration::from_millis(10);
const OUTPUT_POLL_TIMEOUT: Timespec = Timespec {
    tv_sec: 0,
    tv_nsec: 10_000_000,
};

pub(crate) struct Inner {
    shared: Arc<Shared>,
}

struct Shared {
    process: Mutex<Process>,
    control_master: Mutex<Option<File>>,
    changed: Condvar,
}

struct Process {
    process_group: Pid,
    exit: Option<ExitResult>,
    wait_failure: Option<Failure>,
    closing: bool,
    closed: bool,
}

fn open_master() -> Result<OwnedFd, Failure> {
    // Linux (via `/dev/ptmx` in rustix) and FreeBSD expose `OpenptFlags::CLOEXEC`.
    // macOS `posix_openpt` also accepts `O_CLOEXEC` atomically, even though
    // rustix does not expose the named `OpenptFlags::CLOEXEC` constant there.
    // On `ios` and `illumos`, `posix_openpt` `O_CLOEXEC` support is unverified
    // or unsupported, so those targets set `FD_CLOEXEC` immediately after
    // `openpt` returns.
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY | OpenptFlags::CLOEXEC).map_err(|errno| {
            Failure::OpenPty {
                raw_os_error: errno.raw_os_error(),
            }
        })
    }
    #[cfg(target_os = "macos")]
    {
        let flags = OpenptFlags::RDWR
            | OpenptFlags::NOCTTY
            | OpenptFlags::from_bits_retain(OFlags::CLOEXEC.bits());
        openpt(flags).map_err(|errno| Failure::OpenPty {
            raw_os_error: errno.raw_os_error(),
        })
    }
    #[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "macos")))]
    {
        let master =
            openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY).map_err(|errno| Failure::OpenPty {
                raw_os_error: errno.raw_os_error(),
            })?;
        fcntl_setfd(&master, FdFlags::CLOEXEC).map_err(|errno| Failure::DescriptorFlags {
            raw_os_error: errno.raw_os_error(),
        })?;
        Ok(master)
    }
}

pub(crate) fn open(spec: CommandSpec) -> Result<(PtySession, Receiver<PtyEvent>), Failure> {
    let master = open_master()?;
    grantpt(&master).map_err(|errno| Failure::GrantPty {
        raw_os_error: errno.raw_os_error(),
    })?;
    unlockpt(&master).map_err(|errno| Failure::UnlockPty {
        raw_os_error: errno.raw_os_error(),
    })?;
    let slave_name = ptsname(&master, Vec::new()).map_err(|errno| Failure::PtyName {
        raw_os_error: errno.raw_os_error(),
    })?;
    let slave_path = Path::new(OsStr::from_bytes(slave_name.to_bytes()));
    let slave = open_file(
        slave_path,
        OFlags::RDWR | OFlags::NOCTTY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|errno| Failure::OpenSlave {
        raw_os_error: errno.raw_os_error(),
    })?;
    tcsetwinsize(
        &master,
        Winsize {
            ws_row: spec.size.rows.max(1),
            ws_col: spec.size.columns.max(1),
            ws_xpixel: 0,
            ws_ypixel: 0,
        },
    )
    .map_err(|errno| Failure::Resize {
        raw_os_error: errno.raw_os_error(),
    })?;

    // Set `O_NONBLOCK` on the master file description before duplicating it.
    // `control_master` and `reader_master` intentionally share this open file
    // description: the reader polls and drains without blocking, and `write`
    // handles `WouldBlock` by releasing the lifecycle lock so a full PTY input
    // buffer cannot deadlock `close` or `resize`.
    let master_flags = fcntl_getfl(&master).map_err(|errno| Failure::DescriptorFlags {
        raw_os_error: errno.raw_os_error(),
    })?;
    fcntl_setfl(&master, master_flags | OFlags::NONBLOCK).map_err(|errno| {
        Failure::DescriptorFlags {
            raw_os_error: errno.raw_os_error(),
        }
    })?;
    let control_master = duplicate(&master)?;
    let reader_master = duplicate(&master)?;
    let stdin_fd = duplicate(&slave)?;
    let stdout_fd = duplicate(&slave)?;
    let stderr_fd = duplicate(&slave)?;
    let ctty_fd = duplicate(&slave)?;
    // The parent keeps one slave descriptor until the output reader finishes
    // its final post-exit drain. Some platforms discard unread
    // slave-to-master data on last slave close, so dropping every parent
    // copy at spawn would let a child that writes and exits before the
    // reader's next drain lose its output.
    let retained_slave = duplicate(&slave)?;
    let ctty_file = File::from(ctty_fd);
    drop(master);
    drop(slave);

    let mut command = Command::new(&spec.program);
    command
        .args(&spec.arguments)
        .env_clear()
        .envs(&spec.environment)
        .current_dir(&spec.working_directory)
        .stdin(Stdio::from(File::from(stdin_fd)))
        .stdout(Stdio::from(File::from(stdout_fd)))
        .stderr(Stdio::from(File::from(stderr_fd)));

    install_session_hook(&mut command, ctty_file);

    let child = command.spawn().map_err(|error| Failure::Spawn {
        raw_os_error: error.raw_os_error(),
    })?;
    drop(command);
    let process_group = Pid::from_child(&child);
    drop(child);
    let shared = Arc::new(Shared {
        process: Mutex::new(Process {
            process_group,
            exit: None,
            wait_failure: None,
            closing: false,
            closed: false,
        }),
        control_master: Mutex::new(Some(File::from(control_master))),
        changed: Condvar::new(),
    });
    let session = PtySession {
        inner: Inner {
            shared: shared.clone(),
        },
    };
    let (sender, receiver) = sync_channel(OUTPUT_QUEUE_CAPACITY);

    let reaper_shared = shared.clone();
    if let Err(error) = thread::Builder::new()
        .name("longhorn-pty-reaper".to_owned())
        .spawn(move || reap_child(reaper_shared))
    {
        abort_child(&shared);
        return Err(Failure::WorkerSpawn {
            raw_os_error: error.raw_os_error(),
        });
    }

    if let Err(error) = thread::Builder::new()
        .name("longhorn-pty-output".to_owned())
        .spawn(move || {
            read_output(
                File::from(reader_master),
                File::from(retained_slave),
                sender,
                shared,
            )
        })
    {
        let _ = session.close();
        return Err(Failure::WorkerSpawn {
            raw_os_error: error.raw_os_error(),
        });
    }

    Ok((session, receiver))
}

// This narrow allowance overrides the workspace's `forbid` so the crate can
// use CommandExt's required unsafe pre-exec registration point.
#[allow(unsafe_code)]
fn install_session_hook(command: &mut Command, ctty_file: File) {
    // SAFETY: the closure runs after fork and before exec. It calls only
    // `setsid` and `ioctl(TIOCSCTTY)` through rustix; the captured descriptor
    // is an owned PTY slave descriptor kept open for this handover.
    unsafe {
        command.pre_exec(move || {
            setsid().map_err(std::io::Error::from)?;
            ioctl_tiocsctty(&ctty_file).map_err(std::io::Error::from)
        });
    }
}

fn duplicate(fd: &OwnedFd) -> Result<OwnedFd, Failure> {
    fcntl_dupfd_cloexec(fd, 0).map_err(|errno| Failure::DuplicateDescriptor {
        raw_os_error: errno.raw_os_error(),
    })
}

fn process_lock(shared: &Shared) -> MutexGuard<'_, Process> {
    shared
        .process
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn master_lock(shared: &Shared) -> MutexGuard<'_, Option<File>> {
    shared
        .control_master
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn record_exit(process: &mut Process, status: WaitStatus) -> ExitResult {
    let result = if let Some(code) = status.exit_status() {
        ExitResult::Code(code)
    } else if let Some(signal) = status.terminating_signal() {
        ExitResult::Signal(signal)
    } else {
        // waitpid without WUNTRACED or WCONTINUED returns only terminal states.
        unreachable!("waitpid returned a terminated child status")
    };
    process.exit = Some(result);
    result
}

fn poll_exit(process: &mut Process, changed: &Condvar) -> Result<bool, Failure> {
    if process.exit.is_some() || process.wait_failure.is_some() || process.closed || process.closing
    {
        return Ok(false);
    }
    poll_child(process, changed)
}

fn poll_child(process: &mut Process, changed: &Condvar) -> Result<bool, Failure> {
    if process.exit.is_some() || process.wait_failure.is_some() {
        return Ok(false);
    }
    match waitpid(Some(process.process_group), WaitOptions::NOHANG) {
        Ok(Some((_, status))) => {
            record_exit(process, status);
            changed.notify_all();
            Ok(false)
        }
        Ok(None) => Ok(true),
        Err(Errno::INTR) => Ok(true),
        Err(errno) => {
            let failure = Failure::Wait {
                raw_os_error: errno.raw_os_error(),
            };
            process.wait_failure = Some(failure);
            changed.notify_all();
            Err(failure)
        }
    }
}

fn reap_child(shared: Arc<Shared>) {
    loop {
        {
            let mut process = process_lock(&shared);
            if process.exit.is_some() || process.wait_failure.is_some() {
                return;
            }
            if !process.closing {
                let _poll = poll_child(&mut process, &shared.changed);
                if process.exit.is_some() || process.wait_failure.is_some() {
                    return;
                }
            }
        }
        thread::sleep(REAPER_POLL_INTERVAL);
    }
}

enum DrainResult {
    Drained,
    Finished,
}

enum SendOutcome {
    Sent,
    Stop,
}

/// Sends one output chunk with back-pressure, waking immediately if the session
/// begins closing while the queue is full so the reader releases the PTY
/// descriptors.
fn send_output(sender: &SyncSender<PtyEvent>, shared: &Shared, mut event: PtyEvent) -> SendOutcome {
    loop {
        match sender.try_send(event) {
            Ok(()) => return SendOutcome::Sent,
            Err(TrySendError::Disconnected(_)) => return SendOutcome::Stop,
            Err(TrySendError::Full(unsent)) => {
                let process = process_lock(shared);
                if process.closing || process.closed {
                    return SendOutcome::Stop;
                }
                let (process, _) = shared
                    .changed
                    .wait_timeout(process, SEND_BACKPRESSURE_INTERVAL)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if process.closing || process.closed {
                    return SendOutcome::Stop;
                }
                event = unsent;
            }
        }
    }
}

fn drain_output(
    master: &mut File,
    sender: &SyncSender<PtyEvent>,
    shared: &Shared,
) -> Result<DrainResult, Failure> {
    loop {
        // A blocked send owns this buffer; no second full-size output buffer
        // remains in the reader while queue capacity is exhausted.
        let mut buffer = vec![0u8; OUTPUT_CHUNK_BYTES];
        match master.read(&mut buffer) {
            // A zero-byte read is not end-of-file on a pseudo-terminal: it
            // carries no information about the child, so poll again exactly
            // like a momentarily empty master.
            Ok(0) => return Ok(DrainResult::Drained),
            Ok(length) => {
                buffer.truncate(length);
                if matches!(
                    send_output(sender, shared, PtyEvent::Output(buffer)),
                    SendOutcome::Stop
                ) {
                    return Ok(DrainResult::Finished);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                return Ok(DrainResult::Drained);
            }
            Err(error) if error.raw_os_error() == Some(Errno::IO.raw_os_error()) => {
                return Ok(DrainResult::Finished);
            }
            Err(error) => {
                return Err(Failure::ReadOutput {
                    raw_os_error: error.raw_os_error(),
                });
            }
        }
    }
}

fn read_output(
    mut master: File,
    held_slave: File,
    sender: SyncSender<PtyEvent>,
    shared: Arc<Shared>,
) {
    let mut read_failure = None;
    loop {
        match drain_output(&mut master, &sender, &shared) {
            Ok(DrainResult::Drained) => {}
            Ok(DrainResult::Finished) => break,
            Err(failure) => {
                read_failure = Some(failure);
                break;
            }
        }

        let process_ended = {
            let process = process_lock(&shared);
            process.exit.is_some() || process.wait_failure.is_some()
        };
        if process_ended {
            // The leader may have exited after the prior read reached WouldBlock.
            // Drain once more before reporting its result, without waiting for
            // descendants that still hold the slave descriptor.
            if let Err(failure) = drain_output(&mut master, &sender, &shared) {
                read_failure = Some(failure);
            }
            break;
        }

        let mut descriptors = [PollFd::new(&master, PollFlags::IN)];
        match poll(&mut descriptors, Some(&OUTPUT_POLL_TIMEOUT)) {
            Ok(_) | Err(Errno::INTR) => {}
            Err(errno) => {
                read_failure = Some(Failure::ReadOutput {
                    raw_os_error: Some(errno.raw_os_error()),
                });
                break;
            }
        }
    }
    drop(master);
    // The retained slave kept the child's output readable past its death.
    // It closes only after the final drain above (or on close wake when the
    // consumer stopped reading), releasing the PTY pair back to the OS.
    drop(held_slave);

    let mut process = process_lock(&shared);
    while process.exit.is_none() && process.wait_failure.is_none() {
        process = shared
            .changed
            .wait(process)
            .unwrap_or_else(std::sync::PoisonError::into_inner);
    }
    let exit = process.exit;
    let wait_failure = process.wait_failure;
    drop(process);

    if let Some(failure) = read_failure {
        let _ = sender.send(PtyEvent::Failure(failure));
    }
    if let Some(failure) = wait_failure {
        let _ = sender.send(PtyEvent::Failure(failure));
    }
    if let Some(exit) = exit {
        let _ = sender.send(PtyEvent::Exit(exit));
    }
}

pub(crate) fn write(session: &PtySession, bytes: &[u8]) -> Result<(), Failure> {
    let shared = &session.inner.shared;
    let mut written = 0;
    loop {
        let mut process = process_lock(shared);
        if !poll_exit(&mut process, &shared.changed)? {
            return Err(Failure::Ended);
        }
        if written == bytes.len() {
            return Ok(());
        }
        let result = {
            let mut control_master = master_lock(shared);
            let Some(master) = control_master.as_mut() else {
                return Err(Failure::Ended);
            };
            master.write(&bytes[written..])
        };
        match result {
            Ok(0) => return Err(Failure::WriteInput { raw_os_error: None }),
            Ok(length) => written += length,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                drop(process);
                thread::sleep(REAPER_POLL_INTERVAL);
            }
            Err(error) => {
                if !poll_exit(&mut process, &shared.changed)? {
                    return Err(Failure::Ended);
                }
                return Err(Failure::WriteInput {
                    raw_os_error: error.raw_os_error(),
                });
            }
        }
    }
}

pub(crate) fn resize(session: &PtySession, size: PtySize) -> Result<(), Failure> {
    let shared = &session.inner.shared;
    let mut process = process_lock(shared);
    if !poll_exit(&mut process, &shared.changed)? {
        return Err(Failure::Ended);
    }
    let master = master_lock(shared);
    let Some(master) = master.as_ref() else {
        return Err(Failure::Ended);
    };
    let winsize = Winsize {
        ws_row: size.rows.max(1),
        ws_col: size.columns.max(1),
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    tcsetwinsize(master, winsize).map_err(|errno| Failure::Resize {
        raw_os_error: errno.raw_os_error(),
    })?;
    match kill_process_group(process.process_group, Signal::WINCH) {
        Ok(()) => Ok(()),
        Err(error) => match group_failure(Err(error), process.process_group) {
            None => Err(Failure::Ended),
            Some(failure) => Err(failure),
        },
    }
}

pub(crate) fn close(session: &PtySession) -> Result<(), Failure> {
    close_shared(&session.inner.shared)
}

/// Classifies a failed process-group signal. `ESRCH` means the group is gone.
/// macOS and BSDs answer `EPERM` instead when the group holds only zombies, so
/// `EPERM` counts as gone only when the leader has exited. The leader is read
/// with `waitid` and `WNOWAIT`: the exit is seen but the zombie is not
/// collected, so the group identity stays reserved until close finishes. A
/// live leader that refuses the signal stays a `Signal` failure.
fn group_failure(result: Result<(), Errno>, process_group: Pid) -> Option<Failure> {
    match result {
        Ok(()) | Err(Errno::SRCH) => None,
        Err(Errno::PERM) => match leader_exited_uncollected(process_group) {
            Ok(true) => None,
            Ok(false) => Some(Failure::Signal {
                raw_os_error: Errno::PERM.raw_os_error(),
            }),
            Err(errno) => Some(Failure::Wait {
                raw_os_error: errno.raw_os_error(),
            }),
        },
        Err(errno) => Some(Failure::Signal {
            raw_os_error: errno.raw_os_error(),
        }),
    }
}

fn leader_exited_uncollected(process_group: Pid) -> Result<bool, Errno> {
    let options = WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT;
    loop {
        match waitid(WaitId::Pid(process_group), options) {
            Ok(status) => return Ok(status.is_some()),
            Err(Errno::INTR) => {}
            Err(error) => return Err(error),
        }
    }
}

fn close_shared(shared: &Shared) -> Result<(), Failure> {
    let process_group = {
        let mut process = process_lock(shared);
        let live = match poll_exit(&mut process, &shared.changed) {
            Ok(live) => live,
            Err(failure) => {
                process.closed = true;
                master_lock(shared).take();
                shared.changed.notify_all();
                return Err(failure);
            }
        };
        if !live {
            // Even when the child already exited (or the session is already
            // ended), drop the control master and wake any reader blocked on a
            // full queue so the PTY pair is released without sending a signal.
            if !process.closing {
                process.closed = true;
            }
            master_lock(shared).take();
            shared.changed.notify_all();
            return Err(Failure::Ended);
        }
        process.closing = true;
        // Release the control master and wake any output reader blocked on a
        // full queue before waiting out the grace period. Nothing uses
        // `control_master` once `closing` is set, and closing both master
        // copies when the consumer stopped reading lets a child blocked on a
        // full TTY buffer finish exiting on SIGHUP.
        master_lock(shared).take();
        shared.changed.notify_all();
        process.process_group
    };

    let mut failure = group_failure(
        kill_process_group(process_group, Signal::HUP),
        process_group,
    );
    thread::sleep(CLOSE_GRACE_PERIOD);

    {
        let mut process = process_lock(shared);
        // A probe that finds the group gone sends no KILL. The leader stays a
        // zombie until the reaper runs after close, so the probe and the
        // escalation never signal a reaped, reusable group identity.
        let escalation = match test_kill_process_group(process_group) {
            Ok(()) => kill_process_group(process_group, Signal::KILL),
            Err(error) => Err(error),
        };
        if let Some(error) = group_failure(escalation, process_group) {
            failure = Some(error);
        }
        process.closed = true;
        process.closing = false;
        shared.changed.notify_all();
    }

    match failure {
        Some(failure) => Err(failure),
        None => Ok(()),
    }
}

fn abort_child(shared: &Shared) {
    let mut process = process_lock(shared);
    let _ = kill_process_group(process.process_group, Signal::KILL);
    loop {
        match waitpid(Some(process.process_group), WaitOptions::empty()) {
            Ok(Some(_)) | Err(Errno::SRCH) => break,
            Err(Errno::INTR) => continue,
            Ok(None) | Err(_) => break,
        }
    }
    process.closed = true;
    shared.changed.notify_all();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_live_leader_that_refuses_a_group_signal_is_a_signal_failure() {
        let mut child = Command::new("/bin/sleep")
            .arg("30")
            .spawn()
            .expect("the sleeper starts");
        let process_group = Pid::from_child(&child);

        assert_eq!(
            group_failure(Err(Errno::PERM), process_group),
            Some(Failure::Signal {
                raw_os_error: Errno::PERM.raw_os_error(),
            })
        );

        child.kill().expect("the sleeper is stopped");
        child.wait().expect("the sleeper is reaped");
    }

    #[test]
    fn a_zombie_leader_is_gone_and_is_not_collected() {
        let mut child = Command::new("/bin/sh")
            .args(["-c", "exit 0"])
            .spawn()
            .expect("the shell starts");
        let process_group = Pid::from_child(&child);
        // Blocks until the leader exits; WNOWAIT leaves it a zombie.
        let exited = waitid(
            WaitId::Pid(process_group),
            WaitIdOptions::EXITED | WaitIdOptions::NOWAIT,
        )
        .expect("the exit is observed");
        assert!(exited.is_some(), "the leader exited");

        assert_eq!(group_failure(Err(Errno::PERM), process_group), None);
        assert_eq!(
            leader_exited_uncollected(process_group),
            Ok(true),
            "classification left the zombie for close's reaper"
        );

        child.wait().expect("the zombie is reaped by its parent");
    }

    #[test]
    fn send_output_wakes_immediately_when_session_closes_on_full_queue() {
        let (sender, _receiver) = sync_channel(1);
        sender
            .send(PtyEvent::Output(vec![1]))
            .expect("the single queue slot fills");

        let shared = Arc::new(Shared {
            process: Mutex::new(Process {
                process_group: rustix::process::getpid(),
                exit: None,
                wait_failure: None,
                closing: false,
                closed: false,
            }),
            control_master: Mutex::new(None),
            changed: Condvar::new(),
        });

        let closer_shared = shared.clone();
        let closer = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            let mut process = process_lock(&closer_shared);
            process.closed = true;
            closer_shared.changed.notify_all();
        });

        let outcome = send_output(&sender, &shared, PtyEvent::Output(vec![2]));
        closer.join().expect("the closer thread finishes");
        assert!(
            matches!(outcome, SendOutcome::Stop),
            "closing the session stops a send blocked on a full queue"
        );
    }

    #[test]
    fn open_master_sets_cloexec_flag() {
        let master = open_master().expect("a PTY master opens");
        let flags = rustix::io::fcntl_getfd(&master).expect("master descriptor flags are readable");
        assert!(
            flags.contains(rustix::io::FdFlags::CLOEXEC),
            "master descriptor has FD_CLOEXEC set: {flags:?}"
        );
    }
}
