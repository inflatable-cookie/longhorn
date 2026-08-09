# 162 Native Update Execution

Status: complete
Completed: 2026-08-09
Owner: Tom
Roadmap: g02.012 batch 2
Governing refs: contracts 018 and 020; research memo 021
Depends on: none
Auto-start next card: no

## Objective

Implement update execution for hosts with no updater plugin — download,
signature verification, unpack, install, relaunch — under the same
behavioural contract the Tauri plugin path satisfies.

## Rationale

Contract 018 was amended on 2026-08-08. Authorization-only was correct while
Tauri was the only host; it became wrong when GPUI became first-class,
because no plugin exists there. The alternative is every product
implementing minisign verification and macOS bundle replacement itself:
security-sensitive code, duplicated per application, which is what a
framework exists to prevent.

## Scope

- native installer for hosts with no plugin — which turned out to mean every
  host, see the 2026-08-09 decision below
- one shared conformance suite both paths satisfy
- the system browser opener contract 019's RFC 8252 flow needs

## Steps

1. Write the behavioural contract first, as a conformance suite, before
   either implementation is touched. Two implementations without a shared
   suite is a fork, not an adapter.
2. Run the suite against the Tauri plugin path to establish the baseline.
   Anything the plugin does that the suite does not capture is either
   contract or accident, and this is where that gets decided.
3. Implement the native path: download, verify, unpack, replace, relaunch.
   Verification is not optional and has no configuration — an installer that
   does not verify is not an installer.
4. Reuse `longhorn-update`'s policy unchanged. This card adds execution, not
   decisions.
5. Keep the restart interlock host-agnostic. `UpdateGate::authorize` already
   answers for both paths.
6. Add the system browser opener as a host capability.

## Steps 1 Landed — 2026-08-08

The contract and its conformance suite exist before either implementation,
as the card required.

`UpdateInstaller::apply(version, artifact, signature) -> Result<Applied,
InstallFailure>` in `longhorn-update`, with `run_conformance` returning one
outcome per claim.

**The contract is deliberately coarse.** Tauri's plugin performs download,
verification, and replacement as one opaque call, so a contract demanding
separable steps could not be satisfied by it. What both implementations can
promise is observable behaviour: what reaches disk, and what is reported.

**The suite reports every failure rather than the first**, so a report names
everything an implementation gets wrong instead of inviting fix-and-rerun-blind.

**Seven meta-tests prove the suite catches what it claims.** A suite nobody
has seen fail is a suite nobody should trust, so each feeds it a
deliberately wrong implementation and asserts the specific claim goes red:
an installer that skips verification, one that rejects for the wrong reason,
one that conflates an unusable artifact with a signature failure, and one
that applies the wrong version.

Two distinctions the suite enforces that a naive implementation loses:

- **Rejecting is not enough.** A signature failure reported as a generic
  fault invites a retry loop against an attacker-supplied artifact, so
  `SignatureRejected` is not retryable and the suite checks the reason, not
  just the refusal.
- **"We do not trust this" and "we trust it and cannot use it" are different
  messages**, and only one is a security event. `MalformedArtifact` is
  separate from `SignatureRejected`.

### Format decision — minisign, from the Tauri side

Tauri's plugin verifies with **minisign**, via `minisign-verify ^0.2`.

The native implementation must use the same format and the same key.
Anything else means two signatures per artifact, two keys, and two signing
steps per release — for one product shipping to two hosts. One release, one
signature, two verifiers that agree by construction.

This is a constraint on the native implementation, not a choice it gets to
make.

## Step 3 Landed — 2026-08-08

`longhorn-update-native` verifies with minisign, unpacks, and replaces
atomically. It passes the shared conformance suite.

Tauri's macOS install path was read as a **specification** for platform
behaviour — both projects are MIT, so reading is fine and copying would need
attribution; reimplementing from understanding is the honest use. What it
told us: a gzip tar whose single top-level entry is the application, extract
to temp, move the current install aside, move the new one in.

### Three deliberate divergences

1. **No shell interpolation.** Tauri escalates by building
   `rm -rf '{src}' && mv -f '{new}' '{src}'` and running it through
   AppleScript with administrator privileges. A path containing a quote
   breaks out of that string. Escalation here is an injected port, so
   Longhorn never constructs a privileged shell command.
2. **Classified failures.** Tauri returns generic IO errors. The contract
   needs `NotWritable` distinguishable from a transient fault, because one
   needs a manual download and the other can retry.
