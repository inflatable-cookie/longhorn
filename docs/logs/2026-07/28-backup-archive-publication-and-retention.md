# Backup Archive Publication And Retention

Date: 2026-07-28
State: complete implementation batch

## Outcome

- pinned Rust 1.85-compatible `zip` 5.1.1 with default features disabled and
  only zlib-rs-backed DEFLATE enabled
- added deterministic strict version-1 ZIP encoding from immutable snapshots
- added bounded, side-effect-free plaintext archive inspection
- added verified staged publication for operational roots and explicit exports
- added same-app operational listing and safe deterministic retention

## Archive Boundary

The writer emits the manifest first, then payloads in lexicographic order.
Every entry uses the 1980 ZIP epoch, regular `0600` mode, no comments or extra
fields, and fixed DEFLATE level 6 for non-empty data. Receipts hash the exact
produced bytes; compressed bytes across dependency versions are not a
compatibility promise.

Inspection first preflights the raw central directory. This catches duplicate
names and unsupported methods before the ZIP parser can normalize them. It
then enforces finite archive, entry-count, path, per-entry, total, and
compression-ratio bounds. Data stays in private memory. No generic extraction
API writes to a filesystem.

The reader accepts only Stored and DEFLATE regular files. It rejects absolute,
escaping, NUL, duplicate, directory, link, device, encrypted, undeclared,
missing, out-of-order, non-canonical, unknown-format, and checksum-damaged
input.

## Publication

Operational roots and user-selected export targets are separate authority
types. Archive kind must match destination class. Export replacement requires
an explicit enum choice.

Publication writes a unique private sibling partial, syncs it, closes it,
reopens it through bounded inspection, checks the complete archive hash,
renames once, and syncs the parent when durable publication is required.
Failed verification removes the partial and leaves no published target.

## Listing And Retention

Only fully inspected same-app non-export archives become candidates. Locked,
corrupt, unreadable, foreign, unknown-format, duplicate-id, user-export, and
unmanaged entries remain outside deletion with diagnostics. An incomplete
bounded directory scan cannot produce a prune plan.

Candidates order by strict manifest time, archive id, then path. Count, age,
and milestone tiers form a keep union with operation pins and the
just-published archive. Age and milestone buckets anchor to the newest valid
manifest time, never wall clock, filename, or mtime. Clock regression is
reported.

Deletion plans bind each exact root-level path to its complete archive
SHA-256. Application rereads bounded bytes and refuses deletion if the file
changed after planning.

## Evidence

- canonical layout, metadata, deterministic encoding, Stored/DEFLATE, strict
  manifest, exact inventory, and checksum fixtures
- absolute, traversal, NUL, duplicate, directory, symlink, device,
  unsupported-method, entry-count, per-entry, aggregate, and ratio attack
  fixtures
- operational/export authority, overwrite, archive-kind, durability receipt,
  corrupt staging, and cleanup fixtures
- corrupt, locked, foreign, unknown-format, unmanaged, bounded-scan, equal
  time, clock-regression, age, milestone, pin, new-archive, stale-plan, and
  successful deletion fixtures
- Rust 1.85 focused tests, workspace tests, rustdoc, installed-toolchain
  clippy, Effigy doctor, test plan, and full QA

## Boundary

This batch adds no restore planning, migration staging, live-domain mutation,
journal, rollback, age encryption, custom adapter execution, Tauri,
TypeScript, Svelte, or Poodle dependency.

## Posture

`strict-ready`

Card 006 is complete. Card 007 is the only ready implementation lane.

## Next

Execute card 007: non-mutating restore inspection, confirmation-bound
planning, and complete private staging. Do not start it from card 006.
