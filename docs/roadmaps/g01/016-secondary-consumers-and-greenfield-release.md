# g01.016 Secondary Consumers And Greenfield Release

Status: complete
Owner: Tom
Updated: 2026-08-02
Governing refs: contracts 003-007, 009-010, 012-017; g01.018;
`../../architecture/secondary-consumer-migration-map.md`

## Outcome

Prove Longhorn in Soundcheck, Bovine, and Jetstream; leave four buildable
greenfield compositions and a deterministic private `0.1.0` compatibility
candidate. Package-manager publication, release tags, and hosted releases are
not part of this roadmap.

## Generation Runway

This milestone advances g01 from main-donor migration into materially
different secondary consumers and a greenfield adoption path. It proves the
smallest useful graph, optional native-content mechanisms, product-authority
retention, and produced-artifact use before the remaining fork-tree lane.

Immediate runway:

1. refresh read-only consumer behavior and authority evidence
2. admit one exact private artifact graph
3. migrate Soundcheck sequentially
4. migrate Bovine only after unrelated-work clearance
5. migrate Jetstream sequentially
6. prove greenfield compositions and usage docs
7. close on a private compatibility candidate without publication

The next planning checkpoint follows Card 127. It selects the remaining g01
lane from current evidence; it does not infer package publication.

## Execution Plan

### Batch 1: common admission

- [x] freeze exact current behavior, authority, overlap, and rollback inputs
- [x] prove selected private artifacts for all three consumer shapes
- [x] admit only the first bounded Soundcheck write slice

### Batch 2: Soundcheck

- [x] migrate stable-name storage, app config, and protected primary window
- [x] compose product settings with shared storage/backup/recovery modules
- [x] adopt structural operation authority for plugin scan
- [x] adopt isolated-window coordination for disposable plugin inspection
- [x] prove exact artifacts, behavior, rollback, and retained sibling authority

### Batch 3: Bovine

- [x] preserve unrelated docs work and migrate one minimal preference domain
- [x] compose the minimal settings shell and exact Poodle artifacts
- [x] close with no layout, Surface, service, command, history, or native-content edge

### Batch 4: Jetstream

- [x] adopt checked bridge state and command/keymap projection
- [x] adopt backing-surface coordination and Svelte viewport lifetime
- [x] prove engine, renderer, world, WGPU, and semantic-input authority remain local

### Batch 5: greenfield and private release

- [x] build minimal, workspace, full-hosting, and optional-server examples
- [x] publish no packages; document APIs, storage, backup, composition, and migration
- [x] prove deterministic private `0.1.0` artifacts and compatibility
- [x] close g01.016 with registry ownership and publication visibly deferred

## Goals

- [x] each app adopts only useful packages
- [x] Soundcheck uses `com.inflatablecookie.soundcheck` plus stable name `Soundcheck`
- [x] Bovine proves the smallest config/settings graph
- [x] Jetstream proves bridge, command, and backing-surface composition
- [x] product, sibling, renderer, plugin, content, and engine authority stay downstream
- [x] examples prove optional dependency boundaries outside workspace resolution
- [x] filesystem, backup, settings, topology, command, history, and native-content docs are exact
- [x] one coordinated private compatibility candidate is reproducible

## Acceptance Criteria

- [x] exact clean selected Longhorn/Poodle sources and consumer receipts are recorded per cutover
- [x] every source-linked consumer graph has matching produced-artifact proof
- [x] each migrated concern has one active structural authority
- [x] rollback uses retained sources and exact receipts, not dual writes or silent fallback
- [x] Bovine's unrelated work and authored content are untouched
- [x] Soundcheck's SQLite, scan, plugin, DAW, Composer, Keepsake, and Signal authority remain local
- [x] Jetstream's command execution, renderer, WGPU, frame, world, and input authority remain local
- [x] examples contain no donor vocabulary, umbrella dependency, or copied Poodle primitive
- [x] compatibility claims name MSRV, peers, protocol range, platform, and evidence
- [x] npm/Cargo publication, registry ownership, tags, and hosted releases remain deferred

## Batch Cards

Ready:

- none

Active:

- none

Planned:

- none

Complete:

- `batch-cards/113-secondary-consumer-behavior-authority-and-rollback-freeze.md`
- `batch-cards/114-secondary-consumer-private-artifact-admission.md`
- `batch-cards/115-soundcheck-storage-config-and-window-cutover.md`
- `batch-cards/116-soundcheck-settings-backup-and-recovery-cutover.md`
- `batch-cards/117-soundcheck-plugin-scan-operation-adoption.md`
- `batch-cards/118-soundcheck-isolated-window-coordination-cutover.md`
- `batch-cards/119-soundcheck-migration-conformance-and-closeout.md`
- `batch-cards/120-bovine-config-and-settings-cutover.md`
- `batch-cards/121-bovine-minimal-composition-conformance-and-closeout.md`
- `batch-cards/122-jetstream-bridge-command-and-keyboard-cutover.md`
- `batch-cards/123-jetstream-backing-surface-coordination-cutover.md`
- `batch-cards/124-jetstream-migration-conformance-and-closeout.md`
- `batch-cards/125-greenfield-composition-matrix.md`
- `batch-cards/126-api-storage-composition-and-migration-guides.md`
- `batch-cards/127-private-0-1-compatibility-candidate-and-closeout.md`

## Planning Checkpoint

Cards 113-114 freeze exact authority and admit the three selected private
artifact graphs without consumer writes. Cards 115-119 complete Soundcheck.
Cards 120-121 complete Bovine's canonical storage, one preference domain,
minimal settings shell, settings lifetime, failure/restart/rollback matrix,
exact artifact graph, and no-optional-system closeout. Card 122 completes
Jetstream's checked editor-state bridge, sealed command catalogue, fresh
availability admission, and physical-keyboard cutover. Card 123 completes the
checked backing-surface, Svelte viewport, scale, input-gate, and reversible
native-teardown cutover. Card 124 closes exact artifacts, peers, capabilities,
duplicates, retained engine authority, and isolated previous-source rollback.
Card 125 proves four produced-artifact greenfield shapes, exact optional graph
boundaries, storage first-load/mutation/reload, visible renderer lifecycle,
and serverless local authority. Card 126 adds checked public API, package
selection, storage/backup, composition, migration, rollback, and compatibility
guides. Card 127 proves the coordinated private candidate, all seven declared
graphs, exact peers/protocols/platform claims, and both read-only release
gates. g01.016 is complete.
Consumer writes remain sequential so each app closes before the next opens.

Bovine's current dirty files are unrelated Northstar/schema-coverage docs.
They are preserved and are not write authority for g01.016. Card 120 must
recheck overlap before editing any Bovine path.

The release portion ends at a local, deterministic candidate. Working package
names, registry ownership, public compatibility ranges beyond proof, package
publication, tags, and hosted releases remain outside standing `continue`
authority.

## Risks

- Soundcheck's external SQLite and disposable helper need honest adapter boundaries.
- Bovine is actively changing and may require overlap clearance at write time.
- Jetstream native-content support is macOS-only and its current view is process-lived.
- greenfield examples can accidentally become an umbrella framework.
- release language can overstate registry or cross-platform availability.

## Next Task

Continue with g01.017 Card 070. Package publication remains deferred and still
requires separate explicit operator action.
