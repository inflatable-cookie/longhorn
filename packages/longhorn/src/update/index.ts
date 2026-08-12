// The protocol only. The update client -- a controller, a port implementation
// and its validation -- is Card 154, and this root exists ahead of it so the
// Tauri seam can be typed rather than reaching for `unknown`.
export * from "./generated/protocol.ts";
// The generated field maps are public API. A consumer that mirrors these types
// structurally has no compile-time link to them, so a field added here is
// silent on their side; whoever sees both can assert against these and fail
// when it drifts.
export * from "./generated/fields.ts";
export * from "./generated/variant-fields.ts";
