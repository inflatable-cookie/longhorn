# g02.014 First Publication

Status: ready
Owner: Tom
Updated: 2026-08-09
Governing refs: contract 012
Depends on: g02.013 complete

## Outcome

Poodle and Longhorn on the public npm registry under `@inflatable-cookie`,
every consumer on versions rather than `file:` references, and v0.1.0 tagged.

## Generation Runway

Fourteenth g02 milestone, and the first that is gated on something outside the
repository rather than inside it. Contract 012 held package names as "working
names until registry ownership is verified"; the `@inflatable-cookie` scope was
claimed on 2026-08-09, which satisfies that clause and opens this.

It follows g02.013 by necessity rather than by preference. Publishing before
the two consolidations would have meant deprecating twenty-one names that had
existed for exactly one release — eighteen from Longhorn, three from Poodle.

## Execution Plan

### Batch 1. Publication

- [ ] [Card 166](batch-cards/166-first-publication.md) publishes both
  repositories, repoints twenty-four consumer manifests, and cuts the tag

### Batch 2. Disclosure readiness

- [x] [Card 167](batch-cards/167-publication-disclosure-readiness.md) removes
  third-party identity so either repository can be made public. Independent of
  Batch 1 — publication does not require public repositories, but the operator
  intends them.

## Goals

- the six publish-intent packages resolve from the public registry
- no repository in the portfolio pins Poodle or Longhorn by path
- Longhorn's CI TypeScript lane completes for the first time
- v0.1.0 tagged, Rust by git tag and TypeScript by version

## Non-goals

- **crates.io.** Every Rust crate sets `publish = false` and consumers take
  them by git tag. That distribution model is unchanged and is why the tag
  alone was never the whole runway.
- **Card 149's candidate receipt.** It freezes a cross-repository consumer
  graph and stays operator-held on manifest quiescence, which publication does
  not provide.

## Risks

The step is close to irreversible. npm unpublish is available for 72 hours and
only while nothing depends on the package; after that a name can be deprecated
but never reclaimed. Naming is final at first publish, which is precisely what
contract 012's working-names clause was protecting.

The artifact proofs are a pre-publication device. They pack a sibling build
directory and install it into an isolated root to prove a consumer can compose
the real thing — which publication proves better. Card 166 requires that
decision to be taken *before* the repoint, because the repoint removes the
pack reference the proofs derive their pin from.
