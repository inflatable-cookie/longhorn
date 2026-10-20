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

Release resolution and download (part 4b): a `ReleaseSource` resolves one
release of a repository, pinned or `latest`, and downloads one of its listed
assets within a caller bound. `HttpReleases` is the GitHub implementation, over
blocking HTTP with rustls. The API token is a `SecretString`, and it rides only
listing calls. The feature `fixture-server` adds a loopback fixture server for
tests; it is off by default.

The crate reads no catalogue, pins no digest and names no vendor. The caller
supplies the artifact shape, the member path, the digest source, the expected
team, and the release query, endpoints, bounds and token. The install
pipeline is part 4c.

Entry names and link targets are checked by `longhorn-archive-path`, shared
with `longhorn-update-install`. Links are refused, backslash names too, and
every bound counts real decompressed bytes.
