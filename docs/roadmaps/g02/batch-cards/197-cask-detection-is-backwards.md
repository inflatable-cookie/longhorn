# 197 Cask Detection Is Backwards

Status: complete — landed 2026-08-13
Owner: Tom
Roadmap: g02.009 batch 3
Governing refs: contract 018; research memo 019
Depends on: Card 159 (the finding)
Auto-start next card: no

## Why

A Homebrew cask install classifies as `SelfManaged`, so Longhorn would offer it
an in-place update and replace a bundle the package manager owns.

`observe_install` reads the application bundle as a symlink and treats the
target as the signal, on the belief recorded in its own comment:

> A Homebrew cask links `/Applications/Thing.app` into its Caskroom.

It is the other way round. Observed on a real machine 2026-08-13:

```
/Applications/LinearMouse.app                              drwxr-xr-x
/opt/homebrew/Caskroom/linearmouse/0.11.2/LinearMouse.app  -> /Applications/LinearMouse.app
```

The cask moves the bundle into `/Applications` and keeps the symlink in the
Caskroom pointing back at it. `fs::read_link` on the bundle fails, no link
target is recorded, and `classify_install` — which is pure and correct given
its observations — has nothing to classify on.

**No headless test could have caught this.** `classify_install` is tested and
right; the observations feeding it are wrong. It took a real cask on a real
filesystem, which is what Card 159 exists for.

## What It Costs

g02.009 names non-writable handling as the one place where "as well or better
than the plugin" has concrete meaning rather than rhetorical: the plugin had no
typed error for it, and Longhorn does. That advantage is currently unrealised
for the most common externally managed install on macOS.

`InstallProvenance::ExternallyManaged` is also what stops `evaluate` reaching
an offer at all, so the fault reaches the operator as a working install button
on an application Longhorn must not replace.

## The Decision This Card Carries

How a bundle proves it is externally managed, when the evidence lives outside
the bundle. Three candidates, and the card should pick one rather than
accumulate all three:

- **Reverse lookup.** Scan `/opt/homebrew/Caskroom/*/*/` for a symlink whose
  target is this bundle. Direct, and it reads a directory Longhorn does not own
  on every classification.
- **Homebrew's receipts.** `Caskroom/<token>/.metadata/` records what a cask
  installed. More authoritative, more coupled to Homebrew's layout.
- **Host declaration.** The application says how it was installed, because a
  packaged build usually knows. Cheapest and most honest, and it moves the
  problem to every consumer.

Whatever is chosen must not assume Homebrew's prefix. Apple silicon uses
`/opt/homebrew`, Intel uses `/usr/local`, and a `HOMEBREW_PREFIX` override is
supported.

## Steps

- [x] Reproduce in a test, with a fixture laid out the way Homebrew actually
      lays one out — bundle in place, symlink in the Caskroom pointing at it.
      The existing tests encode the inverted belief and will need correcting
      rather than extending.
- [x] Fix the detection by the chosen route.
- [x] Keep the old direction working if it ever occurs. A symlinked bundle is
      still evidence of something external, and removing that branch trades one
      false negative for another.
- [x] Re-run `packaged-update-proof` and see the claim turn true.

## Acceptance

- [x] `effigy qa` passes.
- [x] A real Homebrew cask on a real machine classifies as
      `ExternallyManaged { HomebrewCask }`, proved by
      `packaged-update-proof`'s claim rather than by a fixture alone.
- [x] The prefix is not hard-coded to `/opt/homebrew`.
- [x] A self-managed install in `/Applications` still classifies as
      `SelfManaged`. The fix must not make everything look external.

## Evidence

- [x] The corrected fixture test, and what its predecessor asserted.
- [x] The proof's claim flipping from false to true, with the finding entry
      gone from its output.

## Stop Conditions

- Stop if detection cannot be made reliable without scanning a directory
  Longhorn does not own on every classification. Host declaration is then the
  answer, and the cost moves to consumers deliberately rather than by default.

## Continuation

Card 159's remaining update claims: relaunch and tauri#11392, and the interlock
against a genuinely open transfer session.

## Outcome — 2026-08-13

Fixed in the observation, not the classification. `classify_install` always
asked the right question — is a Caskroom path associated with this bundle —
and only the direction of the evidence was missing. `observe_install` now looks
for a Caskroom entry that resolves *to* the bundle when the bundle is not
itself a link, and both directions set the same field through a builder named
for what was observed.

**Route chosen: reverse lookup, targeted.** Of the three candidates, host
declaration moves the cost to every consumer for a problem Longhorn can answer,
and Homebrew's receipts need the cask token, which is not derivable from a
bundle name. The lookup only reads entries whose filename already matches, and
only when the bundle is not itself a link, so an install already explained by
the first shape costs nothing and a self-managed one costs a `read_dir` per
prefix.

`HOMEBREW_PREFIX`, then `/opt/homebrew`, then `/usr/local`. Hard-coding one
would have fixed the machine it was written on and no Intel Mac.

**The old test asserted the wrong belief in its own comment** — "the shape
Homebrew actually creates". Corrected rather than deleted: that shape does
occur, and removing the branch would trade one false negative for another. Two
tests added — the real direction, and a bundle with no Caskroom entry staying
self-managed, so the fix cannot make everything look external.

Proved on a real machine rather than a fixture alone.
`packaged-update-proof` now reports
`aRealCaskInstallClassifiesAsExternallyManaged: true` against
`/Applications/LinearMouse.app`, with the findings list empty.

**One correction to the proof itself.** It was observing the `.app` inside the
Caskroom, which is the symlink. A launched application reports the bundle in
`/Applications`, and that is the path `detect_provenance` has to classify, so
the proof resolves the link before observing.
