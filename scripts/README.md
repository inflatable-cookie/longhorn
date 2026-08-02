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

The default receipt is the latest Card 119 refresh. Set
`SECONDARY_CONSUMER_ADMISSION_FIXTURE` to Card 114's fixture only when running
against its exact historical consumer checkouts.

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
