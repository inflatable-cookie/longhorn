# 196 Longhorn Is The Update Controller

Status: ready
Owner: Tom
Roadmap: g02.009 batch 3
Governing refs: contract 018; contracts 004, 012, 017; research memo 019
Depends on: Card 151 (complete); Card 152 (complete); Card 190 (steps 1–3
complete)
Blocks: Card 190 step 4; Card 154
Auto-start next card: no

## Why

The operator decision of 2026-08-12 made Longhorn the update controller for
both hosts. Nothing controls anything. `UpdateSnapshot` is built in one place
in the tree and that place is a test.

Every other piece exists. Cards 151 and 152 built policy and sources; the
contract 018 amendment of 2026-08-09 built verification and install; Card 190
built the protocol. What is missing is the thing that sequences them and holds
the state the protocol projects.

## What Already Exists — Checked, Not Assumed

The first draft of this card was wrong about most of this column, and the error
is worth recording: it proposed building verification and recommended minisign
over raw ed25519 as an open decision. Both had already landed on 2026-08-09.

| | Where | Status |
| --- | --- | --- |
| Channel resolution, semver, rollout, deferral | `longhorn-update` | Card 151 |
| Four source adapters, `UpdateSource` | `longhorn-update` | Card 152 |
| Quiescence probes and `UpdateGate` | `longhorn-update` | Card 153 |
| `InstallProvenance`, `classify_install` | `longhorn-update` | Card 153 |
| **Minisign verification** | `longhorn-update-install` | 2026-08-09 |
| **Atomic bundle replacement, escalation port** | `longhorn-update-install` | 2026-08-09 |
| **Classified `NotWritable`, bounded extraction** | `longhorn-update-install` | 2026-08-09 |
| Protocol, snapshot, progress union, four commands | `longhorn-update` | Card 190 |
| **The download** | — | this card |
| **The controller** | — | this card |

`longhorn-update-install` already records why minisign and where it diverges
from Tauri deliberately — no shell interpolation, classified failures, bounded
extraction, and a reasoned decision *not* to diverge on quarantine. The
milestone's "the burden is on diverging" rule was already applied there.

So the 2026-08-12 decision changed less in the tree than the milestone text
implies. It removed the plugin as a *path*; the plugin's job had already been
taken over on 2026-08-09.

## Scope

`longhorn-update`. No host crate, no Tauri commands, no client surface — those
are Card 190 step 4, Card 159 and Card 154, each blocked on something this card
does not supply.

The crate stays pure. No network, no filesystem, no ambient clock. Longhorn
*drives* the download; the host performs it, as the host already performs a
check for `UpdateSource` and the replacement for `UpdateInstaller`.

## Step 1 — The download is described here and performed there

Card 152 set the shape: an adapter describes an exchange, the host performs it.
The same shape, for bytes rather than a manifest.

- [ ] An `ArtifactFetch` describing one transfer: the URL, and the expected
      length when the manifest gives one.
- [ ] The host reports progress back as counted bytes. The crate turns counts
      into `UpdateProgressProjection`, which is where the `Option<f64>`
      fraction Card 190 built gets its `None`: a source with no content length
      cannot produce one, and a bar that invents a number is worse than one
      that says it does not know.
- [ ] A partial or abandoned transfer leaves no state a later check must reason
      about. Resume is out of scope — say so in the type rather than leaving a
      reader to infer it from an absence.

## Step 2 — The controller

- [ ] One type that sequences check → fetch → verify → gate → install, holds
      the state Card 190's snapshot projects, and answers the four commands.
- [ ] It observes; it does not perform. Every side effect is a host call
      through an existing trait — `UpdateSource`, `ArtifactFetch`,
      `QuiescenceProbe`, `UpdateInstaller`.
- [ ] A stale `expectedRevision` is refused on all four commands. Card 190
      lists this as acceptance and there is nothing to refuse it today.
- [ ] The gate sits between verify and install, not before fetch. Downloading
      while work is in flight is fine; replacing the bundle is not, and gating
      too early makes the user wait for a transfer that could have happened in
      the background.

## Step 3 — Verification becomes unreachable-around, not just promised

Verification exists and is correct. What does not exist is any structure
stopping a *second* installer from skipping it.

`UpdateInstaller::apply` takes `artifact: &[u8]` and `signature: &str` and its
doc comment says an implementation must verify before applying. The conformance
suite has an `Unverifying` case that proves the suite catches one. That is a
test of implementations, not a property of the design — and the controller
about to call this trait is the moment to decide which it should be.

- [ ] The controller verifies before it calls an installer, so the guarantee
      holds for every implementation rather than for the ones that remembered.
- [ ] Whether `UpdateInstaller` should then stop taking a signature at all is
      the question to answer here rather than to leave open. If verification
      moves out, say what an implementation still promises and what the
      conformance suite still tests.
- [ ] `InstallFailure::SignatureRejected` is already terminal. Verification
      failing in the controller produces that same outcome, not a second
      vocabulary for the same event.

## Step 4 — Non-writable costs nothing to discover

`classify_install` and `detect_provenance` both exist. Nothing consults them
before a transfer, because nothing performs a transfer yet.

- [ ] A non-writable installation is detected before the download. Fetching
      eighty megabytes and then saying "you installed this with Homebrew" is
      the plugin's behaviour, not an improvement on it.
- [ ] The offer survives. A non-writable install is told a version exists and
      where to get it; it is not an error state and must not project as one.

## Acceptance

- [ ] `effigy qa` passes.
- [ ] A test asserts an installer cannot be reached with an artifact that
      failed verification — by the shape of the call, not by the installer's
      own diligence.
- [ ] A test asserts a signature valid for a different version is rejected.
      `longhorn-update-install` reads the trusted comment; the controller must
      not lose that when it takes over the ordering.
- [ ] A test asserts a source with no content length projects a `null`
      fraction rather than zero. Card 190 lists this and cannot exercise it.
- [ ] A stale `expectedRevision` is refused on all four commands.
- [ ] A non-writable install reaches the manual-download outcome without
      downloading anything, proved by a fetch adapter that fails if called.
- [ ] The conformance suite still passes, with whatever step 3 removed from it
      accounted for rather than quietly dropped.

## Evidence

- [ ] The tests above, named in the batch log.
- [ ] What the conformance suite lost, and what an implementation still
      promises once the controller verifies.
- [ ] Any place the controller's ordering diverges from Tauri's, with the
      reason in the code — the rule `longhorn-update-install` already follows.

## Stop Conditions

- Stop if the controller needs a clock. Rollout staging and deferral both look
  time-shaped and both were built without one; a controller reaching for
  `SystemTime` has found a modelling gap rather than a missing dependency.
- Stop if sequencing the install correctly needs the host crate that does not
  exist. That is Card 190 step 4, and pulling it in would put two unfinished
  things in one card — the failure Card 190 avoided.
- Stop if moving verification into the controller would make
  `longhorn-update-install`'s own verification dead code rather than a second
  line. Two verifiers that can disagree is worse than either arrangement, and
  which one holds it is a design decision, not a refactor.

## Continuation

Card 190 step 4: the host crate and the Tauri commands, with `check` and
`install` as separate capabilities. Then Card 154, the client surface, rescoped
to `packages/longhorn/src/update/`.
