# Desktop Support Conformance

A neutral second consumer for Longhorn's desktop support crates. Contract 003
step 6 asks each extracted mechanism for a second consumer's conformance or
adoption; contracts 024 to 028 record their own extraction against Acowtancy
Desktop, and this example is the second consumer that lets step 6 close.

| Unit | Crate | What it proves |
| --- | --- | --- |
| 024 | `longhorn-redact` | The contract's shape rows and every guarantee: idempotence, marker counting, seeded secrets outside markers, the seed floor, `exact_only()` |
| 025 | `longhorn-support-log` | A `RingLog` and a `RotatingLog` bound to a seeded redactor: capacity, eviction, line shape, redact-then-bound with a private-key block cut, the tail bound, rotation at the byte bound, the oversized drop, a JSON line |
| 026 | `longhorn-diagnostics` | `render` over facts that include a `RingRecord` tail: the exact layout, redact-last (a span across a label and its value), CR/LF escaping, empty-section sentences, the marker count |
| 027 | `longhorn-tool-provision` | A live install against the crate's loopback `fixture-server`: resolve, a bounded download, a checksum file, tar.gz extraction, a `CommandRunner` version probe, the swap, the receipt and health, then a pinned upgrade, a current short-circuit and a forced rollback |
| 028 | `longhorn-pty` | A live `/bin/sh` session: output, a stdin round trip, a resize the child reads back, the exit code, and a close that escalates to `SIGKILL` |

`consumer/` is the whole consumer: one binary and one manifest. It is a
template, not a workspace member. `scripts/verify-desktop-support-conformance.ts`
copies the six crates and this consumer into a disposable private workspace,
writes a manifest naming only those crates, and builds the consumer there, so
every assertion goes through a public API as an outside crate. The consumer
inherits workspace dependencies because that private workspace supplies them.

Run it:

```sh
effigy proof:desktop-support-conformance
```

Inputs to change with care:

- A contract's promised behaviour. The consumer states what each contract
  promises in its own words, so a contract change fails here until the consumer
  follows it.
- The crate list in the proof script. It is the staged set; a crate missing from
  it is a unit with no second consumer.
- The consumer manifest's dependency block. The proof fails if it names a crate
  outside the six.

Limits, recorded in `effigy.toml` as the `desktop-support-conformance` group's
proof limits:

- Unix hosts only. The PTY unit starts real children and the install unit needs
  POSIX file modes.
- The fixture server stands in for GitHub. No request leaves the host, and no
  published release, API token or code signature is involved.
- Crates are staged from their packaged sources. Nothing is installed from a
  registry, and no release tag is exercised.
- It runs one row set per unit. It does not repeat the crates' own suites.

Keep the staged workspace with `KEEP_DESKTOP_SUPPORT_CONFORMANCE=1`.
