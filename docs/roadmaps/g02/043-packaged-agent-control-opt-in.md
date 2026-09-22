# g02.043 Packaged Agent-Control Opt-In

Owner: Tom
Created: 2026-09-22
Governing refs: contract 022 (`Availability And Security`, `Tool Surface`)
Depends on: no other task
UI classification: none

## Outcome

`longhorn-tauri-agent-control` exposes a compile-time opt-in — `agent-control` —
that a consumer may enable in a packaged release build, with `evaluate` split
into `agent-control-evaluate` (off by default, expected only in dev/test). The
dev-only stance is replaced; loopback binding, the per-instance bearer token, and
Origin validation are unchanged.

## Context

Contract 022 was amended 2026-09-22 under operator direction from the Figmatic
lane: a packaged build must be able to include the server so an external harness
drives the app with the operator's agency through a consumer-registered typed
command catalogue. Contract 023's Swallowtail-integrated production MCP role is
withdrawn; the contract 022 server is the production MCP.

## Ready-State Rubric

- [x] The operator directed the change (Figmatic lane, 2026-09-22).
- [x] Contract 022 amended; contract 023's production MCP role withdrawn.
- [x] Figmatic is the first consumer and sets `features = ["agent-control"]`.
- [x] UI classification is none.

## Work

1. Rename the Tauri host `dev` feature to `agent-control`; it ships the server,
   the generic input/snapshot/screenshot/wait_for tools, and the `CommandBridge`
   seam.
2. Split `evaluate` behind `agent-control-evaluate`, off by default and
   propagated to the core crate; a build without it answers typed `Unsupported`.
3. Re-scope `check:agent-control-release-absence`: no `agent-control` feature →
   no surface (the total-exclusion claim holds for the default); an
   `agent-control`-only build → server present and `evaluate` absent; both
   features → `evaluate` present.
4. Update the example proof, the composition guide, and the install skill to the
   new feature names, and document the packaged posture: loopback/token/Origin
   unchanged, the application starts the server, and the registered command
   catalogue is the allowed agency.
5. Tests and fixtures for the three feature states.

## Acceptance and review oracle

| Invariant | Required proof |
| --- | --- |
| Default build has no surface | absence scan on a build with neither feature |
| Packaged opt-in carries the server | an `agent-control`-only build carries the server and no `evaluate` markers |
| `evaluate` is opt-in | without `agent-control-evaluate` the tool answers typed `Unsupported` |
| Security unchanged | loopback, token, and Origin fixtures still pass |

Validation uses the focused agent-control selectors plus
`check:agent-control-release-absence`, then `effigy qa`. An independent
exact-head review must inspect the three feature states, not prose.

## Stop conditions

Stop if the split cannot keep the default build free of the surface, or if any
change would let the server start without an explicit application call. Escalate
to Chatterbox.

## Evidence

On completion, record: the feature shape, the re-scoped absence proof, the
example/skill updates, and the test results.
