# Soundcheck Plugin-scan Operation Cutover

Date: 2026-08-02
Roadmap: g01.016
Card: 117
State: complete; Card 118 ready

## Result

Soundcheck now projects the database-backed plugin-scan lease through one
Longhorn operation authority. The database job id is the operation id;
`soundcheck.plugin-scan` and `soundcheck:library` supply kind and scope. The
catalogue is process-epoch scoped, admits one active operation, retains 16
terminal structural records, and rebuilds current truth from the database on
every snapshot boundary.

Start, status, startup drift, and unit retry reconcile the existing
soundcheck-library job before and after product work. Same-active reuse stays
in the coordinator. A newly observed external job interrupts only a stale
structural projection; it does not alter database truth. Terminal jobs found
after restart register and close in one reconciliation pass.

Cancellation now enters through Longhorn's revision-bound command. The
injected executor checks the exact database job id before calling the native
coordinator. Accepted cancellation stays `cancelling`; scanner evidence may
still close it as succeeded, failed, or cancelled. Late product status cannot
reopen a terminal Longhorn record.

The renderer owns one listener-first `OperationSession`. It loads a fresh
snapshot before cancellation and uses structural state for the app-level scan
gate. Teardown stops observation only. The direct renderer cancellation
command was removed. Product polling, format/unit progress, warnings,
diagnostics, result wording, catalog reload, and targeted retry remain in
soundcheck-library.

## Exact Evidence

- Longhorn source: `0dd3c890e2d9b298ef3ae65016595c21e48bdbc7`
- prior Soundcheck: `af715e2fafca8c14d91bd352740f671ff41f24e9`
- Soundcheck cutover: `2faae9a94182283b2539c4ad16a9a9fad52e50c3`
- retained soundcheck-library: `40418635674d598f7e311bf311c3165300edd35b`
- Soundcheck Cargo lock SHA-256:
  `4b5f4f69bbcbf0dbb077edd2ca7363c47d3e0a8dd9e65dfd7db2148f23b6130c`
- Soundcheck Bun lock SHA-256:
  `fdf9a4f0b439f7dfa4db9a30f8cce70092d2865d65542a2fcae17e7c2c231f1b`
- fixture:
  the migration receipt (retired 2026-08-10; in git history)
- verifier: removed 2026-08-10 — Longhorn no longer keeps consumer-aware proofs; the recorded fixture is the retained evidence

## Validation

Soundcheck passes 108 app-library tests with two local-Codex tests ignored,
29 renderer tests, production Svelte checking, and production bundling. The
exact verifier checks the commits and locks, payload-free Longhorn protocol,
database reconciliation, cancellation identity, retry lineage, removed
direct command, renderer teardown, retained donor behavior, and absent
bridge/notification/scheduler dependencies.

No live scan or database mutation was run. Fresh, restart, cancellation-race,
and native GUI conformance remains Card 119.

## Next

Execute Card 118. Adopt isolated-window coordination around Soundcheck's
disposable plugin-inspection helper while keeping plugin, audio, screenshot,
and helper-process policy downstream.
