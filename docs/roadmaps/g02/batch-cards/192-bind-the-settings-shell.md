# 192 Bind The Settings Shell

Status: in progress — step 1 complete 2026-08-12; steps 2 and 3 need Poodle's
redesigned shell (batch 1)
Owner: Tom
Roadmap: g02.020 batch 2
Governing refs: contract 012; contract 013; contract 020
Depends on: g02.020 batch 1
Blocks: batch 3
Auto-start next card: no

## Why

See the milestone. Five components in `longhorn-poodle-svelte` ship 152 lines
of CSS between them, and all five are the settings family. The other eight
components in the package ship none.

## Step 1 — Two faults that are Longhorn's, and are not CSS

Do these first and on their own commit. They are wrong today, they will still
be wrong under a new shell, and neither is Poodle's to fix.

- [x] **Stop composing two labels into one.** `SettingsShell.svelte:51` builds
      a group label as `` `${module.label} · ${section.label}` `` whenever more
      than one module is registered. Soundcheck reads "STORAGE · STORAGE &
      BACKUPS" because its Storage module holds a Storage & Backups section.
      Pass the section label. If a host needs the module named, that is the
      host's label to write, not a rule applied to every group.
- [x] **Delete the per-page close** — *corrected during execution, see below.*
      `SettingsShell.svelte:212` renders a ghost `Close` into every page's
      `PageHeader` actions while the `Dialog` already renders its own.
- [x] Both have a test. The first asserts a group label is exactly the
      section's; the second asserts one close per host.

### Correction — deleting it outright would have been a defect

Only `host === "modal"` renders a `Dialog`, and only that branch passes
`showCloseButton`. The `window` and `panel` hosts render a bare `Surface` with
no close affordance at all, so the page-header button was **their only way
out**. Removing it for every host would have left two of three unclosable.

The screenshot that prompted this milestone is the modal host, which is why
the duplicate was the visible fault. "Remove it from every page" was a
generalisation from one host.

So it is now conditional: the page keeps its close only where the host provides
none. Per-page remains the wrong home — the redesigned shell should carry one
close in its own chrome for every host — and the file says so where the
condition is.

`offers exactly one close per host, whichever host it is` asserts all three.

## Step 2 — The shell becomes a binding

- [ ] Replace the composition with Poodle's shell, feeding it the
      `SettingsSession` this file already reads. No layout decisions, no
      `<style>` block.
- [ ] Keep every behaviour the current file owns that is not layout: the
      close guard (`session.requestClose()` may refuse), the search wiring,
      deep-link routing to an anchor, and the focus restore Poodle's Dialog
      fix depends on.
- [ ] Under a hundred lines. If it will not fit, the shell is missing
      something and that is batch 1's problem, not a reason to keep layout
      here.

## Step 3 — The other four drop their CSS

`KeybindingSettings`, `BackupSettingsPage`, `RestoreSettingsPage`,
`StorageSettingsPage`.

- [ ] Their **content** stays. They render Longhorn's keymap, storage, backup
      and restore domains, and none of that is general-purpose.

These four are a **strip-and-compose job, not a redesign**. Their structure is
sound; their author reached for CSS where a primitive existed. Every line was
read on 2026-08-12 and it is three patterns:

| Pattern | Where | Replacement |
| --- | --- | --- |
| `display: grid; gap: 0.75rem` | 10 selectors, all four pages | `Stack` |
| `repeat(auto-fit, minmax(14rem, 1fr))` | 3 config pages, identical | `Grid columns="repeat(auto-fit, minmax(14rem, 1fr))"` |
| `minmax(0, 1fr) auto auto` | `KeybindingSettings` rows | `Grid columns=…` |
| `minmax(0, 1fr) minmax(12rem, auto)` | `RestoreSettingsPage` domain rows | `Grid columns=…` |
| `minmax(12rem, 1fr) auto` | `StorageSettingsPage` flow | `Grid columns=…` |
| `overflow-wrap: anywhere` | 2 pages, on paths and digests | **nothing — papercut** |

`Grid` takes an arbitrary `columns` string, so every track above is
expressible today with no new Poodle component.

- [ ] Substitute per the table. No judgement call is needed for the first five
      rows.
- [ ] **The one real gap is text wrapping.** `Text` has `clamp` but no wrap or
      break control, and `Code` has none either. Without it a long filesystem
      path or content digest overflows its column or forces the grid wider.
      Papercut it; do not reintroduce the rule locally.
- [ ] Anything else Poodle lacks: papercut rather than writing CSS here. Those
      papercuts are the evidence batch 3 needs.
- [ ] Worth telling the Poodle thread even though it is not a blocker: the
      `repeat(auto-fit, minmax(14rem, 1fr))` detail grid appears identically in
      all three config pages. One idea written three times, and the moment to
      notice whether it wants a name.

## Acceptance

- [ ] `effigy qa` passes.
- [ ] No `<style>` block remains in `longhorn-poodle-svelte`.
- [ ] The settings binding is under a hundred lines.
- [ ] A test asserts a refused close still surfaces its reason. That behaviour
      is the one most likely to be lost in a rewrite, because it only shows on
      a page with unsaved edits.
- [ ] A worked example in the batch log: the Soundcheck dialog, with the two
      step 1 faults gone.

## Evidence

- [ ] The tests above, named in the batch log.
- [ ] The before-and-after CSS count for the package: 152 lines to zero.
- [ ] Any papercut raised against Poodle, with what the page needed.

## Stop Conditions

- Stop if the shell cannot express the close guard. A settings dialog that
  cannot be refused loses unsaved edits, and moving the design is not worth
  that.
- Stop if a page needs more than two papercuts to lose its CSS. That means the
  redesign missed a class of layout these pages depend on, and the answer is
  another round in Poodle rather than a Longhorn workaround. The survey above
  found exactly one gap across all four, so a page hitting two is a signal
  that something changed rather than that the page is unusual.

## Continuation

Batch 3 adds the check: no `<style>` block in this package. It cannot run
before this card, because the five files it exists to prevent are still here.