3. **Bounded extraction.** Entries are checked before they are written. A
   signature proves origin, not good intent, so an archive stays untrusted
   input after it verifies.

### One deliberate non-divergence

Tauri does not strip `com.apple.quarantine`, and it is right not to. The
attribute is applied by applications that opt into it, not by an ordinary
file write, so extracted files never carry it. Adding handling would be
cargo-culting a problem we do not have.

### Evidence

Eight tests, keys and archives generated in-test so nothing can drift from
the format it claims to accept. Beyond the conformance suite:

- a tampered artifact leaves the installed application **untouched** —
  refusing is necessary, but refusing without having already disturbed the
  install is what makes it safe
- a **signed** archive escaping the destination is refused, with the header
  name written directly because `tar::Builder` will not create such a path.
  Only a hostile producer emits one, which is the point.
- escalation is never attempted on a writable target
- the default escalation declines rather than prompting for a password the
  application never asked to need
- a failed install leaves no staging directory behind
- relaunch is left to the host, preserving macOS's separation

Two bugs the tests caught: `tar`'s `unpack` does not create parent
directories, and a nested file can precede its directory in the archive.

## Step 6 Landed — 2026-08-09

`longhorn-browser` supplies the system browser opener. Host-agnostic: neither
backend has one, so both compose the same crate.

It is two independent defences, because the capability hands a
server-influenced string to an OS launcher:

- **`BrowserUrl` is an allowlist**, not a denylist. HTTPS with a host, ASCII,
  no control characters, no whitespace, no embedded credentials, bounded to
  2048 bytes. `file://`, `javascript:` and custom schemes fail for not being
  https rather than for being individually recognised, which is what stops the
  list going stale.
- **`NativeSystemBrowser` never uses a shell.** It spawns `/usr/bin/open` or
  `xdg-open` with the URL as a single argument. The usual one-line version of
  this capability interpolates a URL into `sh -c` and is a remote code
  execution path.

Neither defence relies on the other. A test asserts the launcher program name
contains neither `sh` nor `cmd`, so relaxing it is a deliberate act.

Windows is `UnsupportedPlatform`, recorded rather than guessed — the platform
launchers there parse their arguments in ways this has not been tested
against, and a launcher that might reach a command interpreter is worse than
one that says it is missing. `DecliningSystemBrowser` is the honest default
for a product with no account flow: declining is a policy, doing nothing
silently is a bug that resembles one.

10 tests.

**Not wired into `longhorn-licence`.** `AccountFlow` yields the verifier,
state and redirect URI but does not assemble the authorization URL, so each
consumer concatenates it — which is exactly where escaping goes wrong. Giving
`AccountFlow` an `authorization_url()` returning a `BrowserUrl` would close
that, but it would make a pure policy crate depend on one that spawns
processes. The clean answer is the `longhorn-update` / `longhorn-update-native`
split applied here too. Left for a decision rather than taken mid-card.

## Step 2 Answered — 2026-08-09, negatively

**The Tauri plugin cannot satisfy `UpdateInstaller` as specified.** Not
"needs a packaged application to prove" — cannot, in either of the two ways
it could be wired. Read from `tauri-plugin-updater` 2.10.1.

`verify_signature` is called in exactly one place: `updater.rs:712`, at the
end of `Update::download`. `Update::install(bytes)` calls `install_inner`
directly, and the macOS `install_inner` extracts the gzip tar and replaces
the bundle with **no verification at any point**.

So verification is welded to the downloader, and an adapter has two choices:

1. **Let the plugin download.** Then it verifies, but the contract's
   `apply(version, artifact, signature)` is the wrong shape — the caller
   holds the bytes and the plugin insists on fetching its own.
2. **Hand it the caller's bytes** via `Update::install`. Then the shape fits
   and nothing is verified, which violates the contract's one absolute rule:
   "there is no configuration, host, or build profile under which an
   unverified artifact may reach disk."

There is no third option. `Update`'s fields are private and only `check()`
constructs one, so an adapter cannot fabricate an `Update` around bytes it
already has, and cannot reach the plugin's verifier without a network round
trip to an endpoint.

### What this means for contract 018

If a Longhorn adapter over the plugin must verify with Longhorn's own
minisign verifier before calling `install`, then the plugin contributes
exactly one thing: macOS bundle replacement. `longhorn-update-native` already
does that, and Step 3 recorded three deliberate ways it does it *more*
safely — no shell interpolation, classified failures, bounded extraction.

Which makes the plugin path strictly worse than the native path on the only
platform either targets today, and makes contract 018's "Tauri hosts use the
updater plugin, non-Tauri hosts use Longhorn's native implementation" a
distinction with nothing behind it. **Update execution looks host-independent,
not host-dependent.**

