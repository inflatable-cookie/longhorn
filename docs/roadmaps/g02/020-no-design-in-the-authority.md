# g02.020 No Design In The Authority

Status: ready
Owner: Tom
Governing refs: contract 012; contract 013; contract 020
Depends on: none

## Outcome

`longhorn-poodle-svelte` binds Longhorn authorities to Poodle components. Five
of its thirteen components do more than that: they make layout decisions and
ship CSS to enforce them. This milestone moves the design to Poodle, keeps the
binding here, and adds the check that stops it happening again.

Reported from field use of Soundcheck's settings dialog: "the layout is bad,
the menu just floats, the labels are way too long, scrolling isn't handled
properly, there are close buttons in each settings page."

## The Measurement

Taken 2026-08-12. A component that ships CSS is deciding layout; a binding has
none. The split is clean and it is not "components in Longhorn" generally.

**Eight bindings, zero CSS between them.** `CommandPaletteBinding` 51,
`NotificationPanel` 66, `NotificationToastHost` 40, `NotificationToastStack`
22, `OperationPanel` 200, `LayoutDockRegion` 146, `LayoutSplitView` 120,
`LayoutTabs` 91. Each wraps a Poodle component and feeds it authority state.
These are correct and stay.

**Five that ship CSS, and all five are the settings family.**

| | Lines | CSS |
| --- | --- | --- |
| `SettingsShell` | 435 | 46 |
| `RestoreSettingsPage` | 529 | 28 |
| `StorageSettingsPage` | 357 | 22 |
| `BackupSettingsPage` | 304 | 30 |
| `KeybindingSettings` | 155 | 26 |

1,780 lines carrying 152 of bespoke CSS.

## Two Faults That Are Not CSS

Worth separating, because moving the styling would not fix either.

**The labels.** `SettingsShell.svelte:51` composes a group label as
`` `${module.label} · ${section.label}` `` whenever more than one module is
registered. Soundcheck has a Storage module containing a Storage & Backups
section, so the operator reads "STORAGE · STORAGE & BACKUPS". The rule is a
composition decision made once and applied to every group.

**The duplicate close.** `SettingsShell.svelte:212` renders a ghost `Close`
button into every page's `PageHeader` actions, while the `Dialog` around it
already renders its own `×`. Two affordances for one action, on every page.

## Scope

The settings family only. The eight bindings are already right and touching
them would be churn.

`KeybindingSettings` and the three config pages keep their **content** here —
they render Longhorn's keymap, storage, backup and restore domains, and that
content is not general-purpose. What moves is the layout: they compose Poodle
primitives and carry no CSS, the standard the eight bindings already meet.

## Execution Plan

- [ ] **Batch 1. Poodle owns the settings shell.** A ground-up redesign, not a
      port: the concept is right and the execution is wrong in almost every
      way. Poodle's card, dispatched with the prompt in the batch log.
- [ ] **Batch 2. Longhorn binds it** (Card 192). `SettingsShell` becomes a
      binding with no CSS, the label rule and the duplicate close go, and the
      four remaining components lose their style blocks.
- [ ] **Batch 3. The rule becomes a check.** No `<style>` block in
      `longhorn-poodle-svelte`. Not before batch 2, or the check fails on the
      five files it exists to prevent.

## Goals

- [ ] No component in `longhorn-poodle-svelte` contains a `<style>` block.
- [ ] The settings dialog scrolls, its navigation sits on a surface, its group
      labels read as one thing, and it has one close.
- [ ] Longhorn ships no general-purpose component. Where a Poodle equivalent
      exists, the Longhorn file is a binding to it.

## Acceptance Criteria

- [ ] `effigy qa` passes.
- [ ] A check fails the build on a `<style>` block in this package.
- [ ] The settings binding is under a hundred lines.

## Explicit Non-goals

- No change to the eight bindings. They already meet the standard.
- No move of settings *content* to Poodle. Poodle must not learn what a
  storage profile is.
- No new Longhorn protocol. The `SettingsSession` the shell reads is unchanged;
  this is a rendering boundary, not an authority one.

## Next Task

Batch 1, in Poodle. Batch 2 cannot start until the shell exists.

## Planning Checkpoint

After batch 1. Whether the four remaining components can drop their CSS
entirely depends on what the redesigned shell gives them to compose, and one
worked example answers that better than a guess.
