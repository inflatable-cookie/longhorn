# 025 Bounded Support Logs

Status: active compiled boundary
Owner: Longhorn maintainers
Created: 2026-10-19
Depends on: contracts 001, 003, 012 and 024
Affects: `longhorn-support-log`, desktop support consumers

## Problem

Desktop support needs a short record of what failed: an in-memory tail for a
diagnostics bundle, a small log file for update and install steps, and a
bounded tail of process output. Acowtancy Desktop holds three such logs. They
share rotation code by copy, disagree on time zones, and one cuts output
before it redacts, so a cut through a PEM private-key block kept key body
lines. Every Longhorn desktop app needs one tested copy.

Source analysis: section "Support and tooling" of the Praxis map
[`shared-tooling-extraction.md`](https://github.com/inflatable-cookie/praxis/blob/dbad7b093f0452800c4df3a96c0798ebf788b08f/docs/knowledge/architecture/shared-tooling-extraction.md)
at `dbad7b0`. Donor: `acowtancy/acowtancy-v2` at `89c8bf568`, under
`apps/bovine-desktop/src-tauri/src`: `diagnostics/log.rs`,
`app_update/update_log.rs` and `managed_tools/install_log.rs`. From this
contract on, this file owns the behaviour; the map and the donor are
evidence.

## Generic versus product

- Longhorn owns the ring log, the rotating log, the tail and field helpers,
  the line formats, the rotation rule and the timestamp format.
- The app owns every value and name: capacities, field and file bounds, tail
  bounds, retained rotations, the log directory and its lookup (including
  environment overrides), file names, JSON field names and record shape,
  attempt IDs, and any process-wide instance. The crate has no defaults and
  no statics.
- The app supplies the `longhorn_redact::Redactor` (contract 024) and so
  decides which secrets and rules apply.

Logs are support evidence, not configuration state. The crate is not a
contract 004 storage domain: no schema version, no migration, no backup, no
recovery. A lost or rotated-away log is not data loss.

## Public API

Crate `longhorn-support-log`: depends on `longhorn-redact` and `std` only,
no features, no TypeScript binding.

- `utc_timestamp(SystemTime) -> String`.
- `Level { Error, Warning, Info }`, `label()` → `error`, `warning`, `info`.
- `RingLog::new(capacity, field_bytes, redactor)`; `record(level, scope,
  message)` stamps now, `record_at(at, …)` stamps `at`; `tail(limit)` returns
  the newest `limit` records, oldest first; `len()`, `is_empty()`.
- `RingRecord`: `at()`, `level()`, `scope()`, `message()`, `line()`. Only a
  `RingLog` makes one.
- `RotatingLogConfig { directory, file_name, max_bytes, retained,
  field_bytes }`.
- `RotatingLog::open(config, redactor)`, `RotatingLog::discarding()`;
  `record(event, detail)`, `record_at(at, event, detail)`,
  `record_json(&JsonRecord)`, `directory()`, `stats() -> LogStats { written,
  oversized, failed }`.
- `JsonRecord::new()` with `text`, `optional_text`, `integer` (`i64`) and
  `optional_integer`. Fields keep the order the caller adds them.
- `bound_field(&Redactor, text, max_bytes) -> String`.
- `bounded_tail(&Redactor, text, TailBounds { lines, bytes }) -> String`.
- `OMITTED_MARKER`: `[earlier output omitted]\n`.

## Redact, then bound

Every text that enters a log or a helper goes through the caller's
`Redactor` before any cut. No public path bounds raw text and then writes or
returns it.

- A field (ring scope and message, rotating event and detail, every JSON key
  and text value) is redacted, then clipped to `field_bytes`, then escaped.
  The field bound counts redacted bytes before escaping.
- JSON values are redacted one by one, before the line is serialized, never
  as a serialized line: a redaction span can run across a JSON quote.
- `bounded_tail` redacts the whole text, then takes lines, then bytes. A
  PEM private-key block is one marker line before any cut, so no cut can
  start inside it.
- A cut in redacted text can split a `[redacted]` marker. That exposes
  nothing.

## Bounds

- **Ring:** at most `capacity` records. A full ring drops its oldest record
  as the newest arrives. Capacity 0 holds nothing.
- **Field:** at most `field_bytes` bytes, cut back to a character boundary.
  The cut keeps the head.
- **Tail:** the last `lines` lines of the redacted text, split as
  `str::lines` splits (`\n` or `\r\n`) and joined by `\n`. If that is over
  `bytes`, the result is `OMITTED_MARKER` followed by the newest bytes that
  fit; the cut moves forward to a character boundary. The result is never
  over `bytes`. A byte bound smaller than the marker returns an empty
  string.
- **File:** no file the log writes is ever over `max_bytes`. A line is one
  whole line or nothing. A line over `max_bytes` is dropped unwritten and
  counted in `oversized`.

## Lines

- Timestamps are UTC, RFC 3339, with exactly three fraction digits and `Z`:
  `2026-10-01T12:00:00.000Z`. Sub-millisecond time is truncated toward the
  past, never rounded. Dates are proleptic Gregorian, computed with `std`
  only.
- Ring line: `<timestamp> [<level>] <scope>: <message>`.
- Rotating text line: `<timestamp> <event> <detail>\n`.
- Rotating JSON line: one JSON object, then `\n`. Strings escape `"`, `\`,
  `\n`, `\r`, `\t`, and other control characters as `\u00XX`; other
  characters are written as they are.
- Interior CR and LF are never written raw. In text and ring lines each CR
  becomes the two characters `\r` and each LF the two characters `\n`; a
  backslash is not escaped. In JSON they are the JSON escapes.

## Rotation

Before each write, the log reads the live file's size at its path. If the
file is not empty and the line would take it over `max_bytes`, the log:

1. drops its open handle;
2. removes `<file_name>.<retained>`;
3. renames `<file_name>.<n>` to `<file_name>.<n+1>`, from `retained - 1`
   down to 1;
4. renames the live file to `<file_name>.1`.

With `retained` 0 it removes the live file. If the live file is still over
the bound after this, the line is dropped and counted in `failed`.

The size check reads the path on every write, so the bound holds over a
long-lived handle, after a restart that finds a full file, and after
something else removes the live file (the stale handle is dropped and the
file reopened). Writes through one `RotatingLog` are serialized by its lock,
so concurrent threads never interleave lines. Two `RotatingLog` instances,
or two processes, on one file are not supported.

## Failure rules

- Logging never panics and never fails its caller. No method returns an
  error.
- A failed directory creation, open, rotation or write drops the line and
  counts it in `failed`. A failed write also drops the handle, so the next
  write reopens.
- An empty `directory` writes nothing and counts each line in `failed`. Use
  `discarding()` for a log that writes nowhere; it opens no file and counts
  nothing.
- A poisoned lock is recovered, not propagated. Every update under a lock
  completes before the guard drops, so the recovered state still holds the
  bounds.

## Compatibility for consumers

Against the Acowtancy donor at `89c8bf568`:

- **All three:** capacities, bounds, file names, the directory and the
  record shape are now caller values. Acowtancy keeps `LOG_CAPACITY` 400,
  `LOG_TAIL_LIMIT` 100, `MAX_LOG_BYTES` 512 KiB, two retained rotations,
  the 48 KiB field bound, the 200-line and 64 KiB tail bounds, the file
  names, `default_log_directory` with `BOVINE_UPDATE_LOG_DIR`, attempt IDs
  and its `OnceLock` statics.
- **`diagnostics::log`:**
  - Lines carry milliseconds: `2026-10-01T12:00:00.000Z`, not
    `2026-10-01T12:00:00Z`.
  - Scope and message are redacted and bounded when recorded; the donor
    stored them raw and unbounded.
  - `line()` escapes interior CR and LF; the donor wrote them raw.
  - A poisoned lock no longer drops the record or returns an empty tail.
- **`app_update::update_log`:**
  - Timestamps are UTC with `Z`. The donor wrote local time with its offset.
  - A line over the file bound is dropped and counted. The donor wrote it
    into an empty file, so one line could exceed the bound.
  - Event and detail are redacted with the app's `Redactor` and bounded. The
    donor trusted callers to pass redacted detail.
  - Interior CR and LF are escaped; the donor wrote them raw.
  - The retained count is a caller value. The rotation order is unchanged.
  - A rotation that cannot clear the live file drops the line instead of
    appending past the bound.
  - An empty directory counts a failure instead of acting as a discarding
    log.
- **`managed_tools::install_log`:**
  - `JsonRecord` replaces the serde `Record`; Acowtancy names the fields.
    Keys are redacted too.
  - Timestamps come from `utc_timestamp`: UTC, not local time.
  - Every text value is clipped to the field bound, tails included. The
    donor clipped only `detail`. The clip keeps the head, so a tail longer
    than the field bound loses its newest output: keep tail bytes at or
    under field bytes.
  - `bounded_tail` redacts, then cuts. The donor cut, then redacted, so a cut
    inside a PEM private-key block kept key body lines, and the result could
    exceed the byte bound. Line counts can change: a redacted block is one
    line, so more earlier lines fit. In the donor's consumer proof the
    direct tail now keeps 194 old lines, as its disk tail did, not 192.
  - One handle stays open between writes; the donor opened the file per
    write. Failed and oversized lines are counted in `stats()`.

## Required Evidence

- `effigy check:support-log` (QA group `support-log`): Clippy, rustdoc with
  warnings denied, the API-reference inventory, and `test:support-log`. The
  tests use the public API only. They run the donor tests (rotation at the
  bound, a retained handle over many writes, the discarding log, ring
  capacity and tail order, the JSONL record, the 200-line and 64 KiB tail),
  then: a tail cut inside a PEM private-key block, cuts next to multi-byte
  UTF-8, two threads writing at once, a restart over a full live file,
  interior CR and LF, an oversized line, failed directory creation, failed
  writes, a rotation that cannot clear the live file, and timestamps at
  fixed instants including a leap day, a non-leap century, a sub-millisecond
  carry and pre-epoch time.

## Extraction state

Contract 003 steps 1–4 are done: donor behaviour named, donor tests ported,
this contract, and the crate.

Open:

- Step 5: Acowtancy Desktop migrates its three logs after a Longhorn release
  carries this crate. That is Acowtancy's task.
- Step 6: second-consumer conformance has no named consumer yet.
- Steps 7–8 follow the migration.

## Stop Conditions

- A product wants its own value or name compiled into the crate.
- A public path would bound or write text before it is redacted.
- A change would let a written file exceed `max_bytes`, or write part of a
  line.
- Logs become durable state that must survive or migrate; that is contract
  004's ground.
