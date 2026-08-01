# Operation Svelte Session And Poodle Projection

Date: 2026-07-31
Card: 078
Status: complete

## Changed

- Added a framework-neutral operation presentation controller with
  listener-first startup, monotonic snapshot installation, selection, and
  request-keyed cancellation and dismissal state.
- Added per-instance Svelte session and mounted lifecycle subpaths.
- Added a controlled public-Poodle operation panel and pure progress/status
  projectors.
- Added Soundcheck scan and Loophole render-queue presentation fixtures.
- Added optional Svelte and Poodle peers without changing the root dependency
  graph.

## Decisions

- Command results are candidate snapshots. A newer subscription snapshot or
  authority epoch wins.
- Renderer teardown clears local state and releases observation. It never
  requests host cancellation.
- Selection is renderer-local and clears when the selected operation leaves
  the authoritative projection.
- Operation labels remain bounded consumer input in the authority protocol.
  Product detail remains an injected Poodle snippet.
- Cancellation confirmation says that acceptance is a request, not a terminal
  stop claim.

## Evidence

- Two sessions observe one host authority while keeping selection independent.
- Late listener registration unlistens exactly once and takes no snapshot
  after teardown.
- Immediate remount reloads current host truth.
- Delayed cancellation and dismissal results cannot regress catalogue revision
  or authority epoch.
- Determinate unit, normalized, and indeterminate progress all render through
  public Poodle primitives.
- Root and Svelte sources contain no Poodle import. The Poodle adapter imports
  only `@poodle/svelte`.
- SSR imports, TypeScript checks, Svelte checks, mounted tests, package tests,
  and dry-run package assembly pass.

## Next

Execute Card 079. Implement the independent retained notification ledger and
failure-isolated optional operation observer.
