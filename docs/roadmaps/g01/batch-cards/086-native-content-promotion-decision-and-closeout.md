# 086 Native-content Promotion Decision And Closeout

Status: complete
Owner: Tom
Roadmap: g01.013 batch 4
Governing refs: contracts 001, 003, 009, 010, 012, 013, and 017;
research memo 017
Depends on: Cards 083, 084, and 085
Auto-start next card: no
Completed: 2026-07-31

## Objective

Compare the private pure and packaged mechanism evidence. Choose one exact
production disposition, update every canonical boundary, and close g01.013
without treating prototypes as release packages.

## Decision

`Promote` the pure coordination kernel, three independently selectable
mechanism layers, checked TypeScript client, and per-instance Svelte lifecycle.
Keep Poodle as a public consumer composition seam without a native-content
package dependency.

Initial native-host support is macOS-only. Child-view Windows and Linux remain
unproved. Isolated-window and backing-surface Windows and Linux remain
unsupported. Mixed-display/live native scale switching remains unproved for
child-view and backing-surface; deterministic 1x/2x conversion passes.

Retain Cards 082-085 workspaces as non-publishable evidence. g01.018 Cards
087-093 implement and artifact-prove production packages before donor writes.

## Gate Decision

| Gate | Result | Disposition |
| --- | --- | --- |
| pure common semantics | 21 passing tests; three lossless traces | promote kernel |
| dependency isolation | pure and mechanism graphs exclude unselected/product stacks | promote split graph |
| child view | packaged macOS core matrix passes; focus/visibility may be unknown; live scale unavailable | promote macOS-first; other targets unproved |
| isolated window | packaged macOS 11/11 matrix passes | promote macOS-only |
| backing surface | packaged macOS 10 passing checks; live scale unavailable | promote macOS-only |
| TypeScript/Svelte reuse | checked state and mounted measurement/gating recur across shapes | promote separate client and Svelte packages |
| Poodle value | public layout seam is sufficient | no Poodle-specific package |
| authority leakage | browser, plugin, GPU, semantic input, native handles, and placement stay external | contract preserved |

## Scope

- three-shape semantic trace comparison
- scale, geometry, visibility, focus, failure, and teardown matrix
- packaged macOS evidence review
- Windows and Linux per-mechanism support ledger
- independent Rust and renderer dependency graphs
- browser, plugin, GPU, input-payload, native-handle, and Poodle authority audit
- framework-neutral, Svelte, and Poodle adapter value assessment
- production package and compatibility decision
- prototype retention, archive, or removal
- canonical docs, roadmap, and g01.013 closeout
- post-decision production-card compilation when promoted

## Decision Outcomes

### Promote

Promote the pure coordination package and only the independently proved
mechanism and renderer adapters. Compile a bounded production runway. Do not
copy private prototype code blindly or claim unproved targets.

### Narrow

Promote only the lossless pure coordination seam or a strict subset of
mechanisms. Record why the excluded adapters remain consumer-owned.

### Retain

Keep the evidence as non-publishable research. Record the unmet reuse,
platform, teardown, or dependency gates. No donor migration follows.

### Reject

Remove or archive the prototypes and record why independent consumer hosts are
clearer than a shared coordination API.

## Out Of Scope

- silent prototype promotion
- production implementation in the decision card
- donor repository writes or migration
- registry publication or release
- cross-platform claims without target evidence
- browser, plugin, or GPU payload promotion

## Steps

1. Review every Cards 082-085 acceptance and stop gate.
2. Compare pure traces and mechanism-specific operation maps.
3. Compare packaged scale, focus, visibility, destruction, and teardown.
4. Compare minimal and selected-adapter dependency graphs.
5. Audit product payloads, raw handles, capabilities, and Poodle boundaries.
6. Classify macOS, Windows, and Linux support per mechanism.
7. Choose exactly one decision outcome.
8. Update architecture, contract, topology, inventory, spec, and research truth.
9. Retain, archive, or remove prototype artifacts per the decision.
10. If promoted or narrowed, compile the next production card runway and make
    only its first fully bounded card ready.
11. Record migration prerequisites without changing donors.
12. Run full Effigy QA and close g01.013.

## Acceptance Criteria

- exactly one disposition is recorded with gate-by-gate evidence
- the common vocabulary remains lossless across every retained mechanism
- native viewport effects remain mechanism-specific
- unused mechanisms stay absent from minimal graphs
- public claims match actual packaged and target evidence
- no prototype artifact masquerades as a production package
- Svelte and Poodle edges exist only if evidence shows reusable value
- browser, plugin, GPU, input, and outer-window authority remain external
- donor migration remains blocked until production artifacts exist
- all front doors agree on the selected next lane
- full Effigy QA passes

## Evidence Required

- gate-by-gate Cards 082-085 decision table
- normalized three-shape trace comparison
- packaged behavior and per-target support matrix
- Rust and renderer dependency inventories
- authority, capability, payload, native-handle, and Poodle audit
- production package decision and prototype disposition proof
- canonical docs diff and compiled follow-up runway when applicable
- migration prerequisite table
- closeout log and full QA

## Stop Conditions

- evidence supports multiple materially different production boundaries
- operator product preference is required to choose between valid outcomes
- one retained mechanism violates contract 017 authority
- packaged evidence is incomplete or contradictory
- dependency isolation fails without combining mechanisms
- a cross-platform claim lacks target evidence
- full QA fails from lane changes

## Next Task

Execute ready Card 088. Card 087 implements the production pure kernel without
promoting prototype source. Do not start donor migration.
