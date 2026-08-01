# Operation Progress, Cancellation, Retention, And Teardown

Date: 2026-07-31
Card: 076
Roadmap: g01.012

## Result

Completed the pure `longhorn-operation` authority. Operations now carry
bounded monotonic overall and phase progress, explicit cancellation support,
optional terminal retry lineage, and canonical structural metadata weight.
Every successful progress or lifecycle mutation advances both operation and
catalogue revision.

Cancellation returns `accepted`, `already_requested`, `unsupported`, or
`terminal`. Accepted running work enters `cancelling`; the executor may still
prove success, failure, cancellation, or interruption. Accepted queued work
becomes terminal only because authoritative `queued` state confirms execution
never started.

## Retention And Teardown

Catalogue limits independently bound active count, retained terminal count,
and retained terminal encoded weight. Active records are never candidates.
Terminal pruning is deterministic oldest-first and every removal has an exact
receipt. Explicit dismissal is terminal-only and does not claim artifact or
log deletion.

Retry registers a distinct operation and may reference one retained terminal
source. It never reopens the source. Controlled teardown requires exactly one
revision-bound terminal or transfer outcome for every active operation. The
batch validates before mutation, commits atomically, and permanently closes
the old authority. Renderer projection disposal remains read-only.

## Boundary Audit

The crate still depends only on `longhorn-core`. It owns no executor,
cancellation token, scheduler, product payload, persistence, clock, bridge,
Tauri, Svelte, Poodle, or notification behavior. Donor repositories remain
unchanged.

## Validation

- `effigy test:operation-core` passes the core and operation suites
- 18 operation contract fixtures cover the complete lifecycle plus Card 076
  matrices
- strict `longhorn-operation` clippy and workspace Rust formatting pass
- progress, cancellation, retention, retry, teardown, and exact failed-state
  invariance fixtures pass
- Northstar operation-authority paths and roadmap posture are checked by a
  dedicated Effigy selector

## Next Task

Execute Card 077. Generate the payload-free operation protocol and prove equal
direct, Tauri, and bridge-domain semantics.
