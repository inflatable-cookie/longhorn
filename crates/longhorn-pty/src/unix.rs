use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use rustix::fd::OwnedFd;
use rustix::fs::{Mode, OFlags, fcntl_getfl, fcntl_setfl, open as open_file};
use rustix::io::{Errno, FdFlags, dup, fcntl_setfd};
use rustix::process::{
    Pid, Signal, WaitOptions, WaitStatus, ioctl_tiocsctty, kill_process_group, setsid,
    test_kill_process_group, waitpid,
};
use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};
use rustix::termios::{Winsize, tcsetwinsize};

use crate::api::{
    CLOSE_GRACE_PERIOD, CommandSpec, ExitResult, Failure, OUTPUT_CHUNK_BYTES,
    OUTPUT_QUEUE_CAPACITY, PtyEvent, PtySession, PtySize,
};

const REAPER_POLL_INTERVAL: Duration = Duration::from_millis(10);

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
    wait_failed: bool,
    closing: bool,
    closed: bool,
}

pub(crate) fn open(spec: CommandSpec) -> Result<(PtySession, Receiver<PtyEvent>), Failure> {
    let master = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY).map_err(|_| Failure::OpenPty)?;
    set_cloexec(&master)?;
    grantpt(&master).map_err(|_| Failure::GrantPty)?;
    unlockpt(&master).map_err(|_| Failure::UnlockPty)?;
    let slave_name = ptsname(&master, Vec::new()).map_err(|_| Failure::PtyName)?;
    let slave_path = Path::new(OsStr::from_bytes(slave_name.to_bytes()));
    let slave = open_file(slave_path, OFlags::RDWR | OFlags::NOCTTY, Mode::empty())
        .map_err(|_| Failure::OpenSlave)?;
    set_cloexec(&slave)?;
    tcsetwinsize(
        &master,
        Winsize {
            ws_row: spec.size.rows.max(1),
            ws_col: spec.size.columns.max(1),
            ws_xpixel: 0,
            ws_ypixel: 0,
        },
    )
    .map_err(|_| Failure::Resize)?;

    let control_master = duplicate(&master)?;
    let reader_master = duplicate(&master)?;
    let reader_flags = fcntl_getfl(&reader_master).map_err(|_| Failure::DescriptorFlags)?;
    fcntl_setfl(&reader_master, reader_flags | OFlags::NONBLOCK)
        .map_err(|_| Failure::DescriptorFlags)?;
    let stdin_fd = duplicate(&slave)?;
    let stdout_fd = duplicate(&slave)?;
    let stderr_fd = duplicate(&slave)?;
    let ctty_fd = duplicate(&slave)?;
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

    let child = command.spawn().map_err(|_| Failure::Spawn)?;
    drop(command);
    let process_group = Pid::from_child(&child);
    drop(child);
    let shared = Arc::new(Shared {
        process: Mutex::new(Process {
            process_group,
            exit: None,
            wait_failed: false,
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
    if thread::Builder::new()
        .name("longhorn-pty-reaper".to_owned())
        .spawn(move || reap_child(reaper_shared))
        .is_err()
    {
        abort_child(&shared);
        return Err(Failure::WorkerSpawn);
    }

    if thread::Builder::new()
        .name("longhorn-pty-output".to_owned())
        .spawn(move || read_output(File::from(reader_master), sender, shared))
        .is_err()
    {
        let _ = session.close();
        return Err(Failure::WorkerSpawn);
    }

    Ok((session, receiver))
}

fn set_cloexec(fd: &OwnedFd) -> Result<(), Failure> {
    fcntl_setfd(fd, FdFlags::CLOEXEC).map_err(|_| Failure::DescriptorFlags)
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
    let duplicate = dup(fd).map_err(|_| Failure::DuplicateDescriptor)?;
    set_cloexec(&duplicate)?;
    Ok(duplicate)
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
    if process.exit.is_some() || process.wait_failed || process.closed || process.closing {
        return Ok(false);
    }
    poll_child(process, changed)
}

fn poll_child(process: &mut Process, changed: &Condvar) -> Result<bool, Failure> {
    if process.exit.is_some() || process.wait_failed {
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
        Err(_) => {
            process.wait_failed = true;
            changed.notify_all();
            Err(Failure::Wait)
        }
    }
}

fn reap_child(shared: Arc<Shared>) {
    loop {
        {
            let mut process = process_lock(&shared);
            if process.exit.is_some() || process.wait_failed {
                return;
            }
            if !process.closing {
                let _poll = poll_child(&mut process, &shared.changed);
                if process.exit.is_some() {
                    return;
                }
            }
        }
        thread::sleep(REAPER_POLL_INTERVAL);
    }
}

fn read_output(mut master: File, sender: SyncSender<PtyEvent>, shared: Arc<Shared>) {
    let mut read_failed = false;
    loop {
        // A blocked send owns this buffer; no second full-size output buffer
        // remains in the reader while queue capacity is exhausted.
        let mut buffer = vec![0u8; OUTPUT_CHUNK_BYTES];
        match master.read(&mut buffer) {
            Ok(0) => break,
            Ok(length) => {
                buffer.truncate(length);
                if sender.send(PtyEvent::Output(buffer)).is_err() {
                    break;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                let process = process_lock(&shared);
                if process.exit.is_some() || process.wait_failed {
                    break;
                }
                let (process, _) = shared
                    .changed
                    .wait_timeout(process, REAPER_POLL_INTERVAL)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if process.exit.is_some() || process.wait_failed {
                    break;
                }
            }
            Err(error) if error.raw_os_error() == Some(Errno::IO.raw_os_error()) => {
                break;
            }
            Err(_) => {
                read_failed = true;
                break;
            }
        }
    }
    drop(master);

    let mut process = process_lock(&shared);
    while process.exit.is_none() && !process.wait_failed {
        process = shared
            .changed
            .wait(process)
            .unwrap_or_else(std::sync::PoisonError::into_inner);
    }
    let exit = process.exit;
    let wait_failed = process.wait_failed;
    drop(process);

    if read_failed {
        let _ = sender.send(PtyEvent::Failure(Failure::ReadOutput));
    }
    if wait_failed {
        let _ = sender.send(PtyEvent::Failure(Failure::Wait));
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
            Ok(0) => return Err(Failure::WriteInput),
            Ok(length) => written += length,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                drop(process);
                thread::sleep(REAPER_POLL_INTERVAL);
            }
            Err(_) => {
                if !poll_exit(&mut process, &shared.changed)? {
                    return Err(Failure::Ended);
                }
                return Err(Failure::WriteInput);
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
    tcsetwinsize(master, winsize).map_err(|_| Failure::Resize)?;
    match kill_process_group(process.process_group, Signal::WINCH) {
        Ok(()) => Ok(()),
        Err(Errno::SRCH) => {
            if !poll_exit(&mut process, &shared.changed)? {
                Err(Failure::Ended)
            } else {
                Err(Failure::Signal)
            }
        }
        Err(_) => Err(Failure::Signal),
    }
}

pub(crate) fn close(session: &PtySession) -> Result<(), Failure> {
    close_shared(&session.inner.shared)
}

fn close_shared(shared: &Shared) -> Result<(), Failure> {
    let process_group = {
        let mut process = process_lock(shared);
        if !poll_exit(&mut process, &shared.changed)? {
            return Err(Failure::Ended);
        }
        process.closing = true;
        process.process_group
    };

    let mut failure = match kill_process_group(process_group, Signal::HUP) {
        Ok(()) | Err(Errno::SRCH) => None,
        Err(_) => Some(Failure::Signal),
    };
    thread::sleep(CLOSE_GRACE_PERIOD);

    {
        let mut process = process_lock(shared);
        match test_kill_process_group(process_group) {
            Ok(()) => {
                if let Err(error) = kill_process_group(process_group, Signal::KILL)
                    && error != Errno::SRCH
                {
                    failure = Some(Failure::Signal);
                }
            }
            Err(error) if error == Errno::SRCH => {}
            Err(_) => failure = Some(Failure::Signal),
        }
        process.closed = true;
        process.closing = false;
        master_lock(shared).take();
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
}
