# g02.013 TypeScript Package Graph

Status: complete
Owner: Tom
Updated: 2026-08-08
Governing refs: contract 012; contracts 013 and 020
Depends on: none

## Outcome

Three published TypeScript packages instead of eighteen, grouped by peer
requirement rather than by domain, with every current entry point preserved
as a subpath.

## Generation Runway

Thirteenth g02 task. Opened by measurement, like g02.011: the split was
examined for what it actually buys and, on the TypeScript side, it buys
nothing the Rust side gets. It is time-boxed by publication rather than by
dependency — published names freeze.

## Execution Plan

### Stage 1. Consolidation

- [x] Card 164
  collapses eighteen packages to three and migrates the four consumers

## Goals

- [x] three packages, every entry point still resolving
- [x] optional peers still gate the subpaths that need them
- [x] a skewed pair of longhorn packages becomes impossible to install
- [x] contract 012 states the Rust and TypeScript cases separately

## Acceptance Criteria

- [x] bindings regenerate with no semantic diff
- [x] nucleus, loophole, soundcheck and jetstream compile and pass
- [x] every asserted package count is refreshed (candidate-receipt counts moved to g02.008 with Card 149)

## Explicit Non-goals

- touching the Rust crate graph. It is measured and earns its keep: finch
  compiles 8 crates of 41, and the pure/host separation is compiler-enforced.
- resurrecting a standalone svelte-without-poodle package. No consumer has
  ever used one without the other; if the case appears it is a subpath
  promotion, not a re-split.

## Next Task

Card 164 landed 2026-08-08 and
Card 165 on 2026-08-09.
`proof:artifacts` is green across all twelve proofs.

## Absorbed records

Absorbed from `batch-cards/` in the flattened-task migration; the directory is removed and git history is the full-fidelity archive.

- Card 164: complete — eighteen packages to three plus consumer migration (landed 2026-08-08).
- Card 165: complete — artifact proof selection model (landed 2026-08-09). Package-count freshness for the candidate receipt moved to g02.008 with Card 149.
