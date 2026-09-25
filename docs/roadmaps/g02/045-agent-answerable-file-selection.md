# g02.045 Agent-Answerable File Selection

Owner: Tom
Created: 2026-09-22
State: Longhorn implementation merged in PR #34; Figmatic live acceptance open
Governing refs: contract 022 (`Protocol`, `Availability And Security`,
`Agent-Answerable Selection`), contracts 006 and 010
Depends on: g02.043 and g02.044 (complete)
UI classification: none — the consumer's existing picker controls remain

## Outcome

An agent driving an opted-in app over contract 022 can answer a pending
Tauri JS `open` or `save` request with paths through MCP. Figmatic's
`ProjectsView` can create or import a project from a fresh storage leaf
without a human operating the folder panel. Human picker behavior remains
the plugin's behavior.

## Context and confirmed decisions

The operator chose Longhorn ownership, `open`/directory and `save` together,
availability in both dev and packaged `agent-control` builds, and Figmatic
as the live acceptance target (2026-09-22). Four consumers use
`@tauri-apps/plugin-dialog` from JS at roughly 48 call sites. Figmatic's
`ProjectsView` calls `open({ directory: true })` directly from its handler;
its command bridge answers `Unsupported`, so contract 022's command route
cannot reach this flow. JS dialog results are paths, requiring no file-byte
transfer. Soundcheck's one HTML file input uses a real `File` and stays a
separate follow-up; it already offers a paste-JSON route.

## Ready-state rubric

- [x] Ownership, scope, availability, and acceptance target are operator-set.
- [x] Contract 022 defines the request, answer, expiry, security, human-path,
      and native-dialog boundaries.
- [x] Existing MCP resource push and the packaged agent-control opt-in supply
      the transport and availability boundaries.
- [x] No UI redesign or consumer product policy is needed in Longhorn.

## Work

1. Add a per-instance pending-selection registry in `longhorn-agent-control`
   with bounded lifetime and explicit settlement. Publish pending state at
   `longhorn://agent-control/selection` through `subscriptions/listen`, with
   resource reads showing still-pending requests even when a subscriber joins
   after request creation. Include request id, kind, multiplicity, filters,
   title, default path hint, window, and webview.
2. Add `answer_selection { id, paths }` and `reject_selection { id, reason }`.
   Enforce instance identity, cardinality, existing open-path kind, and
   exactly-once settlement. Explicit rejection resolves as plugin-compatible
   `null`. Return typed errors for unknown, expired, settled, and malformed
   answers.
   Expiry, cancellation, and instance shutdown must not leave the caller
   awaiting indefinitely. `save` chooses a target; Longhorn never writes it.
3. Bridge requests and results through the Tauri host and add an explicit
   `longhorn` TypeScript replacement for plugin-dialog `open`/`save` at
   consumer call sites. It must retain the plugin's option and result shapes.
   Agent-originated calls publish a request before opening any OS panel;
   human-originated calls invoke the original plugin unchanged. An active
   server by itself is not evidence that a human picker should be captured.
   Do not globally monkey-patch plugin-dialog.
4. Keep the surface under the existing `agent-control` feature in dev and
   packaged builds. A default build contains no selection endpoint or shim;
   the `agent-control-evaluate` feature is unnecessary. Update the composition
   guide, installed skill, and release-absence check for discovery and use.
5. Obtain Figmatic-owned consumer adoption and a live proof from a fresh
   storage leaf: drive New project or Import project through an MCP client,
   answer the folder request with a chosen path, and reach the project view
   while the app remains unfocused. Figmatic owns its call-site changes and
   project/path policy; Longhorn does not write its repository in this task.

## Acceptance and review oracle

| Invariant | Required proof |
| --- | --- |
| Agent path works | MCP client receives the pending resource, answers with a path, and the JS call resolves with the plugin-compatible result without an OS panel |
| Save is bounded selection | single target returned; no Longhorn file write; malformed or multi-path save fails typed |
| Request lifecycle is honest | late subscriber reads pending state; explicit reject yields `null`; expiry, cancellation, shutdown, duplicate, and wrong-instance answers fail typed and settle once |
| Human path is unchanged | human-originated `open` and `save` pass original options to plugin-dialog and preserve results even while the agent-control server is active |
| Packaged boundary holds | no-feature artifact has no selection surface; `agent-control`-only packaged build can answer without `evaluate`; token, Origin, and loopback checks hold |
| Consumer blocker closes | Figmatic fresh-leaf New or Import project succeeds over MCP with no human picker and no OS focus |

Use focused agent-control and TypeScript checks, feature-state and lifecycle
fixtures, `effigy check:agent-control-release-absence`, then `effigy qa` after
the batch. Record live Figmatic evidence separately from fixtures. Independent
review must inspect the actual agent-versus-human routing and save authority.

## Dispatch manifest

- **Capability:** Rust/TypeScript cross-language implementation with Tauri
  host and MCP resource experience; no UI design worker.
- **Owned Longhorn paths:** `crates/longhorn-agent-control/**`,
  `crates/longhorn-tauri-agent-control/**`, `packages/longhorn/**`, related
  agent-control fixtures and proof scripts, `skills/agent-control/**`, and
  the agent-control composition guide. Contract changes after dispatch return
  to Chatterbox.
- **Reserved closeout:** this card, `docs/roadmaps/g02/README.md`,
  `docs/roadmaps/generation-index.md`, `docs/contracts/022-agent-app-control.md`,
  and the delivery log. Stage explicit paths.
- **Concurrency:** no Longhorn sibling lane currently ready. Figmatic
  consumer adaptation is separately owned and follows availability of the
  Longhorn entry point; cross-repo writes need their own authorization.
- **Completion:** all oracle rows pass, the Figmatic acceptance receipt is
  linked, docs and fixtures match behavior, and a release is not implied.
- **Escalation:** Chatterbox for any routing ambiguity, widened filesystem
  authority, incompatible plugin result shape, or need for a second server.

## Stop conditions

Stop if agent origin cannot be distinguished without changing human picker
behavior, if a selected save path causes a Longhorn write, if the seam needs
native OS panel control or a file-byte bridge, or if Figmatic needs a product
decision about which paths may be accepted. Return the finding to Chatterbox.

## Evidence and remaining gate

Longhorn implementation merged through [PR #34](https://github.com/inflatable-cookie/longhorn/pull/34)
at `3b3db6c7` (worker head `fc9ad714`) with independent exact-head review
`5832387981`. Queue task `716bcb17-a24e-43a0-aeff-f4e140a32a8c` reached
`done` after hook-owned closeout at `703012ab`. The worker's `effigy qa`
passed; reviewer reran focused registry/MCP, TypeScript origin/selection,
and release-absence checks. The [implementation log](../../logs/2026-09/25-agent-answerable-selection-implementation.md)
records the evidence and limits.

The Figmatic fresh-leaf New or Import project proof has **not** run. This
card's consumer acceptance row remains open until a Figmatic-owned call-site
adoption uses a source-linked Longhorn candidate, answers the pending folder
request over MCP, and records the live result. Queue `done` closes the
Longhorn PR loop; it does not claim the consumer proof or a release.
