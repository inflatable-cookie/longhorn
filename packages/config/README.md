# @longhorn/config

Checked renderer client for host-owned storage diagnostics, profile
transitions, backup, restore, and recovery operations.

The renderer can select a built-in profile, confirm a host-issued digest, and
select an already inventoried archive by digest. It never receives filesystem
capabilities, executable plans, archive payloads, or encryption identities.
Portable roots and export destinations come from injected host pickers.
Restore archive paths, unlock material, payload bytes, executable plans, and
journals also stay in host authority. Renderer restore commands carry an
inventory digest or host-picker request, explicit per-domain choices, and
host-issued confirmation digests.

Import `@longhorn/config` for the framework-neutral client. The generated
`@longhorn/config/protocol` declarations remain Rust-owned checked artifacts.
Import `@longhorn/config/poodle` for the optional public-Poodle storage,
backup, and restore pages.
