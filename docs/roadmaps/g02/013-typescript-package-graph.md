# g02.013 TypeScript Package Graph

Status: ready
Owner: Tom
Updated: 2026-08-08
Governing refs: contract 012; contracts 013 and 020
Depends on: none

## Outcome

Three published TypeScript packages instead of eighteen, grouped by peer
requirement rather than by domain, with every current entry point preserved
as a subpath.

## Generation Runway

Thirteenth g02 milestone. Opened by measurement, like g02.011: the split was
examined for what it actually buys and, on the TypeScript side, it buys
nothing the Rust side gets. It is time-boxed by publication rather than by
dependency — published names freeze.

## Execution Plan

### Batch 1. Consolidation

- [ ] [Card 164](batch-cards/164-typescript-package-consolidation.md)
  collapses eighteen packages to three and migrates the four consumers

## Goals

- [ ] three packages, every entry point still resolving
- [ ] optional peers still gate the subpaths that need them
- [ ] a skewed pair of longhorn packages becomes impossible to install
- [ ] contract 012 states the Rust and TypeScript cases separately

## Acceptance Criteria

- [ ] bindings regenerate with no semantic diff
- [ ] nucleus, loophole, soundcheck and jetstream compile and pass
- [ ] every asserted package count is refreshed, including Card 149's receipt

## Explicit Non-goals

- touching the Rust crate graph. It is measured and earns its keep: finch
  compiles 8 crates of 41, and the pure/host separation is compiler-enforced.
- resurrecting a standalone svelte-without-poodle package. No consumer has
  ever used one without the other; if the case appears it is a subpath
  promotion, not a re-split.

## Next Task

Card 164, before poodle publishes.
