# 004 Configuration Coordination And Atomic Mutation

Status: promoted  
Owner: Tom  
Updated: 2026-07-28  
Promotes to: contract 004; g01.002 batch 2

## Question

How should Longhorn serialize safe configuration mutation across store
instances and processes without weakening capability-confined file access or
claiming unsupported remote-filesystem guarantees?

## Donor Evidence

The five audited Tauri donors use useful pieces of the write protocol but no
complete cross-process coordinator.

| Donor | Evidence | Gap |
| --- | --- | --- |
| Loophole | `echo-configuration/src/native_adapter.rs` and `echo-profile-config/src/workspace_storage.rs` write a same-directory temporary file, sync it, then rename | deterministic temporary names, no directory sync, no process lock |
| Soundcheck | `src-tauri/src/app_settings.rs` serializes through a static process mutex and renames a temporary JSON file | process-local only; no file or directory sync |
| Soundcheck Keepsake | `src-tauri/src/keepsake_settings.rs` uses a PID/sequence temporary name, `create_new`, file sync, rename, and cleanup | no process lock or directory sync |
| Nucleus | workspace/editor persistence includes direct writes and local temporary-file replacement | no shared coordinator |
| Jetstream and Split-shell | no reusable configuration coordination mechanism found | no shared coordinator |

No audited donor declares `fs2`, `fs4`, `fd-lock`, or an equivalent Rust file
lock dependency. Longhorn can retain the proven replacement shape. The
coordination contract is new shared infrastructure, not extracted donor code.

## Platform And Library Evidence

