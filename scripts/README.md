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

The default donor is `../nucleus`. Set `NUCLEUS_REPO` to an exact clean
checkout when unrelated Nucleus work is in progress.

Prefer Effigy for generic operations. If Longhorn needs repo-owned automation,
use TypeScript with Bun. Bash is thin glue only; Python needs a concrete
technical reason.
