Artifact proofs now check what they claim. The bridge artifact audit's
`@inflatable-cookie/longhorn` export and protocol-version checks had been keyed to
a package that is not packed, so they never ran; they now check the packed
`./bridge*` exports and `BRIDGE_PROTOCOL_VERSION` on `@inflatable-cookie/longhorn`.
The peer check that named the never-packed `longhorn-bridge` package is removed.

`effigy proof:artifacts` runs every member after a failure and reports every
failing proof, instead of stopping at the first one.

Every proof that stages a Rust workspace takes its staging root from external
build scratch (`stagingDirectory`), not `os.tmpdir()`. A new static test fails
any such proof that touches `os.tmpdir()`.

The `longhorn-bridge-topology-conformance` QA group runs the bridge artifact
audit (`proof:bridge-topology-artifacts`) as well as the conformance proof, so a
bridge source change no longer passes without the audit.

No public API changed.
