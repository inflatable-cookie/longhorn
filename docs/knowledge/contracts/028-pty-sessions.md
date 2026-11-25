# 028 — PTY Sessions

Status: active compiled boundary
Owner: Longhorn
Tier: core
Updated: 2026-11-25

## Boundary

`longhorn-pty` runs one caller-selected child program on a real pseudo-terminal.
It owns the PTY, child session, process-group signaling, bounded output stream,
terminal writes and size changes, close escalation, and exactly-once child
reaping. It has no Tauri, renderer, managed-tool, credential, or product policy
dependency.

The caller supplies a program path, argument vector, complete environment,
working directory, and initial rows and columns. Longhorn clears the inherited
environment and supplies exactly that map. It does not add `TERM`, `COLORTERM`,
`PATH`, or any other variable. Use [`DEFAULT_SIZE`](../../../crates/longhorn-pty/src/api.rs)
for the donor's 24×80 starting size. A requested zero row or column is clamped
to 1.

## Public API

- `CommandSpec` carries the program, arguments, environment, working directory,
  and initial `PtySize`.
- `PtySession::open` returns the session and a `Receiver<PtyEvent>`.
- `PtyEvent::Output` carries bytes; `PtyEvent::Failure` carries a classified
  asynchronous read or wait failure; `PtyEvent::Exit` carries one
  `ExitResult::Code` or `ExitResult::Signal`.
- `PtySession::write` sends all supplied bytes to the PTY input.
- `PtySession::resize` updates the terminal size and sends `SIGWINCH` to the
  child process group.
- `PtySession::close` hangs up the process group and escalates when needed.
- `ExitResult::compatibility_code` maps a code unchanged and a signal to
  `128 + signal`, matching the donor's shell-style result.

The crate returns raw bytes. Renderer encoding, event names, and session ids
belong to the host adapter.

## Lifecycle and process-group rules

The child calls `setsid` and makes the PTY slave its controlling terminal
before `exec`. Its process id is therefore also its session and process-group
id. The child receives the PTY slave for stdin, stdout, and stderr. The parent
retains one slave descriptor while the output reader is draining and closes
it when the reader finishes: after its final post-exit drain when the
consumer reads to the exit, or at its early exit when the consumer dropped
the receiver. Some platforms discard unread slave-to-master data on last
slave close, so without the retained copy a child that writes and exits
before the reader's next drain would lose its output. In the dropped path
the child's own slave copies still hold the slave open, so last close still
cannot precede the child's own end.

`write`, `resize`, close, and child status polling share one lifecycle lock.
Once the PTY accepts the last byte of a write, that write succeeds even if the
child exits immediately.
The child is reaped once by rustix `waitpid`; the cached result closes the
session to further operations. A later write, resize, or close returns
`Failure::Ended` and sends no signal. A signal already serialized before a
child exit can only target the still-unreaped process group; no signal is sent
after reaping, so the child pid or group cannot be reused as a signal target.

Close marks the session as closing, drops its control master descriptor, and
notifies any output reader blocked on a full queue before sending `SIGHUP` to
the process group. When the child has already exited or `waitpid` has already
failed, `close` still drops the control master descriptor and notifies any
back-pressured reader so the PTY pair is released, while returning
`Failure::Ended` (or `Failure::Wait`) and sending no signal. Reaping is held
until close finishes, preserving the group identity through escalation. After
the fixed 300 ms grace period, Longhorn probes the group and sends `SIGKILL` if
it remains, marks the session closed, and allows the reaper to record the
direct child's final status. When the receiver queue is not full, the
nonblocking output reader drains currently readable bytes after the leader is
reaped, then sends the one exit result. A descendant that keeps the slave open
cannot prevent that result; output written after the direct child exits is not
delivered.

A group that is already gone counts as closed, and so does a group that holds
only zombies. `ESRCH` always means gone. macOS and BSDs answer `EPERM` for a
group whose members are all zombies, so Longhorn treats `EPERM` as gone only
when `waitid` with `WNOWAIT` shows the leader has exited. That check reads the
exit without collecting the zombie, so the group identity stays reserved until
the reaper runs after close. A leader still running that refuses the signal is
`Failure::Signal`. Close and resize use the same classification. When the
escalation probe finds the group gone, close sends no `SIGKILL`. Limit: a
descendant that outlives its exited leader and refuses signals (for example, a
setuid program) cannot be told from a zombie without process listing, so it is
reported as closed.

If `waitpid` fails before a status is available, the reader sends the
classified `Failure::Wait` and no exit result is available. The session is
ended for further operations and no later signal is sent.

## Output bound and back-pressure

Each output event contains at most 8 KiB. The receiver queue holds at most
eight output events, and the reader holds at most one additional 8 KiB chunk
while blocked on a full queue. Thus the crate retains no more than 72 KiB of
output payload per session, plus fixed channel and buffer metadata. When the
queue is full, the reader waits; the PTY kernel buffer then fills and applies
back-pressure to the child. While the session remains open, chunks are not
dropped, coalesced, or accumulated in an unbounded buffer; a consumer that
stops reading pauses the child until it resumes or closes the session. Calling
`close()` (including on an already-exited session or via `Drop`) wakes a reader
that is waiting on a full queue. In that close-under-backpressure path, the
reader discards the single unsent chunk it was holding and stops draining
further PTY output so it immediately releases both its master copy and the
retained slave descriptor back to the OS, even if the consumer keeps the
receiver alive without reading. Events already queued in the receiver (up to
eight chunks) remain readable, followed by the terminal `PtyEvent::Exit` (or
`PtyEvent::Failure`) once the consumer reads from the receiver.

## Failures and platform behavior

