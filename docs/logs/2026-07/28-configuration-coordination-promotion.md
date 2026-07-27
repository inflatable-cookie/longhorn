# Configuration Coordination Promotion

Date: 2026-07-28  
State: complete research and planning batch

## Outcome

- audited configuration write and lock behavior across Loophole, Nucleus,
  Soundcheck, Jetstream, and Bovine
- confirmed no donor has a reusable cross-process file-lock protocol
- characterized Rust rename, macOS/Linux advisory locking, Windows replacement,
  `fs4`, `atomic-write-file`, and `cap-std`
- promoted one stable store-wide lock layered with a process-local mutex
- bounded the guarantee to cooperating processes on one machine and supported
  local filesystems
- separated atomic visibility from verified power-loss durability
- kept capability-relative mutation as a hard boundary
- compiled one ready coordinated atomic-mutation card

## Decision

Ordinary local stores receive an injected coordination authority. The Tauri
adapter normally places `.longhorn/config.lock` below app data. Writers acquire
a process-local mutex and then an exclusive advisory OS lock on that stable
file.

The guard starts before authoritative reread and ends after publication,
durability handling, and receipt creation. Public mutation patches the freshly
reread value. It does not expose blind whole-file replacement.

The lock file persists. Presence, age, or PID content is never stale-lock
evidence. Process failure releases the owning handle.

Remote and multi-machine mutation require an external transaction authority.

## Scope Kept Out

- debounce and shutdown flush
- cross-domain transactions
- migration rewrite before backup
- archive, restore, encryption, and retention
- TypeScript, Svelte, and Tauri bindings
- consumer migration

## Posture

`strict-ready`

The lock-model gate is closed. Card 002 is the only ready implementation card.

## Next

Execute coordinated atomic configuration mutation when implementation resumes.
Compile debounce and flush only after that card closes.
