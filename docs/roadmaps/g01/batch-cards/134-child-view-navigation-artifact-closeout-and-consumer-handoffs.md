# 134 Child-view Navigation Artifact Closeout And Consumer Handoffs

Status: complete
Owner: Tom
Roadmap: g01.020 batch 3
Governing refs: contracts 003, 012, and 017; Cards 132-133
Depends on: Card 133
Auto-start next card: no
Completed: 2026-08-03

## Objective

Prove the revised child-view artifact in isolation, preserve every common
boundary, and leave exact read-only resume gates for Figmatic and Nucleus.

## Scope

- private Rust source artifact and isolated consumer compile
- common native-content protocol drift and dependency isolation
- API/reference and package inventory
- Figmatic g03.006 resume handoff
- Nucleus raw-label navigation replacement note
- Longhorn closeout and full QA

## Out Of Scope

- consumer repository writes
- package-manager publication
- common renderer navigation commands
- immediate Nucleus migration claims

## Steps

1. Pack and compile the child-view crate from its produced source artifact.
2. Prove pure and renderer native-content fixtures are unchanged.
3. Audit dependency, raw-handle, URL-payload, and capability boundaries.
4. Publish adapter usage guidance and exact consumer handoffs.
5. Run focused artifact gates and full Effigy QA.
6. Close g01.020 without editing Figmatic or Nucleus.

## Acceptance Criteria

- isolated consumers resolve no workspace or sibling source
- pure and renderer protocol digests remain unchanged
- navigation remains native-side and policy-admitted
- Figmatic can retain one preview webview across selection changes
- Nucleus has one explicit follow-up to remove global label lookup
- full QA passes

## Evidence Required

- artifact identity and dependency graph
- protocol digest comparison
- packaged Card 133 report
- closeout and two consumer handoff logs
- full QA receipt

## Stop Conditions

- produced artifacts differ from workspace-tested behavior
- common protocol or capabilities expand
- a consumer must change before generic proof can pass

## Next Task

Return the completed navigation primitive to Figmatic g03.006. Nucleus may
schedule its separate adapter-control cleanup without blocking Figmatic.

## Evidence

- the private source artifact compiles `current_url` and `navigate` from an
  isolated Nucleus-shaped Rust consumer
- the isolated graph contains only core, native-content, and the selected
  child-view mechanism
- the fixture and generated TypeScript protocol digests remain exact and
  contain no navigation payload
- package guidance keeps URL normalization, command authorization, ordering,
  and history downstream
- separate Figmatic and Nucleus handoff logs name the remaining consumer work
- focused navigation QA and full Effigy QA pass