That reverses an operator decision recorded in memo 021 on 2026-08-08, so it
is stated here rather than taken. The plugin may still be wanted for Windows
NSIS/MSI handling, which Longhorn's native installer does not cover — that is
the strongest remaining argument for it and it is not a macOS argument.

### The suite did its job

Nothing above was found by running the suite, because the suite cannot be run
against an implementation that cannot be written. But the suite is why the
gap is legible: it demanded verify-before-apply as a claim, and that is
precisely the claim the plugin's public surface cannot honour for
caller-supplied bytes.

## macOS Bundle Replacement Proved — 2026-08-09

Against a real `Soundcheck.app`, built with `cargo tauri build --debug
--bundles app`. `examples/packaged-update-proof` takes any `.app`, copies it
twice — one standing in for the installed application, one bumped to the next
version and signed — and runs the native installer against the copy. The real
build is never at risk.

```json
{"outcome":"pass","bundle":"Soundcheck.app",
 "claims":{"aTamperedArtifactIsRejected":true,
           "aTamperedArtifactLeavesTheInstallUntouched":true,
           "aVerifiedArtifactReplacesTheBundle":true,
           "executableBitsSurviveTheRoundTrip":true},
 "versions":{"installed":"0.1.0","applied":"0.1.1","requested":"0.1.1"}}
```

The fourth claim is the one the crate's own tests cannot make. They archive
flat files at mode `0644` under a fake `Example.app`; a real bundle carries
executable bits, and an installer that dropped them would produce an
application that replaces cleanly and will not start. Both of Soundcheck's
binaries came out the far side still executable.

Relaunch is **not** claimed, by design rather than by omission. macOS
separates replacement from relaunch and `longhorn-update-native` keeps that
separation, so relaunch belongs to the host and contract 018 says so.

Soundcheck was chosen over a fresh proof application because it is a real
product already composing five `longhorn-tauri-*` crates, so the bundle under
test is one a user would actually receive. It needed no changes — the proof
consumes a build output. Operator authorised work in that repository on
2026-08-09; none turned out to be necessary.

## Decision — 2026-08-09: execution is host-independent

Operator decision. Contract 018 is amended: Longhorn owns update execution on
every host, and `longhorn-update-native` is the single implementation.

The reasoning is the direction of the guarantee. The host with no plugin has
to work solidly no matter what, and an implementation that satisfies GPUI
necessarily satisfies Tauri, because nothing in it is host-specific. Building
for the weaker host and letting the stronger inherit is the only ordering
that leaves neither under-served. The reverse — build for the plugin, bolt on
a fallback — leaves the fallback under-tested, which is exactly where an
unverified artifact would eventually get through.

**Tauri's updater stays the specification, not the mechanism.** Its artifact
shape — a gzip tar whose single top-level entry is the application — is what
Longhorn matches, so one signed release still serves both hosts. Longhorn
diverges only where copying would be unsafe: no shell interpolation,
classified failures, bounded extraction.

Windows NSIS and MSI remain unimplemented, and the plugin is the obvious
donor specification when they are wanted — on the same terms.

### What changed in code

Nothing structural. `longhorn-update-native` already existed and is now
proved against a real bundle. Five doc comments asserted the plugin story and
were corrected, one of which — `longhorn-tauri-update`'s "installation is the
Tauri updater plugin's job" — had become actively false.

## Acceptance Criteria

- [x] ~~both paths satisfy one conformance suite~~ — **superseded**. There is
  one path, and it satisfies the suite. The criterion was written when two
  were expected; the plugin cannot satisfy the suite at all, which is the
  finding rather than a shortfall.
- [x] an unverified artifact is refused on every host, under every
  configuration
- [x] policy is untouched: no channel, rollout, or floor logic is duplicated
- [x] the non-writable-installation case is classified, not surfaced as
  failure
- [x] macOS bundle replacement proved against a real packaged application;
  relaunch recorded as the host's, by design

## Evidence Required

- [x] the conformance suite, and the native path passing it
- [x] a tamper test refused by the native path — and, on a real bundle, shown
  to leave the installation untouched
- [x] the recorded baseline of plugin behaviour: it cannot implement the
  contract, with the source citations

## Stop Conditions

- the native path cannot satisfy a claim the plugin satisfies, in which case
  the divergence is stated in contract 020 rather than hidden

## Next Task

Close g02.012. Cards 161, 162 and 163 are complete.
