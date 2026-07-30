# Config-backed Settings Apply Units

Date: 2026-07-29  
Card: g01.008 / 043  
Status: complete

## Outcome

Added fresh checked configuration mutation and the config-backed settings apply
unit. Card 044 is ready.

## Coordinator Proof

The existing `ConfigStore::mutate` path already acquired one process/file
coordinator, recovered interrupted restore state, reread and validated the
latest domain, patched, validated, and atomically published before releasing
the guard.

The missing surface was inspection and veto. `ConfigStore::mutate_checked`
extends that same path with a fresh context containing decoded state, schema,
origin, diagnostics, and exact source bytes. A check error publishes nothing.
Accepted unchanged bytes return no publication receipt. No second lock or
settings-owned persistence path was added.

## Settings Adapter

`longhorn-settings-config` supplies:

- one sealed apply unit bound to one ordinary writable config domain
- consumer-owned typed projection, intent, validation, patch, reset, and
  activation stages
- configured/default/effective/policy/editability projection validation
- host token and process-local monotonic scope revision tracking
- registry, page, apply-unit, and scope checks
- forced-policy and editability rejection before patch
- consumer constraint validation under the config coordinator
- exact conflict snapshot, changed/unchanged outcome, and config durability
- reset limited to named projected user overrides
- corrupt/unavailable recovery snapshots
- activation computed only after configuration success
- explicit consumer transaction authority with a separate receipt type

The built-in binding cannot span domains. It rejects storage classes whose
authority is not an ordinary writable user, machine, or workspace config file.

## Evidence

Fourteen contract tests cover:

- immediate and staged commands through one apply path
- missing/default, changed, and unchanged state
- exact fresh source evidence
- helper-process intervening writer conflict with byte invariance
- configured value retained under a forced effective policy
- constrained, read-only, hidden, and unsupported rejection
- invalid staged intent byte invariance
- named reset preserving another override, secret authority, and another domain
- corrupt recovery without rewrite
- publication failure before rename and zero activation calls
- exact recovery-token change from source bytes
- separately receipted consumer transaction authority
- dependency exclusion for Tauri, layout, Surfaces, windowing, Svelte, and
  Poodle

Validation:

- `effigy test:settings-config`
- `cargo clippy -p longhorn-config -p longhorn-settings-config --all-targets -- -D warnings`
- `effigy qa:northstar:g01-settings-config`
- `effigy qa`
- `effigy scan god-files --json`

All requested tests and QA pass. The god-file scan remains at the exact known
baseline: 59 findings, 58 warnings, and one high finding in
`crates/longhorn-tauri-windowing/src/lifecycle/model.rs`. Card 043 adds no
finding.

## Limits

- scope revisions are host-process authority; opaque tokens bind current
  encoded config plus checked field and policy projection
- managed policy format and product schemas remain consumer authority
- cross-domain atomicity requires the explicit consumer transaction seam
- TypeScript, Tauri, Svelte, Poodle, and shared recovery pages remain Cards
  044-047
