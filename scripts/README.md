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
```

The default donors are `../loophole` and `../poodle`. Override them with
`LOOPHOLE_REPO` and `POODLE_REPO` only for exact clean checkouts.

Prefer Effigy for generic operations. If Longhorn needs repo-owned automation,
use TypeScript with Bun. Bash is thin glue only; Python needs a concrete
technical reason.