- Rust [`std::fs::rename`](https://doc.rust-lang.org/std/fs/fn.rename.html)
  replaces an existing target and rejects moves across mount points. Temporary
  and target files therefore belong in the same directory.
- [`fs4`](https://docs.rs/fs4/latest/src/fs4/lib.rs.html) provides shared,
  exclusive, and non-blocking whole-file locks. It uses `flock` on Unix and
  `LockFileEx` on Windows. Version 1.1.0 declares Rust 1.75, below Longhorn's
  Rust 1.85 MSRV.
- Apple documents [`flock`](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/flock.2.html)
  as advisory: cooperating processes observe it; unrelated writers can ignore
  it. Non-blocking acquisition reports `EWOULDBLOCK`.
- Linux documents [`flock`](https://man7.org/linux/man-pages/man2/flock.2.html)
  as tied to an open file description and released when all owning descriptors
  close. Network filesystem behavior varies by filesystem, mount options, and
  kernel version.
- Windows
  [`MoveFileEx`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw)
  exposes replacement and write-through flags. Rust's rename contract exposes
  replacement, but not a portable directory-sync or write-through guarantee.
- [`atomic-write-file`](https://docs.rs/crate/atomic-write-file/latest/source/README.md)
  demonstrates file sync, atomic replacement, cleanup, and Unix directory
  sync. Its public API starts from ambient paths rather than an existing
  capability directory. Its generic non-Unix path does not establish the
  Windows write-through guarantee Longhorn would need to claim.
- `cap-std` supplies relative open, rename, cloning, and file sync operations
  from an already opened directory capability. This preserves the domain
  store's path-confinement boundary during mutation.

## Promoted Decision

### One store-wide coordination lock

Every ordinary local store receives an explicit coordination authority. The
Tauri adapter normally places it in the app data root. The stable lock file is:

```text
.longhorn/config.lock
```

The lock file is infrastructure. It is never used as a status flag, replaced,
or deleted during normal operation. Locking a domain file is insufficient:
atomic replacement changes the file object while another process may retain a
lock on the old object.

All participating writers acquire, in order:

1. a process-local mutex keyed by the coordination authority
2. an exclusive OS advisory lock on the stable lock file

One lock covers every domain in the store. It avoids per-domain ordering and
cross-domain deadlocks, and gives backup and restore one future coordination
boundary. Configuration writes are short and infrequent enough that the
reduced parallelism is acceptable.

The in-process mutex is still required. OS lock behavior for separately opened
handles in one process differs across platforms, and consumers can construct
more than one store instance.

### Transaction lifetime

The exclusive guard starts before the authoritative reread. It remains held
through patch application, validation, encoding, temporary-file publication,
durability handling, and receipt creation.

Standalone reads do not acquire the lock. Atomic replacement lets them observe
one complete old or new document. Any read-modify-write path must reread while
holding the guard. Public mutation accepts a typed patch operation against
that fresh value; it does not expose a blind whole-document overwrite.

The guard is non-reentrant. Internal operations receive an unforgeable guard
token instead of acquiring the coordinator again.

### Acquisition and failure

Mutation receives a finite wait policy. The implementation uses non-blocking
lock attempts against a monotonic deadline. It never waits forever.

Failures remain typed:

- `busy` or `timeout` when another writer retains the lock
- `io` when the lock file cannot be opened or operated
- `unsupported` when the requested authority cannot provide the contract

Host adapters run blocking filesystem coordination away from the Tauri UI
thread.

Closing or dropping the owning handle releases the OS lock after process
failure. The persistent file is not a stale lock. Its presence, contents, age,
or recorded PID never grants permission to remove it or bypass acquisition.

### Authority limit

The default coordinator covers cooperating Longhorn processes on one machine
and a filesystem with supported local lock semantics. Advisory locks cannot
protect against direct external writers.

Longhorn does not infer cross-machine safety for NFS, SMB, synced folders, or
other remote authorities. A `project-shared` domain may use the local
coordinator only when the consumer explicitly owns a single-machine local
authority. Multi-machine mutation requires a server or other external
transaction authority. Secrets continue through the secure-store adapter.

Compatible app releases must retain the same coordination authority and lock
path.

## Atomic Commit Shape

The first implementation should use `cap-std` directly:

1. open the registered target parent from its root capability
2. create a unique hidden temporary file with exclusive creation
3. write the exact versioned envelope and sync the file
4. close the temporary writer
5. rename within the same parent capability
6. sync the parent directory where supported and report the achieved
   durability level
7. clean up an unpublished temporary file after failure

The temporary name includes enough process-local uniqueness to avoid
collisions and retries exclusive creation. Unix files start at mode `0600`;
Windows uses the parent directory's inherited ACL. The implementation must not
reopen the target through an ambient absolute path after validation.

Atomic visibility and power-loss durability are separate claims. A platform
without verified directory synchronization may still report atomic
replacement, but cannot report fully durable publication. A caller that
requires full durability receives an error rather than a false receipt.

## Migration Boundary

An in-memory migrated value is not writable in this batch. Contract 004
requires a verified pre-migration backup before destructive rewrite. Mutation
returns `migration-backup-required` until the backup batch supplies that
operation.

Missing and current valid documents are mutable. Recovery, future-version,
unavailable-authority, and migrated-in-memory outcomes are not.

## Rejected Options

| Option | Reason |
| --- | --- |
| process mutex only | does not coordinate two app processes |
| lock each target file | atomic replacement can detach the lock from the current pathname |
| lock per domain | complicates cross-domain ordering and future backup/restore without useful config-write throughput |
| presence/PID lock file | crash recovery and PID reuse make stale detection unsafe |
| infinite blocking acquisition | can hang shutdown, workers, or UI-facing commands |
| direct `atomic-write-file` adoption | ambient-path API weakens the established capability-root contract; Windows durability claim remains incomplete |
| assume network lock equivalence | platform documentation explicitly varies by filesystem and mount behavior |

## Required Proof

- two store instances in one process cannot lose unrelated patch updates
- two helper processes serialize on the same stable lock
- timeout is finite and typed
- killing a lock holder permits later acquisition without deleting the file
- a persistent unlocked file is harmless
- injected failures before publication preserve the previous valid document
- successful reads see a complete old or new envelope
- unpublished temporary files are cleaned up
- capability confinement still rejects parent symlink escape
- achieved durability is explicit in the mutation receipt
- Rust 1.85 check and packaged Windows validation remain release evidence

## Promotion Result

Contract 004 now owns local coordination, transaction lifetime, authority
limits, and durability reporting. The decision unblocks one implementation
card for coordinated atomic configuration mutation. Backup, restore,
encryption, debounce, TypeScript/Tauri adapters, and remote coordination remain
later work.
