# First Publication — 0.1.0

## Outcome

Longhorn's first release is out. `@inflatable-cookie/longhorn`,
`@inflatable-cookie/longhorn-poodle-svelte`, and
`@inflatable-cookie/longhorn-tauri` are published at `0.1.0` on npm, and
`v0.1.0` is tagged at `7e81daa2`. Rust crates stay `publish = false` and are
taken by the tag.

## Evidence

- All seven release gates passed via
  `effigy release prepare --check-gates --yes --version 0.1.0`:
  `private-candidate`, `advisories`, `rustdoc`, `prototypes`, `workspace`,
  `floor` (MSRV 1.95.0), and `source`.
- Dry run: GitHub Actions run `35081084638` green in 40m44s, exercising the QA
  gate, the Metal toolchain install, `effigy release:gates`, and the pack
  verification. The `packed-tarballs` artifact is retained.
- Published integrity matches the dry-run artifact byte for byte: each
  package's `dist.integrity` equals the sha512 of its packed tarball.
- The first publish was manual, following the Poodle precedent — the three
  packages had no npm settings page until they existed, so the trusted
  publisher was configured after the manual `0.1.0` create. `release.yml` owns
  every release from `0.1.1`.
- The dry run's first dispatch failed on a Rust 1.98 clippy lint
  (`clippy::chunks_exact_to_as_chunks`) that the local 1.97 toolchain did not
  have; `longhorn-transfer` was fixed and the run re-dispatched green.

## Material limits

- Consumer repoint is outstanding: Nucleus, Loophole, Soundcheck, Split-shell,
  and Jetstream still reference `file:` paths. That is cross-repo work.
- Crates.io remains out of scope; Rust crates are taken by git tag.
