# 159 Update And Licence Packaged Proof

Status: restarted 2026-08-13 by operator decision; scope corrected against the
tree on the same day
Owner: Tom
Roadmap: g02.009 batch 3 / g02.010 batch 3 (shared)
Governing refs: contracts 018 and 019; research memos 019 and 020
Depends on: Cards 151, 152, 155, 156, 196 (complete); Card 158 for the
licence client half
Auto-start next card: no

## Objective

Build one packaged proof application that exercises update install, restart,
and licence activation against real platform behaviour, unblocking the host
halves of Cards 153 and 157.

## Why This Exists

Cards 153 and 157 each stopped at the same boundary, for the same reason:
their remaining work cannot be verified headlessly. Two cards blocked on one
absent thing means the absent thing is a card.

The claims that need a real packaged application:

- macOS in-place bundle replacement, and the relaunch afterwards.
  tauri#11392 reports relaunch failing on macOS, with a close handler
  calling `api.prevent_close()` as a contributing factor — and Longhorn owns
  close handling, so this is ours to get right rather than an upstream
  curiosity.
- The restart interlock refusing an install while a transfer session is
  genuinely open, rather than while a test double says one is.
- A platform credential backend actually storing and retrieving through a
  real keychain, including the locked-keychain path.
- The system browser opening and the loopback listener receiving a real
  redirect.
- A non-writable installation producing the manual-download fallback rather
  than an error.

The repository already proves native-content behaviour this way; this is the
same shape applied to two new boundaries.

## Scope

- one proof application under `examples/`, following the existing
  native-content proof conventions
- update: check, download, quiesce, install, relaunch
- licence: file import, key redemption against a stub endpoint, account flow
  against a stub authorization server
- recorded evidence, not an automated CI gate — these are claims CI cannot
  make

## Steps

1. Scaffold the proof application against the existing example conventions.
2. Compose `longhorn-update` with a static-JSON source served locally, and
   drive a real install of a signed artifact.
3. Prove the restart interlock against a genuinely open transfer session,
   then against a quiescent host.
4. Record the macOS relaunch result explicitly, including whether
   `prevent_close` interferes as tauri#11392 suggests.
5. Compose `longhorn-licence` with a platform credential backend and prove
   store, retrieve, remove, and the locked-keychain path.
6. Drive the RFC 8252 flow end to end against a stub authorization server.
7. Prove the non-writable installation fallback.
8. Record every result as retained evidence; unmet claims are recorded as
   unmet rather than quietly dropped.

## Acceptance Criteria

- an update installs and the application relaunches on macOS
- an install is refused while a transfer session is open and proceeds once
  it closes
- credentials survive a restart through the platform backend
- an account sign-in completes through the system browser
- a non-writable installation falls back rather than erroring
- every unmet claim is recorded as unmet

## Evidence Required

- the proof application and its recorded run
- an explicit finding on tauri#11392 under Longhorn's close handling

## Stop Conditions

- macOS relaunch cannot be made reliable under Longhorn's close handling, in
  which case the finding is the deliverable and the interlock gains a
  documented manual-relaunch path

## Progress

Batch 1 (2026-08-08) lands the headless half: the proof harness under
`examples/update-licence-proof/rust/harness` exercises every pure claim with
no packaged application and no new dependencies — update decision and rollout
evaluation, the restart interlock gate against each deferral cause, licence
signature verification and tamper rejection, both activation sources,
usability windows and the clock guard, and credential slot round-trips. Run:
`cargo run -p longhorn-update-licence-proof`. All 20 headless claims pass and
the evidence record is emitted as JSON.

Two questions had to settle before the packaged half could be built:

1. the `tauri-plugin-updater` pin on the locked Tauri graph (the concrete
   installer that Card 153 originally fronted with a port, since removed), and
2. the keychain crate for the `CredentialStore` seam (the platform backend
   from Card 157).

