# Dependency Refresh Sweep

Date: 2026-08-04
Card: 148
Roadmap: g02.007

## Result

Compatible transitive drift closed (37 crates) and all four held-back pins
decided with conformance evidence: rusqlite and zip bumped across their
gaps, sha2 bumped with an explicit hex seam, ts-rs taken to the 11.x head
with the 12 major deferred to its own card.

## Decisions

- **rusqlite `=0.31.0` → `=0.40.1` — bumped.** API migration confined to
  test fixtures (`DatabaseName::Main` → `MAIN_DB`, `u64` → `i64` row get).
  SQLite native-snapshot, grouped-restore, WAL, and storage-transition
  suites green; picks up two years of bundled-SQLite CVE fixes.
- **zip `=5.1.1` → `=7.2.0` — bumped to the 1.85 ceiling.** Zero source
  changes; the deflate-flate2-zlib-rs feature survives two majors. All
  backup-archive publication, determinism, retention, and restore suites
  green. The 8.x line raises MSRV to rustc 1.88 and waits on a Rust
  toolchain-floor decision.
- **ts-rs `=11.0.0` → `=11.1.0` — minor bumped; 12 deferred.** All 13
  binding domains byte-current on 11.1.0. The 12 major changes the
  generator API at 605 `decl()` call sites plus large-int emission
  semantics — a protocol-affecting migration that needs its own bounded
  card, not a sweep item.
- **sha2 `0.10.9` → `0.11.0` — bumped.** The digest output array lost
  `LowerHex`; hex encoding now goes through one explicit helper behind
  `Sha256Digest`. All frozen digest fixtures pass unchanged, proving
  byte-identical output.
- **tauri 2.10.3 → 2.11.5 (compatible drift) — one test fix.** 2.11
  hardens the ACL so non-local origins always resolve capabilities; the
  bridge mock-invoke test now uses the local `tauri://localhost` origin.
  Production behavior unaffected (consumers configure real capabilities).

## Exact Evidence

- `cargo update` applied, then bounded by the Rust 1.85 gate: darling,
  plist, serde_with, time, idna_adapter, writeable, and the icu 2.2 family
  pinned back to their newest 1.85-compatible versions (the fresh releases
  raised MSRV to 1.86-1.88); 22 transitive crates now deliberately behind
  latest, each either upstream-constrained or MSRV-pinned
- 149 workspace test suites green; Clippy and fmt clean; full `effigy qa`
  passes
- all 13 binding checks report current; no golden fixture or archive
  changes
