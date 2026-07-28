# 006 Backup Archive Publication And Retention

Status: planned after card 005
Owner: Tom
Roadmap: g01.002 batch 3
Governing refs: contracts 001, 004, and 012; research memo 006
Auto-start next card: no

## Objective

Encode a captured configuration snapshot as the strict version-1 ZIP bundle,
verify and publish operational backups or user exports, inspect plaintext
archives safely, and prune only proven retention candidates.

## Scope

- Rust 1.85-compatible standard ZIP dependency characterization
- strict manifest JSON encoding and decoding
- fixed layout, entry order, timestamp, permissions, and compression policy
- Stored/DEFLATE reader with archive-bomb and path limits
- duplicate, undeclared, link, device, directory, traversal, and unknown
  version rejection
- payload SHA-256 and archive SHA-256 verification
- private sibling partial, reopen verification, atomic publication, durability
  receipt, and cleanup
- injected operational root versus explicit export destination
- explicit export overwrite authority
- same-app operational listing and safe count/age/milestone retention
- pins, new-archive preservation, clock-regression diagnostics, and safe
  treatment of damaged or foreign files

## Public Behavior

The writer accepts only an immutable card-005 snapshot. It never reacquires
the configuration guard.

The version-1 archive:

- writes `longhorn/manifest.json` first
- writes declared payloads in lexicographic path order
- normalizes entry metadata
- uses DEFLATE for non-empty writer payloads
- emits no directory, link, comment, or ambient metadata entries

Inspection reads with finite entry, path, compressed, uncompressed,
compression-ratio, and aggregate limits. It requires an exact declared entry
inventory and verifies SHA-256 independently of ZIP CRC.

Operational publication and export share encoding but not destination
authority or retention. Retention never guesses from filename or mtime and
never deletes an archive it cannot successfully inspect as a same-app
operational candidate.

## Out Of Scope

- restore-domain compatibility, plan binding, migration staging, or mutation
- restore journal, safety backup, rollback, or crash recovery
- age encryption and locked archive listing
- custom snapshot adapters
- consumer migration or UI

## Acceptance Criteria

- a fixed snapshot produces the required entry names, order, metadata, and
  strict manifest
- reader accepts Stored and DEFLATE and rejects every other compression method
- unknown fields and format versions fail safe
- duplicate, undeclared, absolute, escaping, NUL, link, device, and directory
  entries fail before extraction
- finite limits reject high ratio, oversized entry, aggregate overflow, and
  excessive entry count
- checksum mismatch distinguishes damaged payload from parse failure
- failed encode or verify leaves no published destination
- publication receipt states archive hash and achieved durability
- export refuses accidental overwrite and never enters retention
- retention always keeps the new archive and active pins
- count/age/milestone selection is deterministic under equal times and clock
  regression
- corrupt, unreadable, foreign, unknown-version, and arbitrary files survive
  pruning with diagnostics
- Rust 1.85 remains supported

## Stop Conditions

- the selected ZIP dependency raises the Rust floor
- generic archive extraction can write directly into live roots
- reader safety depends on path mangling instead of exact declaration
- retention must trust filesystem mtime, filename parsing, or unauthenticated
  sidecars
- pruning can remove an uninspectable archive
- the card expands into restore or encryption

## Next Task

Run after card 005 closes. Then activate restore inspection and planning.
