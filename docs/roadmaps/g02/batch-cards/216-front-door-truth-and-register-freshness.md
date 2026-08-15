# 216 Front-door Truth And Register Freshness

Status: complete
Completed: 2026-08-15
Owner: Tom
Roadmap: g02.025 batch 2
Governing refs: contract 001; contract 012; memo 023 (H5, H6, M2, M5,
docs-L1)
Depends on: Card 215 (the registers cite the architecture it fixes)
Auto-start next card: no

## Objective

Counts in front doors are generated or gone, the held-surface register earns
its "single source of truth" claim with a freshness gate, and the generation
index carries one pointer instead of three.

## Why this exists

- `docs/README.md:32-33` and `docs/reference/private-0-1-candidate.md:41-42`
  say "18 TypeScript packages and 41 Rust crates" — reality is 3 and 45.
  Handwritten, wrong twice.
- The held-surface register is stale on 3 of 7 rows: update (`:31` — renderer
  surface exists since g02-154), licensing (`:32` — TS surface exists since
  g02-158), replay (`:35` — `apply_with_replay` has zero callers or tests).
  `verify-held-surface.ts` enforces existence, not freshness.
- `docs/roadmaps/generation-index.md` has three competing next-step pointers
  (`:105`, `:121-122`, `:136-137`) and asserts both "publication is no longer
  deferred" (`:106-107`) and "publication remains deferred" (`:119`).
- Publication-status drift: contract 012 (`:73,173,218`) and the
  compatibility guide (`:19-20`) assert deferral and `private: true`; no
  publishable package is private, all carry `publishConfig.access: public`.
- Contract-index statuses understate 015/016 (`:47-48`) and describe closed
  milestones in present tense (`:117-129`); header predates Card 179 while
  citing it (`:46`).
- Contract 020 asserts the drag ceiling both proved (`:264`, `:399-413`) and
  unproven (`:266-270`, `:587-590`).
- The card127 receipt (`fixtures/release/card127/private-0-1-candidate-v1.json`)
  misdescribes its frozen commit (duplicated entries, missing layout crates,
  post-rename package names); the hash chain locks the error in.

## Scope

- the four front doors and two registers named above
- `scripts/verify-held-surface.ts` — the freshness gate
- contract 012 and the compatibility guide — the publication amendment
- `docs/reference/private-0-1-candidate.md` — the receipt annotation

## Steps

1. Counts: generate them (the way `api-surface.md` already is) or delete them
   from prose. No handwritten census survives this card.
2. Correct the three held-surface rows from the code. Extend
   `verify-held-surface.ts`: a row whose named trigger card/milestone is
   closed in the roadmap while the row still reads "awaiting" fails.
3. Generation index: one `## Next Task` field, updated in the same commit
   that closes a card — write that rule into the file header and apply it.
   Resolve the publication self-contradiction per step 4.
4. Publication amendment: contract 012 and the compatibility guide describe
   the post-scope-claim reality (scope claimed 2026-08-09, packages
   publishable, tag blocked on Poodle v0.2.0 per operator).
5. Contract-index statuses brought current; 020's ceiling stated once.
6. Annotate the card127 receipt discrepancy in `private-0-1-candidate.md` —
  the receipt stays frozen; the annotation carries the truth.

## Do Not

- Regenerate or edit the frozen receipt. Annotation, not revision.
- Fix the *symptom* counts while leaving them handwritten — that is the third
  time this exact drift gets fixed by hand.

## Result

- **Counts**: `docs/README.md` no longer carries handwritten package/crate
  counts — it points at the generated `api-surface.md`, which is always
  current. The candidate reference's "live tree" sentence does the same.
- **Register**: the update and licensing rows graduated out of the register
  entirely (g02.009/g02.010 shipped crates + renderer + host + proof), the
  replay row tells the truth again (tested since the Card 211 port; no
  production caller), and `verify-held-surface.ts` now fails any row whose
  named milestone or card is closed. Proven: it caught both stale rows on
  its first run.
- **Generation index**: one live pointer, with the one-pointer rule written
  into the section. The running narrative is gone; closeout lives on the
  cards. The publication self-contradiction left with it.
- **Contract index**: 015/016 read "active compiled boundary"; the roadmap
  paragraph describes the current runway, not present-tense closed work.
- **Contract 020**: the drag ceiling is stated once, as closed, and the live
  ceiling (teardown with a flush in flight, Card 176) is what both spots now
  name.
- **Publication amendment**: contract 012's adoption section and the
  compatibility guide now say scoped-and-queued — scope claimed 2026-08-09,
  publishable packages, tag waiting on Poodle v0.2.0. Card-evidence
  paragraphs from g01 keep their historical wording.
- **Receipt annotated**: the card127 candidate reference now records the
  receipt's misdescription of its frozen commit (duplicated/missing entries,
  post-rename names) — the hash chain stays untouched.

## Acceptance Criteria

- [x] no handwritten package/crate count remains in a front door
- [x] the freshness gate fails on today's stale rows and passes after the fix
  (both halves observed)
- [x] the generation index has exactly one live pointer
- [x] no doc asserts publication deferral to a completed milestone

## Evidence Required

- the gate's failing-then-passing run
- the annotation text
- the diffs

## Stop Conditions

Stop if the publication amendment needs the Poodle v0.2.0 timeline stated
formally — that timeline is the operator's, and the card records what is
given rather than inventing dates.
