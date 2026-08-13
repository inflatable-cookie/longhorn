# 197 Cask Detection Is Backwards

Status: ready
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

- [ ] Reproduce in a test, with a fixture laid out the way Homebrew actually
      lays one out — bundle in place, symlink in the Caskroom pointing at it.
      The existing tests encode the inverted belief and will need correcting
      rather than extending.
- [ ] Fix the detection by the chosen route.
- [ ] Keep the old direction working if it ever occurs. A symlinked bundle is
      still evidence of something external, and removing that branch trades one
      false negative for another.
- [ ] Re-run `packaged-update-proof` and see the claim turn true.

## Acceptance

- [ ] `effigy qa` passes.
- [ ] A real Homebrew cask on a real machine classifies as
      `ExternallyManaged { HomebrewCask }`, proved by
      `packaged-update-proof`'s claim rather than by a fixture alone.
- [ ] The prefix is not hard-coded to `/opt/homebrew`.
- [ ] A self-managed install in `/Applications` still classifies as
      `SelfManaged`. The fix must not make everything look external.

## Evidence

- [ ] The corrected fixture test, and what its predecessor asserted.
- [ ] The proof's claim flipping from false to true, with the finding entry
      gone from its output.

## Stop Conditions

- Stop if detection cannot be made reliable without scanning a directory
  Longhorn does not own on every classification. Host declaration is then the
  answer, and the cost moves to consumers deliberately rather than by default.

## Continuation

Card 159's remaining update claims: relaunch and tauri#11392, and the interlock
against a genuinely open transfer session.