The installer question was settled by architecture, not dependency:
Longhorn does not implement an installer — Tauri's updater plugin performs
check, download, verification, and bundle replacement. `longhorn-tauri-update`
is pure again (no tauri dependency): `UpdateGate::authorize` decides whether
the application may install and carries the reason when it may not. See
Card 153 for the recorded plugin findings. The platform credential backend
remains an open composition decision (the licence crate's documented
posture): a keyring-backed `CredentialStore` would live in the consumer app,
not in a Longhorn crate.

## Restart — 2026-08-13

Restarted by operator decision. The pause of 2026-08-08 held that the packaged
application was more surface than the evidence was worth. What changed is not
the cost — a signed bundle and a machine to replace it on cost the same — but
what the run would exercise.

**Two thirds of this card's own Progress section describe a division that no
longer exists**, and restarting on it would have built against a fiction:

> "Longhorn does not implement an installer — Tauri's updater plugin performs
> check, download, verification, and bundle replacement. `longhorn-tauri-update`
> is pure again (no tauri dependency)."

Void since 2026-08-12. Longhorn is the update controller for both hosts, there
is no plugin, `longhorn-update-install` performs verification and replacement,
and `longhorn-tauri-update` was recreated on 2026-08-13 with a tauri dependency
and four capabilities. The installer question this card recorded as "settled by
architecture" was settled the other way.

### What already exists, checked rather than assumed

| Claim | Where | Status |
| --- | --- | --- |
| Update decision, rollout, interlock gate, licence verification, activation sources, usability, credential slots | `examples/update-licence-proof` | 20 headless claims pass |
| Real `.app` bundle replacement, tamper rejection, executable bits preserved | `examples/packaged-update-proof` | passes against a real bundle |
| Check → fetch → verify → gate → install as one sequence | `UpdateController`, Card 196 | headless, twelve tests |
| Tauri commands and four capabilities | `longhorn-tauri-update`, Card 190 step 4 | nine tests |

So the packaged half is not a green field: bundle replacement is already proved
against a real application. That was the single hardest claim, and it landed
while this card was paused.

### What is still unmet, and what each needs

1. **macOS relaunch, and tauri#11392 under Longhorn's close handling.**
   `packaged-update-proof` records relaunch as "unmet by design — relaunch is
   the host's". That is a correct division and *not* the finding this card
   requires: the question is whether `prevent_close` interferes, and a host
   that never relaunches cannot answer it.
2. **The interlock against a genuinely open transfer session**, rather than a
   `CountingProbe` that says one is open.
3. **A non-writable installation reaching the manual-download fallback.**
   `evaluate` returns `ManagedElsewhere` before any offer and Card 196 tests it
   headlessly with a fetch adapter that fails if called; what is unproved is a
   real administrator-installed copy classifying that way.
4. **Platform credential persistence through a real keychain**, including the
   locked path.
5. **RFC 8252 through the system browser** with a real loopback listener.

Claims 4 and 5 are the licence half and are blocked differently: the platform
`CredentialStore` backend is still an open composition decision, and the
licence client surface is Card 158, unbuilt. Splitting them is the honest
sequence — the update half can run now, the licence half cannot.

## Operator Decision — 2026-08-08 (superseded)

The packaged proof application is **deprioritized**: it is more surface to
maintain than the evidence it buys is worth right now. The machine-bound
claims (macOS bundle replacement and relaunch, interlock against a genuinely
open session, platform credential persistence, RFC 8252 browser flow,
non-writable classification through the real plugin) are recorded as unmet
and stay recorded as unmet — they are not quietly dropped. The headless
harness remains as the regression layer for the pure flows. Resume this card
when a consumer needs the packaged evidence, at which point the app shell,
the keyring backend, and the stub servers are the remaining code.

