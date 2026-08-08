# Generated Bridge Client And Direct Conformance

Date: 2026-07-30
Card: 051
Roadmap: g01.009

## Result

Generated the bridge v1 TypeScript protocol and Rust-owned golden fixture,
then added the framework-neutral `@inflatable-cookie/longhorn-bridge` package. The package
negotiates exact compatibility before exposing a session and keeps product
payload meaning in injected codecs and routes.

## Generation

`longhorn-bindings` exports negotiation, authority, operation, failure,
ordering, retry/deduplication, and optional job wire types. The generated
fixture contains valid envelopes, deliberate incompatibility cases, and a
Rust-computed semantic trace.

Authority descriptors now use contract camelCase at the wire. Rust tests
prove camelCase output and reject internal Rust field names. Binding checks
fail on any generated protocol or fixture drift.

## Checked Client

- exact negotiation before session exposure
- strict validation of versions, connection states, authority, outcomes,
  retry metadata, ordering cursors, and job correlation
- capability and separate read/write/execution authority checks before route
  execution
- injected typed codecs and domain routes
- no generic product operation registry or unchecked product JSON authority

The root export is framework-neutral and query-safe. Stream support is an
explicit subpath and brings no Tauri, Svelte, Poodle, service, or consumer
runtime into the package root.

## Adapter Conformance

Direct and deterministic JSON-loopback adapters execute through one checked
host router. Their negotiation, query, command, retry, ordering, and job
outcomes match the Rust semantic trace. Loopback encode and decode failures
remain explicit adapter-boundary errors.

The checked stream connection installs its listener before snapshot load,
retains intervening state, rejects stale/gapped/foreign data, disposes late
listener registration, and tears down exactly once.

## Structure

Split the former Card 049 negotiation and contract-test high files into
protocol-focused modules before adding generation. The god-file scan now
reports no high finding in Card 049 or Card 051 code.

## Validation

- `effigy qa`
- `effigy check:bridge-bindings`
- `effigy test:bridge-core`
- `effigy test:bridge-ts`: 11 tests, 80 expectations
- `effigy check:bridge-package`
- `cargo clippy -p longhorn-bridge -p longhorn-bindings --all-targets -- -D warnings`
- `cargo fmt --all --check`
- `git diff --check`

## Next

Card 052 is ready. Add narrow Tauri registered-domain host/client assembly and
prove it through the mock runtime without pulling events into query-only use.
