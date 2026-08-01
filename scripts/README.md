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

The default donor is `../nucleus`. Set `NUCLEUS_REPO` to an exact clean
checkout when unrelated Nucleus work is in progress.

Prefer Effigy for generic operations. If Longhorn needs repo-owned automation,
use TypeScript with Bun. Bash is thin glue only; Python needs a concrete
technical reason.
