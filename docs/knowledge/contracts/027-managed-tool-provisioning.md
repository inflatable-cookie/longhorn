# 027 Managed Tool Provisioning

Status: active compiled boundary for parts 4a and 4b; 4c not yet specified
Owner: Longhorn maintainers
Created: 2026-10-19
Depends on: contracts 001, 003, 012, 018 and 024; the shared path rule in
`longhorn-archive-path`
Affects: `longhorn-tool-provision`, `longhorn-archive-path`,
`longhorn-update-install` (path rule only), desktop tool consumers

## Problem

A desktop app that manages command-line tools (Effigy, the GitHub CLI, Codex,
Claude Code) downloads a vendor artifact and must not run what it did not
verify. Acowtancy Desktop does this in `managed_tools/verify.rs` and
`managed_tools/install.rs`, against Acowtancy contract 042. The verification
and the extraction are generic. The catalogue, the pins and the release names
are not. Every desktop app that installs tools needs one tested copy.

Source analysis: unit 4 of the Praxis map
[`shared-tooling-extraction.md`](https://github.com/inflatable-cookie/praxis/blob/dbad7b093f0452800c4df3a96c0798ebf788b08f/docs/knowledge/architecture/shared-tooling-extraction.md)
at `dbad7b0`, read from `acowtancy/acowtancy-v2` at `1d2d13022`. Donor for
this part: `acowtancy/acowtancy-v2` at `3c21b1ac8`,
`apps/bovine-desktop/src-tauri/src/managed_tools/{verify,install}.rs`. From
this contract on, this file owns the behaviour; the map and the donor are
evidence.

Tom split unit 4 into three tasks on 2026-10-19: 4a verification and bounded
extraction (this contract, now); 4b release source and download; 4c the install
pipeline. One crate, `longhorn-tool-provision`, carries all three.

## Generic versus product

Longhorn owns the mechanism: digest formats, the verification types, the
signature port and its macOS adapter, bounded single-member extraction, the
failure classes, and the shared path rule.

The caller owns every value: the artifact shape and member path (already
expanded), the asset name, the digest source text, the pinned digests, the
expected vendor team, and the staging directory. The catalogue entries,
version pins, vendor team identifiers, release and asset names, the
`expand_template` of a member name against a version, and Acowtancy contract
042 stay in Acowtancy.

## Public API

Crate `longhorn-tool-provision`: depends on `longhorn-archive-path`, `sha2`,
`hex`, `flate2`, `tar` and `zip`, and for part 4b on `reqwest` (blocking,
rustls), `secrecy`, `semver`, `serde` and `serde_json`. One feature,
`fixture-server`, is test support and is off by default. No TypeScript
binding and no Tauri. The network is used only by the release source
(part 4b, below).

Verification:

- `DigestSource<'a>`: `ReleaseChecksums(&str)`, `Sidecar(&str)`,
  `Pinned(&str)`.
- `DigestOrigin`: `ReleaseChecksums`, `Sidecar`, `Pinned`; named in a digest
  refusal.
- `Declaration<'a> { digest: Option<DigestSource>, vendor_team: Option<&str> }`.
- `verify_artifact(&Path, asset, Declaration) -> Result<VerifiedArtifact, ProvisionError>`.
- `VerifiedArtifact`: no public accessor. Only `verify_artifact` makes one.
- `digest_from_checksums`, `digest_from_sidecar`, `sha256_file`: public so
  the formats are testable on their own.

Extraction:

- `ArtifactShape<'a>`: `Raw`, `TarGz { member }`, `Zip { member }`.
- `ExtractionLimits`: `DEFAULT` (20 000 members, 4 GiB of decompressed
  bytes); `new(max_members, max_bytes)`; read-only accessors.
- `extract_executable(&VerifiedArtifact, ArtifactShape, staging, staged_name, ExtractionLimits) -> Result<StagedExecutable, ProvisionError>`.
- `StagedExecutable`: no public accessor. It is not installable.

Signature:

- `SignatureReport`: `Team(String)`, `Unverified { detail }`.
- `SignatureVerifier` (trait): `report(&self, &Path) -> Result<SignatureReport, ProvisionError>`.
- `CodesignVerifier`: the production adapter. Scripted verifiers implement the
  trait in tests.
- `verify_signature(StagedExecutable, &dyn SignatureVerifier) -> Result<InstallableExecutable, ProvisionError>`.
- `InstallableExecutable`: `path()` and `vendor_team()`. The only value that
  counts as installable. Its constructor is `verify_signature`.

Failures (contract 027's classes; see Failure classes):

- `ProvisionError::Refused(Refusal)` and `ProvisionError::Fault(Fault)`.

Crate `longhorn-archive-path` (contract 018 and this contract): `bounded`
and `PathRefusal`. The one entry-name and link-target rule.

## Verification

A digest source, when declared, is parsed and checked before extraction. The
file's SHA-256 is compared with the parsed digest. Each source has its own format.

- **Release checksum file.** Lines are `<digest>  <name>`, with whitespace
  between, and an optional `*` before the name. The digest may be upper or
  lower case and is returned lowercase. Blank lines are skipped. The asset
  must appear exactly once. Absent: `ChecksumNotCovered`. Present but not
  SHA-256 hex, or present twice: `MalformedChecksum`.
- **Sidecar.** Exactly `<64 lowercase hex>  <asset>\n`, with one line and one
  trailing newline. Uppercase hex, another name, an extra line or a missing
  newline: `MalformedSidecar`.
- **Pin.** A caller-supplied lowercase SHA-256. Anything else: `MalformedPin`.

A digest mismatch is `DigestMismatch` with the origin. Neither a digest nor a
vendor team declared is `NothingDeclared`, returned before the file is read. A
declared team with no digest is accepted here; the team is checked after
extraction.

`VerifiedArtifact` records a valid declaration and any digest check.
`extract_executable` takes one and rechecks the digest on its private copy
when declared. A team-only artifact can be staged without a digest check;
it cannot become an `InstallableExecutable` until its declared signature
passes. `verify_artifact` does not vouch for the file afterwards.

**The check that counts is on the copy extraction reads.** Before anything
is read, extraction copies the artifact into a private file in the staging
directory and hashes that copy as it writes it. A declared digest is compared
with that hash. A mismatch is `DigestMismatch` with the origin; extraction
attempts to remove the private copy. With a declared digest, changed source
bytes therefore cannot reach the executable. The private copy is removed
before `extract_executable` returns unless cleanup fails with `Fault::Io`
(see Extraction).

## Signature check

A declared `vendor_team` is checked on the staged executable, never on the
archive: Apple signs Mach-O binaries, not archives. The check is the port
`SignatureVerifier`.

- Team matches: `InstallableExecutable`.
- Another team: `TeamMismatch { found, expected }`.
- The signature does not verify, or names no team: `Unsigned { expected, detail }`.
- No team declared: the executable passes through. Its digest was the
  declared check, and the verifier is not consulted.
- The verifier cannot run: `Fault::Verifier`, not a judgment.

Every failed signature check removes the staged file, including adapter
refusals and verifier faults. Successful checks keep it for installation.
If cleanup fails, the result is `Fault::Io` with the staged path, the cleanup
failure and the original signature error in its detail; the caller must
resolve that host fault. A file already removed counts as cleaned up.

The macOS adapter (`CodesignVerifier`) runs `/usr/bin/codesign --verify
--strict --verbose=1 <path>`, then `codesign -dv <path>` for the
`TeamIdentifier` line. Both use a fixed path and an argument vector, never a
shell. The environment is `PATH=/usr/bin:/bin`. `not set` and `unknown` are
no team. On other platforms the adapter refuses with
`SignatureUnsupported`, so a declared team fails closed.

## Extraction

`extract_executable` writes one file: the requested member, at
`staging/<staged_name>`. The staged name is one file name chosen by the
caller. A name with a directory part is `Fault::Usage`.

- The file is created new. An existing file is an I/O fault and is not
  overwritten.
- A refusal or fault after the file was written triggers removal: a later
  entry can refuse an archive whose member was already written. A file that was at
  the path before the call is never touched. Cleanup ownership starts only
  after atomic file creation succeeds; a losing concurrent call cannot remove
  the winner's file.
- The mode is set to `0755` after the write. No permission from the archive
  is applied.
- Raw artifacts are copied to the staged name under the same byte bound.

**Cleanup failures are host faults.** Removal is attempted for every owned
file, even if extraction or an earlier cleanup already failed. A failed
removal returns `Fault::Io` with the failed path and the original error in
its detail. If both removals fail, the final fault retains both paths and
the original error. This also covers a copy or digest failure while sealing
the artifact. A file already removed counts as cleaned up. A failed removal
may leave that file behind; the caller must resolve the reported host fault.

**Bounds count real bytes.** Limits are `ExtractionLimits`, 20 000 members and
4 GiB by default. A tar reader counts every decompressed byte it consumes,
members it skips included. A zip reader reads the requested member's stored
stream without the zip library's decompression; a deflated member is decoded
here and every byte it produces is counted. Entries that are not extracted
are never decompressed. The header's uncompressed size is never read, so a
header that understates its member cannot hide real bytes: a member that
decompresses past the bound is `ArchiveTooLarge` whatever it declares.

Zip members are `Stored` or `Deflated`. Any other compression method, and any
encrypted member, is `UnreadableArchive`, wherever it sits in the archive. The zip extraction does not check
the CRC-32 in the header: integrity is the declared digest's job, and for a
team-only declaration the signature check on the staged executable is.

**Every entry is checked, not only the member.** A tar or zip is refused
whole if any entry anywhere:

- has a name the path rule refuses, or contains a backslash
  (`UnsafeEntry`);
- is a symbolic or hard link (`LinkEntry`);
- is neither a file nor a directory, such as a device or fifo
  (`UnsupportedEntry`). For zip the file type is the Unix mode's `S_IFMT`
  bits, not the name: a fifo, character or block device, or socket with a
  file name is refused, and a regular file whose name ends in `/` is a file.
  A zip without Unix attributes is a file;
- for zip, is encrypted or uses a compression method other than `Stored` or
  `Deflated` (`UnreadableArchive`).

The root directory entry (`./`) writes nothing, but it is still checked. `./` prefixes are normalised, so
`./tool` and `tool` name the same member. The member occurring twice is
`DuplicateMember`. A missing member is `MemberMissing`. Too many entries is
`TooManyMembers`. A file the reader cannot decode is `UnreadableArchive`.
A host failure while reading the private copy is `Fault::Io` with the copy's
path, never a refusal: the archive is not at fault.

**Path rule.** The name and target rule is `longhorn-archive-path::bounded`:
`..`, a root or a drive prefix refuses with `Escapes`; nothing left refuses
with `Empty`. It is shared with `longhorn-update-install` (contract 018),
whose behaviour is unchanged. The backslash refusal is this contract's own,
added on top: update-install accepts backslashes on Unix, and must keep doing
so.

## Failure classes

`ProvisionError` separates the verdict from the host:

- **Refused.** The artifact or executable is bad. The same bytes refuse again:
  `NothingDeclared`, `ChecksumNotCovered`, `MalformedChecksum`,
  `MalformedSidecar`, `MalformedPin`, `DigestMismatch`, `UnreadableArchive`,
  `TooManyMembers`, `ArchiveTooLarge`, `UnsafeEntry`, `LinkEntry`,
  `UnsupportedEntry`, `DuplicateMember`, `MemberMissing`, `TeamMismatch`,
  `Unsigned`, `SignatureUnsupported`.
- **Fault.** The host could not finish. A retry may succeed: `Io` (a file
  could not be opened, read, written or removed), `Verifier` (codesign could
  not run).
  `Usage` is a caller bug: a staged name or member this crate cannot use.

Callers match the enums. The messages are for people.

## Compatibility for consumers

Against the Acowtancy donor at `3c21b1ac8`:

`verify.rs`

- `digest_from_checksums`, `digest_from_sidecar`, `sha256_file`: same
  formats, same lowercasing and same sidecar rule. Two deltas: a checksum
  file that lists the asset twice is refused as ambiguous (the donor took the
  first); the errors are typed refusals and faults, not strings.
- `DigestSource`: same three variants. `DigestOrigin` names the origin on a
  refusal.
- `verify_artifact(path, asset, digest, codesign_team)` returned `()` and
  checked the team separately. Now it takes a `Declaration` and returns a
  `VerifiedArtifact`. The neither-declared refusal is unchanged, and now runs
  before the file is read.
- `SignatureVerifier`: `Result<Option<String>, String>` becomes
  `SignatureReport` with typed errors.
- `CodesignVerifier`: same commands, arguments and environment. Off macOS it
  refuses where the donor's always ran `/usr/bin/codesign`.
- `verify_signature(binary, team, verifier) -> Result<(), String>` becomes
  `verify_signature(StagedExecutable, verifier) -> Result<InstallableExecutable, _>`.
  The type carries the check. A failed signature check removes the staged
  executable; a cleanup failure is an I/O fault.

`install.rs`

- `MAX_ARCHIVE_MEMBERS` (20 000) and `MAX_ARCHIVE_BYTES` (4 GiB) become
  `ExtractionLimits::DEFAULT`, with the same values.
- **Tar member copy:** the donor had no byte bound on the copy. The bound now
  counts every decompressed byte.
- **Zip bytes:** the donor summed declared sizes over all entries. It now
  counts the real decompressed bytes of the member copied and never reads the
  declared size. A header that understates its member is refused only if the
  real bytes pass the bound.
- **Zip special files:** the donor's zip path did not check the Unix type. Fifos,
  devices and sockets are refused.
- **Zip entry type:** the donor took a directory from the name's trailing
  slash. The Unix mode now decides it. Encrypted and unsupported-compression
  entries are still refused archive-wide, now by a check of each entry's
  header instead of the zip library's open.
- **Digest timing:** the donor checked the digest before extraction on the
  source file. Here the digest is checked on the copy extraction reads (see
  Verification), so the source can change after verification without effect.
- **Unsafe names:** the donor skipped an unsafe entry and went on. It now
  refuses the archive. The donor's `safe_member` refused an empty name, a
  leading `/`, a backslash, and any name that contains `..` as a substring.
  The name rule here is the shared component rule plus the backslash refusal.
  It refuses an empty name, a root or drive prefix, a `..` component and a
  backslash. A name with `..` inside a component, such as `tool..backup`, is
  now accepted. The donor's substring refusal is deliberately not kept: the
  shared rule is also `longhorn-update-install`'s, whose behaviour must not
  change.
- **Links:** the donor ignored link entries. It now refuses the archive.
- **Duplicates:** the donor took the first match. It now refuses.
- **Member comparison:** the donor compared the raw name. It now compares the
  normalised name (`./tool` matches `tool`).
- **Staged name:** the donor used the member's file name. The caller now
  names the staged file.
- **Mode:** the donor set `0755` after unpacking. It is still set after the
  write, and no archive permission is applied.
- `ArtifactShape` takes a borrowed member, already expanded. The donor's
  `expand_template` stays in Acowtancy.

`longhorn-update-install`

- `bounded` now delegates to `longhorn-archive-path::bounded`. Its public API
  and its refusal text are unchanged. Its tests pass unchanged.

## Required Evidence

- `effigy check:tool-provision` (QA group `tool-provision`): Clippy, the
  tests, rustdoc with warnings denied, and the API-reference inventory.
- The integration tests use the public API only, with archives built in fresh temporary
  directories. They cover the donor's verification and extraction cases with
  neutral names and digests; checksum and sidecar edge cases; a digest
  mismatch; neither source declared; a team mismatch and an unsigned binary
  through a scripted verifier; failed signature checks removing the staged
  file (mismatch, unsigned, unsupported adapter and verifier fault), cleanup
  failure reported as an I/O fault; the macOS adapter refusing an unsigned file
  (off macOS, the unsupported refusal); tar.gz and zip archives with `..`,
  absolute and backslash names (tar and zip), symlink and hard-link members,
  too many members, skipped bytes counted against the bound, an honest member
  over the bound, a zip header that understates its member and whose real
  bytes pass the bound (`ArchiveTooLarge`), zip fifo, device and socket
  entries, a zip regular file named with a trailing slash, an encrypted or
  unsupported-compression zip entry that is not the member or is the `./`
  root (`UnreadableArchive`), a source changed after verification
  (`DigestMismatch`, nothing left in staging), a missing member, a duplicate
  member, a raw artifact, and
  the executable mode on the staged file. The mode tests are Unix-only.
- Part 4b (`tests/release.rs`, feature `fixture-server`): the donor's resolution
  tests with neutral names (latest without paging, a pinned tag without latest,
  a missing pin, a tag mismatch, the token on listing calls only, the call
  budget); draft and pre-release refusal on both routes; a floor; a tag that
  names no version; a malformed listing; a tag that is not plain; a rate limit
  with and without a reset; a declared length over the bound; a stream over
  the bound without a declared length (chunked); a truncated body; a partial
  file removed after each failure; an existing file not overwritten; a
  listing redirect to the same origin (token kept) and to another origin
  (refused, nothing requested there); a download redirect (no token); the
  token absent from every error and `Debug` output; a non-HTTPS endpoint
  refused outside `loopback_for_tests`; a name with a separator refused before
  any request; zero bounds refused; and a silent source as an unreachable
  fault at the timeout.
- Cleanup-boundary unit tests force removal errors with fixture-local
  directories: staged-file cleanup, private-copy cleanup after a refusal or
  successful extraction, both removals failing, and a sealing copy fault.
  They check the I/O class, failed paths, retained original failure and that
  the other owned file still gets a cleanup attempt. Missing files preserve
  the original result.
- Read-fault unit tests drive the tar.gz and zip readers over a source that
  fails mid-read and expect `Fault::Io` with the copy's path; undecodable
  bytes from a healthy source still refuse as `UnreadableArchive`. The public
  API reads only its own private copy, so it cannot inject this fault.
- `longhorn-update-install`'s tests pass unchanged against the shared rule.

## Extraction state

Contract 003 steps 1 to 4 are done for part 4a: the donor is named, its tests
are ported with neutral names, this contract is the generic definition, and
`longhorn-tool-provision` is the smallest shared package. Steps 5 to 7 are
Acowtancy's migration, after a Longhorn release it can tag.

Open:

- Step 6: second-consumer conformance has no named consumer yet. Candidate:
  a greenfield tool-provision proof.
- Acowtancy contract 042 remains the donor's behaviour until its own
  migration task.

### Part 4b: release source and download

Specified below in [Part 4b](#part-4b-release-source-and-download). Contract
003 steps 1 to 4 are done for this part: the donor is named, its tests are
ported with neutral names, this section is the generic definition, and the
crate carries it. Steps 5 to 7 are Acowtancy's migration, after a Longhorn
release it can tag.

### Part 4c: install pipeline

Not yet specified. Staging, the version probe, the receipt, the swap with
rollback, login-shell PATH discovery, and a host-injected executor with no
Tauri. Specified and tested in its own task.

## Part 4b: release source and download

Donor: `acowtancy/acowtancy-v2` at `75e425296`,
`apps/bovine-desktop/src-tauri/src/managed_tools/releases.rs` (its tests
included). From this section on, this contract owns the behaviour.

### Generic versus product

The crate takes every value: the owner, the repository, an optional pinned
tag, an optional version floor, the endpoints, the user agent, the bounds and
an optional API token. The caller owns the tool catalogue (owners, repositories,
pins, floors, asset names), the token broker and keychain access, the user-agent
string, and Acowtancy contract 042. The crate names no tool, owner or
repository, and its tests use neutral ones.

### API

- `ReleaseSource` (trait): `resolve_release(&ReleaseQuery) -> Result<Release, ReleaseError>`
  and `download_asset(&Release, asset, destination_directory) -> Result<PathBuf, ReleaseError>`.
- `ReleaseQuery { owner, repository, pinned_tag, version_floor }`, all borrowed,
  public fields.
- `Release`: `owner()`, `repository()`, `tag()`, `version() -> Option<&Version>`,
  `asset_names()`, `has_asset()`. Only `resolve_release` makes one.
- `ReleaseEndpoints::github()`, `ReleaseEndpoints::new(api, download)`
  (`https` only), and with feature `fixture-server`,
  `ReleaseEndpoints::loopback_for_tests(api, download)` (`http` or `https`,
  loopback hosts only).
- `ReleaseOptions::new(user_agent)`, with `with_timeout` and
  `with_max_artifact_bytes`. Defaults: 60 seconds and 2 GiB
  (`DEFAULT_TIMEOUT`, `DEFAULT_MAX_ARTIFACT_BYTES`).
- `HttpReleases::new(endpoints, options, Option<SecretString>) -> Result<HttpReleases, ReleaseError>`:
  the one production source, blocking, over rustls.
- `version_from_tag(&str) -> Option<Version>`.
- Feature `fixture-server` (test support, off by default, no extra
  dependency): `FixtureServer` and `FixtureRoute`. The server binds
  `127.0.0.1` on an OS-assigned port, answers one request at a time, and
  records each request's path and headers.

### Resolution

- A pinned query asks `GET {api}/repos/{owner}/{repository}/releases/tags/{tag}`.
  An unpinned query asks `GET {api}/repos/{owner}/{repository}/releases/latest`.
  One listing request per resolve. Nothing pages.
- `404`: `NoCompatibleRelease`, naming the pin when there is one.
- A draft or a pre-release, on either route: `NoCompatibleRelease`.
- A pinned tag must equal the listing's `tag_name` exactly. A different
  answer, even a newer one, is `TagMismatch`. It is never installed.
- A listed tag that is not a plain name (below) is `Malformed`.
- The version is `version_from_tag`: `v1.2.3`, `1.2.3` and `rust-v1.2.3` name
  1.2.3. Any other tag names no version.
- With a floor, a release with no version or a version below the floor is
  `NoCompatibleRelease{floor}`. Without a floor, a release with no version
  resolves, and `version()` is `None`.
- Names are plain when they are ASCII letters, digits, `.`, `_`, `-` or `+`,
  and not `.` or `..`. The owner, repository, pinned tag and asset name must
  each be plain. A name that is not plain is a usage fault, before any request.

### Rate limit

A `403` or `429` with `X-RateLimit-Remaining: 0` is `RateLimited`, with
`reset_unix` from `X-RateLimit-Reset` when it parses. The rate-limit answer is
never paged around or retried inside one call. Any other `403` or `429` is an
unreachable fault.

### Download

- An asset must be listed by the release, or it is `MissingAsset` and nothing
  is requested.
- The request is `GET {download}/{owner}/{repository}/releases/download/{tag}/{asset}`
  on the download client, which carries no token.
- A declared `Content-Length` over the bound is `ArtifactTooLarge`, refused
  before any file is created.
- The file is created new in the destination directory. A file already there
  is an I/O fault. Nothing is overwritten.
- Bytes are counted as they arrive, through `take(bound + 1)`. A stream past
  the bound is `ArtifactTooLarge`, whatever it declared. The bound counts the
  bytes the server sent: no gzip or other decoding is enabled.
- Any failure after the file is created removes it. A failed removal is an
  I/O fault that keeps the original failure in its detail. A read failure or a
  truncated body is an unreachable fault.
- The timeout bounds the whole request, the body included. A bound raised for
  a large artifact needs a timeout that can finish it too.

### Tokens

- The token is a `secrecy::SecretString`, passed in by the caller.
- It is sent only as `Authorization: Bearer` on listing requests, to the
  configured API origin. It never rides a download, and a redirect that leaves
  the origin is refused before the token is sent.
- It never appears in an error, a `Debug` output or a log. The crate logs
  nothing. Tests check the display and debug text of the refusals and faults
  a token-bearing query produces (a rate limit, a malformed listing, a missing
  release, a cross-origin redirect, a missing asset, an oversized download),
  and of the source and release values.

### Redirects and TLS

- Production endpoints are `https` only. `ReleaseEndpoints::new` refuses any
  other scheme as a usage fault. Plain `http` is allowed only through
  `loopback_for_tests`, on `127.0.0.1` or `localhost`.
- Listing redirects: followed only to the same origin (scheme, host and port),
  up to 10 hops. A cross-origin redirect is `UnsafeRedirect`, and the target is
  never requested.
- Download redirects: followed to any `https` target (or loopback `http` in a
  test endpoint), up to 10 hops. A non-HTTPS target is `UnsafeRedirect`.
- TLS is rustls with the aws-lc-rs provider, as the donor ships it. Enabled
  reqwest features: `blocking` and `rustls`. Default features are off, so
  `http2`, `charset` and `system-proxy` are not enabled, and no gzip decoding
  runs. The donor's feature set was the same, plus `json`, which this crate
  does not need.
  The crate's normal dependencies add no `tokio`. The blocking client runs on
  its own runtime inside reqwest, as the donor's did.
- Feature unification: Cargo unifies reqwest's features across one build. A
  build that enables this crate and the agent-control carrier's `client` feature
  therefore gives the carrier's client a TLS back end. The carrier's boundary
  is kept in code instead: `run_carrier` refuses any endpoint that is not plain
  `http://127.0.0.1:`, so the carrier cannot speak TLS whatever the feature set.
  `test:agent-control-carrier-guard` runs that refusal.

### Failure classes

`ReleaseError` splits the verdict from the host, as `ProvisionError` does:

- **Refused** (`ReleaseRefusal`): the same answer refuses again.
  `NoCompatibleRelease`, `TagMismatch`, `Malformed`, `MissingAsset`,
  `UnsafeRedirect`, `ArtifactTooLarge`.
- **Fault** (`ReleaseFault`): a retry may succeed. `Unreachable` (transport,
  timeout, a failing status, a truncated body), `RateLimited`, `Io` (a file
  could not be created, written or removed), `Client` (the HTTP client could
  not be built), `Usage` (a caller bug: a non-HTTPS endpoint, a name that is
  not plain, a zero bound or timeout, or an empty user agent).

Callers match the enums. The messages are for people.

### Compatibility for consumers

Against the donor at `75e425296`:

- `ReleaseSource::resolve_release(&ToolSpec)` becomes `resolve_release(&ReleaseQuery)`.
  The catalogue's repo, pin and floor move to the query's `owner`,
  `repository`, `pinned_tag` and `version_floor`. The donor's `spec.pinned_tag()`
  is `v{pinned version}`. The caller supplies the tag string now.
- `download_asset(&ToolSpec, &Release, asset, dir)` becomes
  `download_asset(&Release, asset, dir)`. The release carries the owner and
  repository it came from.
- `Release { tag, version, asset_names }`: crate-private fields become accessors.
  `version` is `Option`, so a tag that names no version resolves unless a floor
  refuses it. The donor refused every release with no version.
- `ReleaseFailure` becomes `ReleaseError::{Refused, Fault}`. `NoCompatibleRelease`,
  `MissingAsset` and `RateLimited` keep their shapes. `RateLimited` loses the
  donor's local-time text: the reset is unix seconds, and the caller formats it.
- `Unreachable(String)` stays a fault, now `Unreachable { detail }`. The donor
  also folded an unreadable body into `Malformed`. Here a body that cannot be
  read is `Unreachable`, and a body that is not release JSON is `Malformed`.
- A pinned tag: the donor compared the listing's version with the pin's. Here
  the listing's `tag_name` must equal the pinned tag exactly, and a different
  answer is `TagMismatch`, a refusal rather than `Malformed`.
- A pinned draft or pre-release names the pin in `NoCompatibleRelease`. The
  donor named none.
- Download: the donor created the file with truncation, overwriting any file
  there. Here it is created new, and an existing file is an I/O fault.
- Overflow: the donor refused a declared length over the bound as
  `Unreachable`, and removed the file on a real-bytes overflow. Here both are
  `ArtifactTooLarge`, and any failure after creation removes the partial file.
- Redirects: the donor used reqwest's default policy (any scheme, ten hops). The
  listing now stays on its origin, and a download must be `https`.
- User agent and timeout: the donor fixed the user agent and the 60-second
  timeout. Both are caller values here. The defaults match the donor.
- Token: the donor read the token from Acowtancy's broker in
  `HttpReleases::github()`. The broker read stays in Acowtancy, which passes the
  token in as a `SecretString`. The donor's `Zeroizing<String>` is replaced.
- Asset names: the donor passed any name to the URL. Here a name that is not
  plain (the names above) is a usage fault. Acowtancy's catalogue asset names
  must satisfy the rule, or the consumer change refuses them.
- `HttpReleases::github()` (broker-backed) and the donor's `FixtureReleases`
  stay in Acowtancy. The crate has `FixtureServer` for the same end-to-end use.

## Stop Conditions

- A product value (a team identifier, a pin, an asset name, a catalogue
  entry) is compiled into the crate.
- A path reaches an installable executable without a declared check passing.
- The bounds stop counting real bytes, or a declared size becomes the bound.
- The path rule diverges between `longhorn-archive-path` and its consumers.
- The crate grows a network call outside the release source, a shell
  invocation, or a Tauri type. Part 4b adds `HttpReleases` as the one network
  call site.
