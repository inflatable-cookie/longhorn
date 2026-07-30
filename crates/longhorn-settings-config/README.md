# longhorn-settings-config

One-domain settings apply units over `longhorn-config`.

The crate binds one sealed settings apply unit to one registered configuration
domain. Consumer adapters retain product projection, intent, validation, reset,
policy, and activation semantics. Longhorn supplies fresh coordinated token
checks, publication, conflict snapshots, and exact receipts.

It does not provide cross-domain atomicity. Broader units require an explicit
consumer transaction authority and a separate authority receipt.

The crate also registers optional storage, backup, and restore renderer pages.
Page admission follows independent base capabilities. These operational pages
do not gain settings apply-unit authority. Active or recovery-required restore
state remains a host gate and projects as settings recovery state.
