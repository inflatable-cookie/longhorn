# @longhorn/bridge

Framework-neutral checked bridge negotiation, authority, operation, retry,
lifecycle, and serialized-loopback support.

The package root has no event or service runtime. It exposes injected-clock
connection/reconnect and bounded query-retry controllers. Import
`@longhorn/bridge/stream` only for ordered listener-first projections.

Import `@longhorn/bridge/tauri` for checked query-only composition over the
invoke-only `@longhorn/tauri` root. Import
`@longhorn/bridge/tauri-events` only for listener-first Tauri snapshots and
request/job-correlated events. Domain packages still own route names and
payload codecs.

Import `@longhorn/bridge/supervision` only when a consumer composes an
optional service. It accepts an injected port, an opaque credential reference,
and stable coded outcomes. It cannot locate, download, update, stop, or replace
an externally owned host.