Failures are typed `Failure` enum values, not localized or path-bearing
diagnostic strings. Classes cover PTY master/slave setup (`OpenPty`,
`GrantPty`, `UnlockPty`, `PtyName`, `OpenSlave`), descriptor duplication and
flags, spawn and worker creation, input, resize, signal, output read, child
wait, ended sessions, and unsupported targets. Every `openpt`, `grantpt`,
`ptsname`, and descriptor-duplication failure is checked and classified.

Every OS-backed `Failure` variant carries the numeric `raw_os_error` reported
by the OS (`Failure::raw_os_error()`), and `Display` appends `(os error <code>)`
when present. Because only the numeric errno is carried — never paths,
environment values, or command arguments — errors remain typed and
redaction-safe. The sessions suite probes the OS once after a failed session
open and includes the probe outcome in its failure report.

On macOS, concurrent master opens can race for the same `/dev/ptmx` slot.
XNU may return its internal `EREDRIVEOPEN` (`-6`) after its own retries run
out. Longhorn retries the master open only for `EREDRIVEOPEN`, up to three
times with a 10 ms delay before each retry. If allocation still fails,
`Failure::OpenPty` carries the last raw error unchanged. Positive `ENXIO`
(`6`), which can indicate a full host PTY pool, and `EMFILE`, which
indicates a process descriptor limit, return immediately.
[XNU PTY allocator](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/kern/tty_ptmx.c#L310-L364),
[PTY error mapping](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/kern/tty_dev.c#L488-L497),
[open retry loop](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/vfs/vfs_vnops.c#L763-L801).

On supported Unix targets (`linux`, `macos`, `ios`, `freebsd`, `illumos`), the
crate uses `rustix` for PTY allocation, descriptor duplication and
close-on-exec flags, `setsid`, controlling-terminal ioctl, terminal sizing, and
process-group signals. Close-on-exec is set atomically on slave opens
(`OFlags::CLOEXEC`) and descriptor duplications (`fcntl_dupfd_cloexec`) on all
supported targets, and atomically on master opens on Linux and FreeBSD
(`OpenptFlags::CLOEXEC`) and on macOS and illumos (`posix_openpt` with
`O_CLOEXEC`). Only `ios` (where `posix_openpt` `O_CLOEXEC` support is
unverified) falls back to setting `FD_CLOEXEC` immediately after `openpt`
returns.
`O_NONBLOCK` is set on the master open file description before duplication:
both the reader copy (poll-driven non-blocking reads) and the control copy
(non-blocking writes that release the lifecycle lock on `WouldBlock` so a full
input buffer cannot deadlock `close` or `resize`) share that file description
and rely on it. The only unsafe boundary is the `pre_exec` closure, which calls
only `setsid` and the controlling-terminal ioctl. On non-Unix targets, the
crate compiles and `open` returns `Failure::Unsupported`.

## Generic and product-owned behavior

Longhorn owns the host mechanism: one process per PTY, direct arguments,
complete environment replacement, byte streaming, resize, group signals,
close escalation, and exit status. The application owns program allowlists,
which environment values to provide, tool credentials, terminal environment
values, renderer base64 encoding, event transport, session identifiers, and
user-facing process policy. No product environment or allowed-program policy
is introduced here.

## Compatibility for consumers

Compared with Acowtancy Desktop's `pty.rs`:

- `TerminalRuntime` and its string session ids become one explicit
  `PtySession`; id allocation and Tauri event transport stay in Acowtancy.
- Output is raw `Vec<u8>` through a bounded receiver instead of base64 Tauri
  events. A slow consumer now back-pressures the PTY child instead of growing
  host memory.
- The caller provides the complete environment. The crate no longer injects
  managed-tool values, `TERM`, or `COLORTERM`.
- PTY descriptors and `dup` results are checked, and failures are typed.
- Resize and close signal the process group. Close sends `SIGHUP`, waits 300 ms,
  and escalates to `SIGKILL` when the group remains.
- Child status is reaped once and reported as a typed code or signal. The host
  can derive the prior `128 + signal` convention with `compatibility_code`.
- Operations after the child is reaped return `Ended` and never signal a
  reused pid or process group.
- Non-Unix builds compile and report `Unsupported` from `open`.

## Extraction state

Contract 003 steps 1 to 7 are done. Acowtancy Desktop migrated in acowtancy-v2 PR 974 (task #690, merged 2026-10-22 at `a7531c19`) and took every Longhorn crate at `v0.4.0`. Its proof was static and compile-only (10 checks); no live PTY run was recorded. Desktop's terminal sessions use `longhorn-pty`; the managed environment, `TERM`/`COLORTERM`, the base64 events and the session ids stay in Desktop's IPC layer. Longhorn's contract and tests are now authoritative.

Step 6 is done by the neutral second consumer at
`examples/desktop-support-conformance/` (task 073). It is the first live PTY
run recorded outside Acowtancy: a `/bin/sh` session checks output, a stdin
round trip, a resize the child reads back, the exit code, and a close that
escalates to `SIGKILL` after the grace period. Limits: one program and one
terminal size, and the proof runs on Unix hosts only.

Open:

- Nothing for steps 1–7.

## Conformance

The public-API integration suite starts real `/bin/sh` and system utility
children. It covers output order, stdin, controlling-terminal access, resize,
exact environment, code and signal exits, 300 ms close escalation, a same-group
grandchild, ended operations, independent sessions, a slow consumer, and the
working directory. A final stress test shares the same binary, so under
`cargo test` it runs as one more thread in the same process: many sessions
at once, each asserting its own exit and its own output marker under churn,
synchronized open bursts, fork-and-exec storms, and close escalations. The
`pty` QA group runs these tests, Clippy, rustdoc, and the generated API
surface check.
