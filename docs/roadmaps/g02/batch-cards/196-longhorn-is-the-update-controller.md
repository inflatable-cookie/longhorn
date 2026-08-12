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

The operator decision of 2026-08-12 removed the Tauri updater plugin and made
Longhorn the update controller for both hosts. The milestone records what that
makes Longhorn's problem. No card builds it.

Card 153 is complete against the *old* division and cannot be reopened to hold
this: its acceptance criterion was "signature verification stays entirely
inside the plugin", which the decision voids. Card 190 built the protocol and
stopped at step 4 because there is no host crate. Card 154 is the client
surface. The controller itself has no card, and every other card in the batch
is either finished, blocked on it, or blocked on the packaged proof.

## What The Crate Already Has

Not a green field. Cards 151 and 152 left more standing than the decision
needs replacing.

| | Exists | This card |
| --- | --- | --- |
| Channel resolution, semver, rollout, deferral | ✅ Card 151 | — |
| Four source adapters, `UpdateSource` | ✅ Card 152 | — |
| Quiescence probes and `UpdateGate` | ✅ Card 153 | — |
| `InstallProvenance`, `classify_install` | ✅ Card 153 | — |
| `UpdateInstaller`, `InstallFailure`, conformance suite | ✅ Card 153 | rescoped |
| Protocol, snapshot, progress union, commands | ✅ Card 190 | driven |
| **Artifact verification** | ❌ | step 2 |
| **The download** | ❌ | step 1 |
| **The controller that sequences them** | ❌ | step 3 |

`Artifact.signature` is documented today as "passed through to the installer
unread". That line is the old division in one sentence.

## Scope

`longhorn-update`, and nothing else. No host crate, no Tauri commands, no
client surface. Those are Card 190 step 4, Card 159 and Card 154 respectively,
and each is blocked on something this card does not supply.

The crate stays pure. No network, no filesystem, no ambient clock — the same
rule `longhorn-licence` keeps. Longhorn *drives* the download; the host
performs it, as the host already performs a check for `UpdateSource`.

## Step 1 — The download is described here and performed there

Card 152 set the shape: an adapter describes an exchange, the host performs it.
The same shape, for bytes rather than a manifest.

- [ ] An `ArtifactFetch` describing one transfer: the URL, and how many bytes
      the manifest says to expect when it says.
- [ ] The host reports progress back as counted bytes. The crate turns counts
      into `UpdateProgressProjection`, which is where the `Option<f64>`
      fraction Card 190 built gets its `None`: a source with no content length
      cannot produce one.
- [ ] A partial or abandoned transfer leaves no state a later check has to
      reason about. Resume is out of scope — say so in the type rather than
      leaving a reader to infer it from an absence.

## Step 2 — Longhorn verifies

The one place "as well as the plugin" is not a matter of taste. Card 153 step 6
said never implement, wrap, or bypass it. That is void.

- [ ] Verification happens **in the crate**, before any byte is offered to an
      `UpdateInstaller`. Today the trait's doc comment promises this and each
      implementation must keep the promise; after this card the crate keeps it
      and the trait cannot be handed an unverified artifact at all.
- [ ] **Adopt minisign framing, not raw ed25519.** Tauri's answer, and the
      milestone puts the burden on diverging. It buys the existing signing
      tooling and public-key format, and interoperability matters here in a way
      it does not for licences: an artifact is signed by a release pipeline
      that already exists, where a licence is signed by us. `longhorn-licence`
      keeps raw ed25519; the two are different problems and sharing a format
      would be the coincidence, not the design.
- [ ] The trusted comment is read and checked, not skipped. It is where
      minisign carries the version, and a signature that verifies against the
      wrong release is a downgrade.
- [ ] `InstallFailure::SignatureRejected` already exists and is already
      terminal. Verification failing in the crate must produce that same
      outcome rather than a second vocabulary for the same event.

## Step 3 — The controller

- [ ] One type that sequences check → fetch → verify → gate → install, holds
      the state Card 190's snapshot projects, and answers the four commands.
- [ ] It observes progress; it does not perform work. Every side effect is a
      host call through an existing trait — `UpdateSource`, `ArtifactFetch`,
      `QuiescenceProbe`, `UpdateInstaller`.
- [ ] A stale `expectedRevision` is refused on all four commands, which is
      Card 190's acceptance criterion and has nothing to refuse it today.
- [ ] The gate sits between verify and install, not before fetch. Downloading
      while work is in flight is fine; replacing the bundle is not, and a
      controller that gates too early makes the user wait for a transfer that
      could have happened in the background.

## Step 4 — Non-writable is a first-class outcome

The milestone names this as where "better than the plugin" has a concrete
meaning: the plugin had no typed error for it.

- [ ] A non-writable installation is detected before the download, not after.
      `classify_install` already answers this, and downloading eighty megabytes
      to then say "you installed this with Homebrew" is the plugin's behaviour,
      not an improvement on it.
- [ ] The offer survives. A non-writable install still gets told a version
      exists and where to get it; it is not an error state and must not project
      as one.

## Acceptance

- [ ] `effigy qa` passes.
- [ ] A test asserts an `UpdateInstaller` cannot be reached with an artifact
      that failed verification. Not "is not", but *cannot be* — by the shape of
      the call, the way the trait's doc comment currently only asks.
- [ ] A test asserts a signature valid for a different version is rejected.
- [ ] A test asserts a source with no content length projects a `null`
      fraction rather than zero, which Card 190 lists and cannot exercise.
- [ ] A stale `expectedRevision` is refused on all four commands.
- [ ] A non-writable install reaches the manual-download outcome without
      downloading anything, proved by a fetch adapter that fails the test if
      called.
- [ ] The conformance suite still passes for a host installer, with
      verification moved out of it.

## Evidence

- [ ] The tests above, named in the batch log.
- [ ] What the conformance suite lost, and what an implementation is still
      required to promise once it no longer verifies.
- [ ] Every place Tauri's behaviour was diverged from, with the reason in the
      code as the milestone requires.

## Stop Conditions

- Stop if minisign framing cannot be verified without a dependency that pulls
  in a runtime or a C toolchain. The format is a means; if it costs the crate
  its purity, raw ed25519 with the version in the signed payload is the
  fallback and the operator should hear about the swap rather than find it.
- Stop if the controller needs a clock. Rollout staging and deferral both look
  time-shaped, and both were built without one; a controller that reaches for
  `SystemTime` has found a modelling gap rather than a missing dependency.
- Stop if sequencing the install correctly needs the host crate that does not
  exist. That is Card 190 step 4's problem, and pulling it in here would put
  two unfinished things in one card — the exact failure Card 190 avoided.

## Continuation

Card 190 step 4: the host crate and the Tauri commands, with `check` and
`install` as separate capabilities. Then Card 154, the client surface, rescoped
to `packages/longhorn/src/update/`.
