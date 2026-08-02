# Scripts

Use Effigy from the repo root:

```sh
effigy tasks
effigy doctor
effigy qa
```

Layout protocol work uses the checked Rust generator:

```sh
effigy generate:layout
effigy check:layout-bindings
effigy test:layout-ts
effigy check:layout-package
```

Poodle adapter work verifies and consumes the exact Card 038 tarballs:

```sh
effigy verify:poodle-preview
effigy check:poodle-svelte
effigy test:poodle
effigy check:poodle-package
```

App-shell proof work packs Longhorn and installs three isolated consumers:

```sh
effigy proof:app-shells
```

Settings composition proof packs Longhorn and installs four isolated
consumers:

```sh
effigy proof:settings-composition
```

Nucleus private admission packs the exact Surface-free Longhorn graph, installs
it with Poodle g12.018 in a clean renderer, and checks the locked Rust 1.85 /
Tauri 2.11.5 graph:

```sh
effigy proof:nucleus-private-artifacts
```

Nucleus storage cutover proof reads the pinned consumer commit, validates its
canonical identity, Longhorn profile use, migration adapters, split stores,
and retained-source posture:

```sh
effigy proof:nucleus-storage-cutover
```

Nucleus window cutover proof reads the pinned consumer and Longhorn commits,
checks the protected host, registered placement domain, backup-first legacy
conversion, guarded reveal, retained capability policy, and Surface absence:

```sh
effigy proof:nucleus-window-cutover
```

Nucleus layout cutover proof reads the pinned consumer and Longhorn commits,
checks the exact five-region/four-slot registry, project isolation, registered
persistence and import, expected-revision mutation, product metadata split,
duplicate removal, and Surface absence:

```sh
effigy proof:nucleus-layout-cutover
```

Nucleus renderer cutover proof reads the pinned consumer and Longhorn commits,
checks listener-first project epochs, exact revisioned commands, public Poodle
composition, explicit overlay geometry, lifecycle teardown, private-selector
absence, and the frozen private renderer graph:

```sh
effigy proof:nucleus-renderer-cutover
```

Nucleus native Browser cutover proof reads the pinned consumer and Longhorn
commits, checks the child-view host, checked renderer session, exact viewport
and visibility policy, retained Nucleus browser policy, remote-capability
closure, superseded-path removal, and the frozen private graph:

```sh
effigy proof:nucleus-native-browser-cutover
```

Nucleus migration closeout proof regenerates the current exact private
artifact graph, replays Cards 096-100, runs fresh and legacy restart tests,
audits capabilities and duplicate mechanisms, proves the Surface-free graph,
and verifies the recorded previous-build readback and cleanup gate:

```sh
effigy proof:nucleus-migration-closeout
```

The default donor is `../nucleus`. Set `NUCLEUS_REPO` to an exact clean
checkout when unrelated Nucleus work is in progress.

Loophole Card 102 verifies the pinned read-only hierarchy, storage, Poodle,
command, settings, and 83-variant history freeze:

```sh
effigy verify:loophole-card102-freeze
```

Loophole private admission restores and pins the clean donor baseline, checks
the selected Soundcheck SQLite graph, packs the complete Cards 105-111
Longhorn/Poodle graph, and installs isolated renderer and Rust 1.85 consumers:

```sh
effigy proof:loophole-private-artifacts
```

Loophole Card 110 verifies the exact admitted settings pages, immediate and
staged apply units, storage-only configuration operations, sealed command and
keymap path, one-shot Echo override import, retained macro edge, and redundant
generic donor removal:

```sh
effigy qa:northstar:g01-loophole-card110
effigy qa:northstar:g01-loophole-card111
effigy qa:northstar:g01-loophole-card112
```

The default donors are `../loophole` and `../poodle`. Override them with
`LOOPHOLE_REPO` and `POODLE_REPO` only for exact clean checkouts.

Secondary-consumer Card 113 checks exact Soundcheck, soundcheck-library,
Signal, Bovine, Jetstream, Poodle, and Longhorn receipts; freezes source-backed
behavior and retained authority; rejects selected/forbidden package overlap;
and protects Bovine's exact docs-only worktree overlap:

```sh
effigy qa:northstar:g01-secondary-consumer-card113
```

The default donors are sibling checkouts. Override them with
`SOUNDCHECK_REPO`, `SOUNDCHECK_LIBRARY_REPO`, `SIGNAL_REPO`, `BOVINE_REPO`,
`JETSTREAM_REPO`, or `POODLE_REPO` only for equivalent exact checkouts.

Card 114 freshly packs the exact selected Longhorn and Poodle artifacts,
installs three isolated renderer graphs, checks generated bindings, and proves
three locked/offline Rust 1.85 and Tauri 2.11.5 graphs:

```sh
effigy proof:secondary-consumer-private-artifacts
```

The default receipt is the Card 119 refresh. Card 120 supplies a Bovine-refreshed
receipt through its focused verifier. Set
`SECONDARY_CONSUMER_ADMISSION_FIXTURE` to Card 114's fixture only when running
against its exact historical consumer checkouts.

Card 120 verifies Bovine's canonical native-platform storage, versioned
workspace-preference domain, exact retained legacy import, two-page modal
settings composition, storage-only capability, scoped source diff, and the
refreshed isolated private artifact graph:

```sh
effigy qa:northstar:g01-bovine-card120
```

Card 121 verifies Bovine's debug-only isolated profile, native first launch
and restart receipt, valid and corrupt target evolution, modal session
teardown/remount, exact minimal graph, retained product authority, previous
source rollback suite, and refreshed artifact admission for Jetstream:

```sh
effigy qa:northstar:g01-bovine-card121
```

Card 122 verifies Jetstream's checked editor-state session, listener-first
whole-snapshot recovery, stale-session rejection, sealed command registry,
fresh execution admission, physical keyboard policy, exact four-package
TypeScript and Rust graphs, and retained engine/renderer authority:

```sh
effigy qa:northstar:g01-jetstream-card122
```

The receipt binds Jetstream `2a8afbb7`. Focused Rust, IPC, renderer, and
release-build gates pass. Jetstream's aggregate validation remains blocked by
an unchanged pre-existing denied Clippy lint in the renderer vignette test.

Card 123 verifies Jetstream's stable native-content island, checked backing-
surface coordination, full-host storage versus physical clip, presentation and
semantic-input gates, fresh renderer epochs, Svelte viewport/scale lifetime,
reversible NSView teardown, explicit no-surface policy, and retained engine
authority:

```sh
effigy qa:northstar:g01-jetstream-card123
```

The receipt binds Jetstream `e9a54daa`, six TypeScript packages, and six Rust
crates. Focused Rust, IPC, renderer, deny-warnings Clippy, and release-build
gates pass. Aggregate validation stops only on two unchanged renderer-test
approximate constants. Windows and Linux remain explicit unsupported targets;
live scale transition host evidence remains unmet.

Card 124 closes Jetstream on the exact six-package TypeScript and Rust graphs,
retained Card 114 artifact ids, checked peers and capabilities, one authority
per adopted system, unchanged engine/editor paths, and an isolated previous-
source rollback suite:

```sh
effigy qa:northstar:g01-jetstream-card124
```

The rollback proof passes 19 previous Rust unit tests, four IPC tests, and 65
renderer tests from disposable sibling worktrees. Current focused suites pass
26 Rust unit, six IPC, and 67 renderer tests. No package is published.

Card 125 packs the selected Longhorn and Poodle TypeScript packages, installs
and mounts four isolated renderer shapes, archives the exact Rust inventory,
and runs five locked/offline Rust 1.85 compositions. It checks storage startup,
atomic mutation, visible failure, teardown, graph minimality, and removal of
optional supervision against the frozen receipt:

```sh
effigy qa:northstar:g01-greenfield-card125
```

Set `KEEP_GREENFIELD_COMPOSITION_PROOF=1` only to retain its disposable
artifact workspace. No registry publication is attempted.

Card 126 generates the exact private Rust crate/feature and TypeScript export,
dependency, and peer reference from live manifests. Its guide proof checks
local links, public-only snippets, storage/platform claims, migration safety,
native support limits, and deferred publication. Focused QA also builds Rust
documentation with warnings denied and checks every TypeScript/Svelte surface:

```sh
effigy generate:api-reference-card126
effigy qa:northstar:g01-guides-card126
```

Generation writes only `docs/reference/api-surface.md`. The check selector
fails on drift.

Card 127 produces and checks the coordinated private compatibility candidate:

```sh
effigy generate:private-candidate-card127
effigy qa:northstar:g01-private-candidate-card127
effigy release gates
```

`generate:private-candidate-card127` is the only receipt-writing command. The
proof packs every TypeScript/Poodle artifact twice, inventories and archives
every Rust package twice, installs seven isolated renderer and Rust graphs,
and records exact sources, peers, protocols, platforms, and checksums.
`release gates` is read-only. It runs the private candidate and workspace QA;
it does not prepare, publish, tag, or create a hosted release.

Aggregate Northstar QA runs each completed migration's static closeout gate.
Exact cross-repository HEAD and artifact proofs remain directly addressable by
their card selectors. Advancing or concurrently editing a consumer does not
make an immutable receipt a repository-wide live HEAD invariant.

Set `KEEP_SECONDARY_CONSUMER_ARTIFACT_PROOF=1` only when the disposable proof
workspace must be inspected after the run.

Soundcheck Card 115 verifies the exact storage/config/window cutover commits,
the retained SQLite authority, isolated override boundary, hidden protected
window host, and the same-layout inventory bound:

```sh
effigy qa:northstar:g01-soundcheck-card115
```

The default donor is `../soundcheck`. Override it with `SOUNDCHECK_REPO` only
for a checkout containing the recorded cutover commit. Card 119 closes the
fresh/restart matrix; the visible native GUI path is recorded as environment
unmet where computer control timed out.

Soundcheck Card 116 verifies the sealed modal settings registry, exact admitted
config capabilities, one-domain Agent Review apply unit, native SQLite backup
inventory and confirmation-bound restore adapter, removed renderer authority,
and retained soundcheck-library commit:

```sh
effigy qa:northstar:g01-soundcheck-card116
```

The default sibling checkouts are `../soundcheck` and
`../soundcheck-library`. Override them only with `SOUNDCHECK_REPO` or
`SOUNDCHECK_LIBRARY_REPO` checkouts containing the recorded commits. Card 119
closes native restore/restart conformance.

Soundcheck Card 117 verifies the database-reconciled payload-free operation
authority, revision-bound and identity-checked cancellation, retry lineage,
isolated renderer session, removed direct cancellation command, and retained
soundcheck-library job/report authority:

```sh
effigy qa:northstar:g01-soundcheck-card117
```

The default sibling checkouts are `../soundcheck` and
`../soundcheck-library`. Override them only with `SOUNDCHECK_REPO` or
`SOUNDCHECK_LIBRARY_REPO` checkouts containing the recorded commits. Card 119
closes scan/restart and cancellation-race conformance.

Soundcheck Card 118 verifies the exact isolated-window cutover and lock
receipts, generation-bound helper channel, listener-before-launch adapter,
bounded request history, constrained resize receipts, disposable-process
teardown, checked observation-only renderer client, explicit macOS support,
and retained Soundcheck/Signal authority:

```sh
effigy qa:northstar:g01-soundcheck-card118
```

The default sibling checkouts are `../soundcheck` and `../signal`. Override
them only with `SOUNDCHECK_REPO` or `SIGNAL_REPO` checkouts containing the
recorded commits. Card 119 closes the automated helper matrix; visible live
plugin interaction remains explicitly environment unmet where computer control
timed out.

Soundcheck Card 119 refreshes the full private artifact graph, replays Cards
115-118, verifies the isolated product profile, audits exact dependencies and
capabilities, checks the storage/window/settings/scan/helper matrices, and
verifies the recorded previous-build readback:

```sh
effigy proof:soundcheck-card119
```

The visible plugin-GUI click path is recorded as environment-unmet when macOS
computer control cannot enumerate applications. The proof does not widen
Windows, Linux, scale, package-publication, or live-data claims.

Prefer Effigy for generic operations. If Longhorn needs repo-owned automation,
use TypeScript with Bun. Bash is thin glue only; Python needs a concrete
technical reason.
