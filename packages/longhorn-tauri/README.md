# @inflatable-cookie/longhorn-tauri

The Tauri host edge: `invoke`/`listen` transport plus the per-domain adapters
that bind a Longhorn client to a Tauri command surface. Peers on
`@inflatable-cookie/longhorn` and `@tauri-apps/api`.

```sh
effigy check:ts
effigy test:ts
```

The root is the transport itself, and `/events` its event edge. Everything else
is one domain's host adapter:

- `/bridge`, `/bridge-events` — typed bridge over the Tauri command surface
- `/history`, `/history-tree`, `/native-content`, `/notifications`, `/operation`

Each adapter is a thin port construction over the framework-neutral client in
`@inflatable-cookie/longhorn`. It holds no domain logic, which is what lets a
second host — GPUI — supply the same ports without reimplementing a domain.

Contract 020 governs that boundary. `tests/boundary.test.ts` enforces the half
of it that lives here: the package reaches the framework only through published
specifiers, never by relative path into the sibling.
