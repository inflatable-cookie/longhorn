# g02.047 Rust-side Agent Selection

Owner: Tom
Created: 2026-09-26
State: ready for Queue delivery
Governing refs: contract 022 (`Agent-Answerable Selection`, amended
2026-09-26), contracts 006 and 010
Depends on: g02.045 (complete, in `0.2.1`)
UI classification: none — consumer picker controls are unchanged

## Outcome

A consumer's Rust command that opens a native picker can let an agent answer
it over MCP, through the same pending-selection registry as the JS route.
Soundcheck's two `blocking_save_file` sites — library backup export and
manifest save — become agent-answerable after consumer adoption. A human's
picker still opens the dialog plugin unchanged.

## Context and confirmed decisions

Operator direction 2026-09-26: origin is an explicit token the consumer
carries from the page to its own command; Longhorn never infers origin on the
host from recent agent input. Soundcheck is the live acceptance target,
consumer-owned and proved after Longhorn merges on fixtures.

The JS route decides origin in the page shim (`isAgentOriginated`, sequence
counters over trusted `pointerdown`/`keydown`/`click` against agent input).
A Rust command invoked from that page cannot see the counters. The
consumer's JS already calls `invoke(...)` for these commands, so it can pass
the origin it reads at call time.

`SelectionRegistry::begin` in `longhorn-agent-control` and
`plugin_result` already implement request, answer, expiry, rejection, and
shutdown. `longhorn_agent_control_begin_selection` in
`longhorn-tauri-agent-control` is the JS-facing bridge; this task adds the
Rust-facing one.

## Work

1. In `packages/longhorn` (`agent-control` entry), export a function that
   returns the page's current selection origin as a serialisable value for
   command arguments. It reads the installed shim; with no shim it reports
   human.
2. In `longhorn-tauri-agent-control`, add a Rust entry point a consumer
   command calls with that origin, the invoking `Webview`, the registry, and
   the picker request (kind, directory, multiple, filters, title, default
   path). Agent origin begins a pending selection bound to the webview's
   window and webview and resolves with the same result shape as the JS
   route. Human, missing, or malformed origin returns an explicit "use the
   plugin" outcome and publishes nothing. There is no native fallback on the
   agent path.
3. Keep everything under the existing `agent-control` feature. A default
   build has no new symbol or asset; extend the release-absence scan's
   positive and negative controls to the new entry points.
4. Document the Rust route in the composition guide, the crate README, and
   `skills/agent-control/SKILL.md`, showing a `blocking_save_file` call site
   before and after.

## Acceptance and review oracle

| Invariant | Required proof |
| --- | --- |
| Agent Rust path works | fixture: agent origin → pending resource with window/webview → `answer_selection` → Rust caller receives the path; no dialog call |
| Human path unchanged | fixture: human, absent, and malformed origin → "use the plugin" outcome, nothing published, registry generation unchanged |
| Lifecycle matches JS route | reject yields cancellation; expiry, shutdown, duplicate, wrong-instance, and malformed-save answers fail typed and settle once |
| Origin is page truth | TS fixture: origin reads human after a trusted event following agent input, agent after agent input, human with no shim |
| Feature boundary holds | `effigy check:agent-control-release-absence` covers the new symbols both ways |
| Consumer adoption (after merge, consumer-owned) | Soundcheck manifest save or backup export answered over MCP, unfocused, no OS panel |

Run the focused agent-control Rust and TypeScript checks, the release-absence
check, then `effigy qa`. Independent review must inspect origin handling for
any path where a human picker could be captured.

## Dispatch manifest

- **Capability:** Rust/TypeScript cross-language with Tauri host and MCP
  experience; no UI design worker.
- **Owned paths:** `crates/longhorn-tauri-agent-control/**`,
  `crates/longhorn-agent-control/src/selection.rs` (only if the Rust route
  needs a registry accessor), `packages/longhorn/src/agent-control/**`,
  `packages/longhorn/tests/agent-control/**`, `skills/agent-control/**`,
  `docs/guides/agent-control-composition.md`,
  `scripts/verify-agent-control-release-absence.ts`, and regenerated
  bindings / `docs/reference/api-surface.md`.
- **Reserved closeout:** this card, `docs/roadmaps/g02/README.md`,
  `docs/roadmaps/generation-index.md`, contract 022, and the delivery log.
- **Concurrency:** runs alongside g02.049 (disjoint paths; if g02.049 lands
  first, rebase over its proof-script changes). g02.048 follows this task
  serially — both own the agent-control shim and skill.
- **Completion:** all fixture rows pass, docs and skill match behavior; the
  Soundcheck row is recorded as pending consumer adoption. No release is
  implied.
- **Escalation:** Chatterbox for any need to infer origin on the host, to
  change the JS route's behavior, or to widen filesystem authority.

## Stop conditions

Stop if the route can capture a human picker, if a selected save path causes
a Longhorn write, if the consumer would need to change its dialog options or
result handling beyond passing origin, or if the feature boundary cannot
exclude the new symbols. Return the finding to Chatterbox.
