# `@longhorn/core`

Framework-neutral TypeScript transport and checked snapshot-connection
primitives.

The package owns listener registration, bounded refresh coalescing, failure
reporting, and exact asynchronous teardown. Domain packages inject payload
validation, event interpretation, and freshness comparison.

It imports no Longhorn domain, Tauri, Svelte, Poodle, or browser runtime.
