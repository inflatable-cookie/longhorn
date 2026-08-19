# g02.033 Agent Control Adoption Kit

Status: ready
Owner: Longhorn maintainers
Created: 2026-08-19
Governing refs: contract 022; contracts 001, 003 (consumer boundary), 012
Depends on: g02.029-032 complete (PRs 2-5)

## Outcome

A consumer app team — human or agent — can adopt the agent-control
surface from Longhorn artifacts alone: a composition guide for the Rust
half, and one canonical, drift-locked skill that teaches an agent to
find a running instance, connect over stateless MCP, and drive the
semantic tools correctly. Proved by an agent following only the skill.

## Generation Runway

- [ ] [Card 235](batch-cards/235-agent-control-guide-and-skill.md) —
      composition guide, canonical skill, instance-finder script.
- [ ] [Card 236](batch-cards/236-agent-control-skill-install-and-drift.md)
      — effigy install selector for consumer repos; a qa check locking
      the skill's documented tool surface to the core vocabulary.
- [ ] [Card 237](batch-cards/237-agent-control-skill-dogfood-proof.md) —
      the skill proved by use: an agent drives the packaged proof app
      following only the skill, evidence committed.

## Acceptance

- The guide covers mount, both exit hooks, `CommandBridge`, and the dev
  feature, sufficient for a consumer card to execute without reading
  Longhorn source.
- The skill teaches discovery, dynamic connection (ephemeral port +
  token), every tool with its limits (untrusted input, DOM-relative
  waits, listen-as-resources, ref staleness), and when to stop and fall
  back rather than fight the surface.
- Skill installation into a consumer repo is one effigy invocation, and
  skill-vs-vocabulary drift fails `qa`, not a reader's patience.
- Consumer repos are not modified by this milestone (contract 003);
  per-app adoption cards run from each app's own roadmap.

## Next Task

After completion: per-app adoption cards in the five consumer repos
(compose the plugin, install the skill). The stdio proxy client
(`longhorn-agent-control-client`, static `.mcp.json`) is recorded in the
candidate runway, not compiled here.
