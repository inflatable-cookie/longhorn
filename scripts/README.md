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

Prefer Effigy for generic operations. If Longhorn needs repo-owned automation,
use TypeScript with Bun. Bash is thin glue only; Python needs a concrete
technical reason.
