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

Prefer Effigy for generic operations. If Longhorn needs repo-owned automation,
use TypeScript with Bun. Bash is thin glue only; Python needs a concrete
technical reason.
