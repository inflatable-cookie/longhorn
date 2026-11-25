Artifact proofs now check what they claim. The bridge artifact audit's
`@inflatable-cookie/longhorn` export and protocol-version checks had been keyed to
a package that is not packed, so they never ran; they now check the packed
`./bridge*` exports and `BRIDGE_PROTOCOL_VERSION` on `@inflatable-cookie/longhorn`.
The peer check that named the never-packed `longhorn-bridge` package is removed.
