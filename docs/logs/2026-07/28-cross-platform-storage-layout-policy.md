# Cross-platform Storage Layout Policy

Date: 2026-07-28
State: complete research and planning batch

## Outcome

- audited Loophole, Nucleus, Soundcheck, Bovine, and Jetstream storage paths
- verified Apple, XDG, Windows, and Tauri path semantics
- made canonical app id the default leaf and one stable storage name the
  explicit whole-layout override
- made `platform-native-v1` the default
- defined unified-root and explicit portable alternatives
- mapped config, data, state, cache, log, runtime, and backup roots
- classified databases by data lifecycle rather than extension
- defined a fixed bootstrap locator to avoid profile-selection recursion
- defined journaled locator-last profile transition and read-only legacy
  discovery
- compiled cards 011 and 012

## Default

Every platform uses canonical app id as its default app-specific leaf.
Soundcheck may opt into one stable storage name, `Soundcheck`, for the whole
layout while `audio.infiniteloop.soundcheck` remains canonical identity in
locators, manifests, and receipts. The storage name is not changed by a later
product display-name rename.

Linux uses XDG config, data, state, cache, and runtime bases. Windows uses
LocalAppData and makes no roaming claim.

The profile id is versioned. Library upgrades cannot silently move an
existing profile.

## Alternatives

`unified-app-root-v1` keeps typed children below one native durable root. It
matches Nucleus's useful single-root shape but reports lost native cache and
runtime semantics.

`portable-v1` requires one explicit absolute root. It never guesses the
working directory or executable directory.

## Transition

The selected profile lives in a minimal fixed-native locator, not inside the
root it selects. Profile change previews both layouts, stages and verifies
data, journals beside the locator, commits the locator last, and retains the
source until separate receipt-bound cleanup.

Cache rebuilds by default. Runtime is not migrated. Secrets remain external.
SQLite uses a native adapter from card 010.

## Posture

`strict-ready`

Card 011 is the sole ready lane. Card 009 resumes after it. Card 012 runs after
custom adapter conformance.

## Next

Execute card 011: pure platform storage layout profiles and Tauri path-facts
adapter.
