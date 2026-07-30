# longhorn-bridge

Pure bridge identity, negotiation, connection, capability, authority,
operation, ordering, and retry protocol.

The crate owns no domain payloads, transport implementation, executable
acquisition, renderer state, or consumer policy. Hosts advertise capabilities
and authority as separate facts over an exact-version protocol.

`BridgeConnectionMachine` adds authority-gated readiness, bounded reconnect,
transition receipts, and old-session/authority rejection over injected time
and backoff. Query retry is explicitly bounded. Command replay still requires
a durable idempotency key and finite advertised deduplication.

The optional `supervision` feature exposes only a consumer-injected local or
remote lifecycle port, checked ownership, opaque credential references, and
redacted coded observations. It supplies no executable, endpoint, credential
provider, updater, or production network transport. The `bindings` feature
includes this protocol slice for TypeScript generation.
