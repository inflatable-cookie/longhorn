# Longhorn 0.2.0 release → Oracle notification

Status: open — pending the 0.2.0 release
Owner: Longhorn Chatterbox
Next check: when the operator publishes Longhorn 0.2.0

## Obligation

The acowtancy Desktop lane (`g05.179`, contract 038) is retained and blocked on
the **released** Longhorn `0.2.0` artifact, not on planning. Upstream task merge
alone does not trigger Desktop adoption. When the operator-owned `0.2.0` release
is published, notify Oracle `ead33773-22cb-4f81-aac7-1c7ac1ad8528` with the
identities Desktop needs for staging:

- the immutable tag (`v0.2.0`) and the commit it dereferences to;
- npm versions **and** integrity for `@inflatable-cookie/longhorn`,
  `@inflatable-cookie/longhorn-poodle-svelte`, and
  `@inflatable-cookie/longhorn-tauri`;
- the frozen Rust identity: the git tag/commit Desktop pins the `longhorn-*`
  crates to.

## Context

Contract 018 was amended 2026-09-22 with a staged update protocol and an
exclusive admission lease; `g02.041` and `g02.042` implement it, and `0.2.0` is
the release that carries it. The release is operator-owned — no silent
publication. `g02.041` is queued/working; `g02.042` waits on it.

## Disposition

Keep open until `0.2.0` is published and Oracle is notified; then delete this
note in the same commit that records the release.
