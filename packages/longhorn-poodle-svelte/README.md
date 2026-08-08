# @inflatable-cookie/longhorn-poodle-svelte

Reactive Svelte bindings over the Longhorn domains, and the Poodle projections
that render them. Peers on `@inflatable-cookie/longhorn`, `svelte`, and
`@inflatable-cookie/poodle-svelte`; the Poodle peer is optional, so a consumer
that only wants the bindings never installs it.

```sh
effigy check:svelte
effigy test:vitest
```

The root exports the framework-agnostic reactive state — layout, surfaces,
transfer, and surface transfer — as `.svelte.ts` runes modules:

- `@inflatable-cookie/longhorn-poodle-svelte` — lifecycle, client status, scheduler
- `/layout`, `/surfaces`, `/transfer`, `/surface-transfer`

Per-domain subpaths carry the two projection tiers. `/<domain>/svelte` is the
reactive controller for a domain; `/<domain>/poodle` is the Poodle shell over
that controller:

`commands`, `history`, `history-tree`, `notifications`, `operation`, and
`settings` expose both. `config` exposes `/config/poodle` only.

`/poodle` is the layout projection itself — the binding, document projectors,
and the `LayoutTabs`, `LayoutDockRegion`, and `LayoutSplitView` components that
drive Poodle's primitives. `/native-content` projects native content islands
into Svelte.

Poodle is consumed at its package root and nowhere else, and the framework only
through its published specifiers. Both rules are enforced by
`tests/boundary.test.ts`, as is the pinned-pack resolution that keeps the suite
from silently binding to sibling Poodle source.

The Rust sibling for the GPUI host will be `longhorn-poodle-gpui`; the framework
suffix in this package's name is what keeps that from being a rename.
