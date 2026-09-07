# Longhorn Agent Tool Dispatch

Opt-in, transport-neutral validation and callback dispatch for production
contextual agent tools under contract 023.

The crate owns no tool registry, schema, admission issuer, transport, listener,
lease, correlation kernel, credential resolver, provider, or daemon. A host
normalizes its already admitted registration and binding into the public types,
trusted-clock deadline state, and positive limits, then supplies one callback.
The recording dispatcher rejects mismatched, expired, or terminal work before
callback invocation and exposes only redacted evidence.

Contract 022's developer control tools are not a dependency or feature of this
crate.
