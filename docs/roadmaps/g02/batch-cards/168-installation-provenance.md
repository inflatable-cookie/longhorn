# 168 Installation Provenance

Status: complete
Completed: 2026-08-09
Owner: Tom
Roadmap: g02.012 follow-up
Governing refs: contract 018
Depends on: Card 162
Auto-start next card: no

## Objective

Detect who owns the installed application before offering to update it, and
refuse to self-update anything a package manager manages.

## The defect this fixes

`DeferralCause::InstallationNotWritable` carries this comment today:

> Homebrew casks and administrator-installed copies land here. The remedy is
> a manual download, not a retry.

Both halves are wrong for a cask.

**A Homebrew cask usually will not land there.** Casks install into
`/Applications`, which is group-writable by admin users on an ordinary Mac.
The writability check passes, the application self-updates, and `brew` still
believes it is on the previous version. The next `brew upgrade --cask` fights
the change or reverts it, and `brew list --versions` has been lying in the
meantime.

**A manual download is the wrong remedy anyway.** It deepens the desync. The
correct instruction is `brew upgrade --cask <token>`.

So the current model detects a *symptom* (cannot write) too late, and cannot
distinguish three situations that need three different messages: an
admin-installed copy, a package-manager-managed copy, and a distribution
package.

## Scope

- pure `InstallProvenance`, derived from supplied facts rather than probed
- a probe in `longhorn-update-install` that gathers those facts
- `DeferralCause::ExternallyManaged { manager, command }`
- policy: an externally managed installation is never offered a self-update

## Steps

1. Add `InstallProvenance` to `longhorn-update` as a pure classification over
   facts a caller supplies — executable path, bundle path, environment
   variables, symlink target. No filesystem access, because `longhorn-update`
   is pure and that is worth keeping.
2. Add the probe to `longhorn-update-install`, where filesystem access
   already lives.
3. Add `DeferralCause::ExternallyManaged { manager, command }`, carrying the
   command the user should actually run.
4. Make `evaluate` refuse to offer an update for an externally managed
   install, rather than offering one that will be deferred at install time.
5. Narrow `InstallationNotWritable` back to what it says: a self-managed
   install in a location this user cannot write.

## Detection signals

| Provenance | Signal | Confidence |
| --- | --- | --- |
| Mac App Store | `Contents/_MASReceipt/receipt` exists | definitive; self-update is impossible and against policy there |
| Homebrew cask | the `.app` is a symlink into `…/Caskroom/…` | strong; some casks copy instead, so a miss is possible |
| Flatpak | `FLATPAK_ID` set, or `/app` prefix | definitive |
| Snap | `SNAP` set | definitive |
| AppImage | `APPIMAGE` set | definitive |
| Nix | `/nix/store` prefix | definitive |
| Linux distribution | executable under `/usr` | strong |
| Self-managed | none of the above | the only case that self-updates |

**Windows is unproven and should be recorded as such.** winget, Chocolatey
and MSIX identity are all detectable in principle, and none of it has been
tested here. Guessing a provenance wrongly is worse than reporting
`Unknown`: a false "externally managed" blocks a legitimate update, and a
false "self-managed" corrupts a package database.

## Landed — 2026-08-09

`InstallProvenance` and `classify_install` are pure, in `longhorn-update`,
and take an `InstallLocation` of supplied facts. `observe_install` and
`detect_provenance` do the reading, in `longhorn-update-install`, where
filesystem access already lived. The purity held: `longhorn-update` still has
no filesystem, no network, no clock.

**`evaluate` gained a provenance argument and a `ManagedElsewhere`
availability.** Not `UpToDate`, which would be false — there *is* an update —
and not an offer, which would invite the desync. The variant carries the
version and the manager; the surface derives the command from
`InstallManager::upgrade_command`, because only the surface knows the
application's package name.

The check sits **before the mandatory-version floor**. That looked wrong at
first: the floor exists so a security release is never withheld. It is still
not withheld — the user is told where to get it — but Longhorn cannot install
it here whatever the urgency, and offering an install that corrupts a package
database is not a way to make a release more urgent.

`DeferralCause::ExternallyManaged { manager, command }` is not retryable, and
`InstallationNotWritable` no longer claims to cover Homebrew.

### Evidence

- 8 classification tests, one per signal, including the case that motivated
  the card: a Caskroom symlink classifies as externally managed **even though
  the bundle is writable**
- 4 probe tests that build the real shapes on disk — a symlink into a
  Caskroom, a `_MASReceipt/receipt`, a plain copied bundle — and read them
  back through the same calls the probe uses
- 4 policy tests: externally managed is `ManagedElsewhere`, a mandatory
  release still is, `Undetermined` still offers, and the deferral is not
  retryable and names the command
- 16 policy tests total, 43 unit tests in `longhorn-update`

### One judgement worth recording

`/usr` means a distribution owns it, **except `/usr/local`**. That is where a
machine's owner puts things, which is exactly why packaging policy tells
package managers to stay out of it. Getting that backwards would have made
every locally built install stop updating.

## Acceptance Criteria

- [x] an externally managed installation is never offered a self-update
- [x] the deferral names the manager and the exact command to run
- [x] `InstallationNotWritable` no longer claims to cover Homebrew
- [x] detection is pure and testable from supplied facts, with no filesystem
  access in `longhorn-update`
- [x] an unrecognised layout classifies as `Undetermined` and is treated as
  self-managed, because that is the status quo and this card must not make
  ordinary installs stop updating — named `Undetermined` rather than
  `Unknown`, since it says "not checked" rather than "checked, no idea"
- [x] Windows is recorded as unproven rather than guessed

## Evidence Required

- [x] a table-driven test per provenance signal
- [x] the Homebrew case specifically: a symlink into a Caskroom path
  classifies as externally managed even when the target is writable

## Stop Conditions

- detection cannot reach a confident answer for a platform, in which case it
  returns `Unknown` and that platform is recorded rather than guessed

## Notes

Raised by the operator on 2026-08-09 while reviewing the contract 018
amendment: "what if the app was installed with homebrew or a linux package
manager?" The answer is that we would currently break it, quietly.

## Next Task

None. This is a bounded correctness fix.
