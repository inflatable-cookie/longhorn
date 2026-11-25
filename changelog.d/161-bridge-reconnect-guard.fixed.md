A connection that went offline with `retryForbidden` can no longer reconnect with
a retrying class. `BridgeConnectionMachine::reconnect` (Rust) and
`BridgeConnectionRuntime.reconnect` (TypeScript) refuse it as an invalid
transition and leave the connection offline with its reason intact. A `never`
reconnect still goes offline as before.
