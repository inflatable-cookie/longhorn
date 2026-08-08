# @inflatable-cookie/longhorn-transfer

Framework-neutral session, lease, cancellation, and same-document panel
transfer client.

Rust serde types are authoritative. Regenerate and check the committed
protocol and fixture with:

```sh
effigy generate:transfer
effigy check:transfer-bindings
```

The client accepts the structural `@inflatable-cookie/longhorn-core` transport and checked
connection lifetime. Transfer retains its client-epoch freshness rule. It
contains no raw Tauri call, browser global, Svelte store, or Poodle component.

Lease requests contain client-local geometry and current host-issued client
authority. They cannot name a managed window, provide outer bounds, choose a
host lifetime, or publish screen-space lease geometry. Terminal screen points
remain untrusted hints revalidated against fresh host and policy authority.

`LONGHORN_TRANSFER_MIME_TYPE`, `serializeTransferPayload`, and
`parseTransferPayload` define the native drag envelope. Its exact JSON shape
is:

```json
{"protocol_version":1,"session_id":"abababababababababababababababab"}
```

Parsing rejects extra fields, future protocol versions, and anything except a
host-issued 128-bit lowercase hexadecimal session id. Subject, window, layout,
Surface, and product state never enter `DataTransfer`.
