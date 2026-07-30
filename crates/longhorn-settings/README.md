# longhorn-settings

Pure Rust settings composition and authority protocol.

The crate owns bounded settings identities, declaration validation, capability
admission, immutable registry generations, canonical registry digests,
authority projections, and load/apply/reset protocol types.

It does not own product settings schemas, persistence, Tauri commands,
renderer code, Svelte, or Poodle components.
