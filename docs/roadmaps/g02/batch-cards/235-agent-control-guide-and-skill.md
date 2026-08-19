# 235 Agent Control Guide And Skill

Status: ready
Owner: Longhorn maintainers
Roadmap: g02.033
Governing refs: contract 022; contracts 001, 003; the Card 230-234
closeouts (mount, capture, listen mechanics)
Depends on: g02.032 complete (PR 5)
Auto-start next card: yes — Card 236 in the same lane

## Objective

The two adoption artifacts exist: a composition guide for the Rust half,
and the canonical agent skill (plus its instance-finder script) for the
agent half.

## Scope

- **Composition guide** — `docs/guides/agent-control-composition.md`,
  in the register at `docs/guides/README.md`, matching the existing
  guides' shape. Covers: the `dev` cargo feature and why it must never
  reach release profiles; `mount_agent_control` from `setup`;
  hooking **both** `ExitRequested` and `RunEvent::Exit` (the Card 230
  finding, stated as a requirement with the strand-the-discovery-file
  consequence); `CommandBridge` wiring into the app's contract-006
  registry; what the app gets (tool list) and what it must not expect
  (native surfaces, trusted input, non-macOS capture). Written so a
  consumer card executes from the guide alone, with
  `examples/agent-control-proof` cited as the worked example.
- **Canonical skill** — `skills/agent-control/SKILL.md` (new top-level
  `skills/` directory; the installer mirrors it into consumer repos'
  `.claude/skills/`). Frontmatter: name, description with concrete
  trigger phrases (testing the running app, driving the UI, screenshots
  of the app), and a `longhorn_version` stamp. Body teaches, in order:
  1. when to use this instead of OS computer use (always, for a
     dev-featured Longhorn app) and how to check the app is running;
  2. discovery: the state-dir path per platform, the file schema, the
     stale-pid rule, the finder script as the one-step version;
  3. connection: `claude mcp add --transport http` with the URL and
     `Authorization: Bearer` header from the finder output, and the
     raw-POST fallback for clients without config access (stateless —
     every call is one self-contained POST);
  4. the tool surface, each tool with its contract: snapshot refs are
     live-DOM and go stale honestly (`UnresolvedRef` → re-snapshot,
     never retry blind); waits are DOM-relative and time/rAF waits do
     not exist (and why — WKWebView throttling); input is untrusted
     (no native hover/drag, `isTrusted` false); screenshots are fresh
     even occluded/minimized; commands reach the app's registry;
     events are `resources/updated` on the three
     `longhorn://agent-control/*` URIs with a drop counter;
  5. multi-agent etiquette: instances are interleave-safe, refs are
     shared, pick your instance by app id and pid;
  6. stop rules: what is out of scope (native dialogs and menus — use
     commands; other Spaces; release builds) and when to tell the
     operator instead of falling back to OS input.
- **Finder script** — `skills/agent-control/scripts/find-instance.ts`
  (bun, matching `scripts/` idiom): enumerates the discovery dir,
  filters live pids, optional app-id argument, prints instance URL and
  the ready-to-paste `claude mcp add` line; exits nonzero with a clear
  message when nothing is live. Unit-testable against a temp discovery
  dir; redaction rule: prints the token only in the paste line, never
  in diagnostics.

Redaction discipline per AGENTS.md: the skill and script are executable
surfaces, not prose — no placeholder values that break when copied.

## Acceptance Criteria

- [ ] guide present, indexed, and sufficient: every step a consumer
      card needs, with both exit hooks and the dev-feature rule stated
      as requirements
- [ ] skill present with version stamp and trigger-phrase description;
      tool table matches the wire vocabulary exactly (Card 236 locks
      this with a check — write it to be lockable: one parseable table)
- [ ] finder script fixtures: live instance found, stale skipped,
      empty dir → nonzero with message, token absent from diagnostics
- [ ] no consumer repo touched
- [ ] `effigy qa` passes

## Validation

`effigy qa`; finder-script fixtures; `effigy doctor`.

## Stop Conditions

- the skill needs to document behavior the merged surface does not
  actually have — that is a gap report, not skill prose;
- the guide cannot be written without exposing app-specific authority
  decisions — contract 003 boundary question, stop.

## Continuation

Card 236 makes installation one command and locks the skill against
drift, in the same lane.
