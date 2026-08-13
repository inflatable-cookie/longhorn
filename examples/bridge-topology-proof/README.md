# Five-shape Bridge Topology Proof

Five donor-derived, product-neutral declarations exercise the shared bridge
without donor writes or production networking.

| Shape | Required seam | Optional seam |
| --- | --- | --- |
| Split-shell | query-only Tauri invoke | none |
| Jetstream | listener-first snapshot stream | Tauri events |
| Soundcheck | correlated job and cancellation | external local service |
| Nucleus | per-domain capability and authority | external local host |
| Loophole | stable authority across host forms | remote attach |

The proof runs applicable operations through direct, injected Tauri, and
serialized loopback adapters. It also records the no-event/no-service graph,
topology matrix, bounded retry schedules, and authority checks.

Run:

```sh
bun scripts/verify-bridge-topology-conformance.ts
bun scripts/verify-bridge-topology-conformance.ts-artifacts
```

The Tauri and service ports are deterministic injected fixtures. They make no
production transport, discovery, authentication-provider, provisioning, or
update claim.

The artifact proof packs the three TypeScript packages and installs five clean
consumer roots without workspace aliases. It also inventories the three
private Rust crates and checks separate query-only and full-host graphs
offline. Registry-normalized Rust packaging remains a release-lane gate while
the crates are private.