The packaged-run claims — macOS bundle replacement and relaunch
(tauri#11392), interlock against a genuinely open transfer session,
credentials surviving a restart, an RFC 8252 sign-in through the system
browser, and the non-writable fallback — are recorded as pending in the
harness output, never as passed.

## Progress — 2026-08-13, update half item 1

Done. `packaged-update-proof` drives the whole controller sequence against a
real application bundle, and passes: eight claims, four of them new.

| Claim | What it establishes |
| --- | --- |
| `aLoopbackManifestYieldsAnOffer` | `StaticJsonSource` composes the request, a host serves it over loopback, `evaluate` offers |
| `theArtifactArrivesOverARealSocket` | A genuine HTTP transfer, not a file copy |
| `workInFlightDefersTheInstallAfterTheTransfer` | The gate refuses *after* the bytes arrive |
| `aQuiescentHostInstallsTheOfferedVersion` | The sequence lands the offered version on disk |

The third is the one worth having. Card 196 put the gate between verify and
install rather than before fetch, on the argument that downloading while the
user has work in flight is harmless and gating early makes them wait for a
transfer that could have happened in the background. That was a design
decision defended in a comment; it is now a claim checked against a real
transfer of a real bundle.

The loopback server is hand-rolled over `TcpListener` — two routes and one
verb, where a server crate would be more surface than the thing it serves.
`EndpointUrl` accepts plain HTTP for loopback and nothing else, which is what
makes it addressable.

**One finding, recorded because the symptom lied.** The listener is
non-blocking so its accept loop can poll a stop flag, and on macOS the accepted
socket inherits that. `write_all` on a large body then returns `WouldBlock`
partway and delivers a truncated artifact — which fails verification, so both
installs reported `SignatureRejected`. A transport fault wearing a security
fault's clothes. It was found by recording the rejection codes in the evidence
rather than reasoning about which claim was false, and the fix carries the
explanation.

## Finding — 2026-08-13: cask detection is backwards (resolved same day, Card 197)

Item 3 was attempted and **the claim is false**, which is the finding.

`observe_install` reads the bundle as a symlink and treats the target as the
signal, on the recorded belief that "a Homebrew cask links
`/Applications/Thing.app` into its Caskroom". Homebrew lays it out the other
way round. Observed on this machine:

```
/Applications/LinearMouse.app                      drwxr-xr-x   (a real directory)
/opt/homebrew/Caskroom/linearmouse/0.11.2/LinearMouse.app -> /Applications/LinearMouse.app
```

The cask moves the bundle into `/Applications` and keeps the symlink in the
Caskroom pointing at it. So `fs::read_link` on the bundle fails, no link target
is recorded, and a cask install classifies as `SelfManaged`.

**The consequence is the one `ManagedElsewhere` exists to prevent.** A Homebrew
install would be offered an in-place update and Longhorn would replace a bundle
the package manager owns, desyncing it. The milestone names non-writable
handling as where "as well or better than the plugin" has concrete meaning;
this is that case, and it is currently wrong.

Carded rather than patched in place, because the fix carried a decision about
how a bundle proves external ownership when the evidence lives outside it.
Card 197 took the targeted reverse lookup and landed the same day; the claim is
now true against `/Applications/LinearMouse.app` and the findings list is
empty.

While it was unmet the proof reported `outcome: pass` with the claim false and
a `findings` entry explaining it. Failing the whole run would have buried four
claims that hold behind one that did not — and the finding, not the failure,
was the deliverable.

## Next Task

The update half's remainder. Card 197 closed the cask detection finding.
1. Relaunch, and the explicit tauri#11392 finding under Longhorn's close
   handling. If relaunch cannot be made reliable, the finding is the
   deliverable and the interlock gains a documented manual-relaunch path —
   this card's own stop condition, unchanged.
2. The interlock against a genuinely open transfer session. The sequence
   above uses a `BusyProbe`, which proves the ordering but not that a real
   session reports itself.
3. ~~Non-writable classification on a real administrator-installed copy.~~
   Done 2026-08-13, via the finding above and Card 197.

The licence half waits on two things that are not this card's: the platform
credential backend decision, and Card 158. Recorded as unmet meanwhile, as the
2026-08-08 decision established — that part of it was right and survives.
