# g02.029 Agent Control Spike

Status: ready
Owner: Longhorn maintainers
Created: 2026-08-19
Governing refs: research memo 024; contract 022 (draft); contracts 001, 020
Depends on: nothing on the live runway

## Outcome

The two runtime unknowns in memo 024 are answered with recorded evidence: a
stateless MCP server mounts inside a running Tauri app and a real agent
client drives it, and WKWebView snapshot capture is proved fresh for an
unfocused, occluded window. Contract 022 can be promoted or corrected from
facts instead of documentation.

## Generation Runway

- [ ] [Card 227](batch-cards/227-agent-control-spike.md) builds the
      standalone prototype and records both probe results in memo 024.

## Acceptance

- Both memo 024 gaps (occluded capture freshness; in-app rmcp mount) carry
  recorded evidence, positive or negative.
- The protocol revision a current agent client actually negotiates is
  recorded.
- The prototype stays outside the workspace under `prototypes/`; no shared
  crate or package is added or changed.
- Findings that contradict contract 022's draft are recorded in memo 024,
  not silently patched into the contract.

## Next Task

Orchestrator promotes memo 024 and contract 022 from the spike evidence,
then compiles g02.030-032 cards to ready.
