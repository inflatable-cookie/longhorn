# 027 Managed Tool Provisioning

Status: active compiled boundary for part 4a; 4b and 4c not yet specified
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
`hex`, `flate2`, `tar` and `zip`. No features, no TypeScript binding, no
Tauri, no network.

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

A digest source is parsed and checked before any extraction. The file's
SHA-256 is compared with the parsed digest. Each source has its own format.

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

`VerifiedArtifact` is the gate. `extract_executable` takes one, so no path
from downloaded bytes to a staged file exists without the declared digest
check.

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
- The mode is set to `0755` after the write. No permission from the archive
  is applied.
- Raw artifacts are copied to the staged name under the same byte bound.

**Bounds count real bytes.** Limits are `ExtractionLimits`, 20 000 members and
4 GiB by default. A tar reader counts every decompressed byte it consumes,
members it skips included. A zip reader counts the bytes of the requested
member only; other members are never decompressed. Declared sizes are never
the bound. The zip reader itself refuses a member that decompresses past its
declared size; that is a damaged archive (`UnreadableArchive`), so a header
cannot hide real bytes. An honest member over the bound is `ArchiveTooLarge`.

**Every entry is checked, not only the member.** A tar or zip is refused
whole if any entry anywhere:

- has a name the path rule refuses, or contains a backslash
  (`UnsafeEntry`);
- is a symbolic or hard link (`LinkEntry`);
- is neither a file nor a directory, such as a device or fifo
  (`UnsupportedEntry`).

The root directory entry (`./`) is skipped. `./` prefixes are normalised, so
`./tool` and `tool` name the same member. The member occurring twice is
`DuplicateMember`. A missing member is `MemberMissing`. Too many entries is
`TooManyMembers`. A file the reader cannot decode is `UnreadableArchive`.

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
  could not be opened, read or written), `Verifier` (codesign could not run).
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
  The type carries the check.

`install.rs`

- `MAX_ARCHIVE_MEMBERS` (20 000) and `MAX_ARCHIVE_BYTES` (4 GiB) become
  `ExtractionLimits::DEFAULT`, with the same values.
- **Tar member copy:** the donor had no byte bound on the copy. The bound now
  counts every decompressed byte.
- **Zip bytes:** the donor summed declared sizes over all entries. It now
  counts real bytes of the member copied. A header that understates its
  member is refused as unreadable.
- **Unsafe names:** the donor skipped an unsafe entry and went on. It now
  refuses the archive. The donor's `safe_member` is the shared rule plus the
  backslash refusal.
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
- The tests use the public API only, with archives built in fresh temporary
  directories. They cover the donor's verification and extraction cases with
  neutral names and digests; checksum and sidecar edge cases; a digest
  mismatch; neither source declared; a team mismatch and an unsigned binary
  through a scripted verifier; the macOS adapter refusing an unsigned file
  (off macOS, the unsupported refusal); tar.gz and zip archives with `..`,
  absolute and backslash names, symlink and hard-link members, too many
  members, skipped bytes counted against the bound, an honest member over
  the bound, a header that understates its member, a missing member, a
  duplicate member, a raw artifact, and the executable mode on the staged
  file.
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

Not yet specified. A `ReleaseSource` trait, GitHub release resolution and a
bounded download over the workspace `reqwest`, with the API token passed in as
a value. Specified and tested in its own task.

### Part 4c: install pipeline

Not yet specified. Staging, the version probe, the receipt, the swap with
rollback, login-shell PATH discovery, and a host-injected executor with no
Tauri. Specified and tested in its own task.

## Stop Conditions

- A product value (a team identifier, a pin, an asset name, a catalogue
  entry) is compiled into the crate.
- A path reaches an installable executable without a declared check passing.
- The bounds stop counting real bytes, or a declared size becomes the bound.
- The path rule diverges between `longhorn-archive-path` and its consumers.
- The crate grows a network client, a shell invocation, or a Tauri type.
