# longhorn-notifications

Pure finite authority for retained notification truth. It owns bounded records,
explicit replacement, read state, removal receipts, retention, and newest-first
projections. Renderers own transient toast presentation. Consumers own action
execution, product logs, recovery evidence, persistence, and authorization.

The root crate has no operation dependency. Enable `operation` to project
already-committed terminal operation transitions through a consumer policy.
Publication is idempotent by producer token and cannot change operation state.

The `bindings` feature exposes the exact serde protocol to the checked
generator. Direct and hosted clients use bounded newest-first pages, correlated
mutation results, and non-durable revision hints. Executable action behavior,
toast lifetime, native delivery, and product wording remain outside this crate.

Card 080 adds generated clients, Tauri, Svelte, Poodle, and semantic-action
dispatch.
