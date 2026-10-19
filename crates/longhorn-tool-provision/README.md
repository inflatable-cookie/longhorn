# Longhorn Tool Provision

Verified artifacts and bounded extraction for managed command-line tools.
Contract 027 owns its behaviour.

A downloaded artifact passes three steps, each a type the next one needs:

1. `verify_artifact` checks the declared digest (a release checksum file, a
   `.sha256` sidecar, or a pinned SHA-256) against the file and returns a
   `VerifiedArtifact`.
2. `extract_executable` takes that artifact and writes one requested member
   into the caller's staging directory, within `ExtractionLimits`. The result
   is a `StagedExecutable`, which cannot be installed yet.
3. `verify_signature` checks the declared vendor team on the staged
   executable and returns an `InstallableExecutable`. The macOS adapter,
   `CodesignVerifier`, runs `/usr/bin/codesign` with a fixed argument vector.

The crate reads no catalogue, pins no digest and names no vendor. The caller
supplies the artifact shape, the member path, the digest source and the
expected team. Release resolution, download and the install pipeline are
later parts of contract 027.

Entry names and link targets are checked by `longhorn-archive-path`, shared
with `longhorn-update-install`. Links are refused, backslash names too, and
every bound counts real decompressed bytes.
