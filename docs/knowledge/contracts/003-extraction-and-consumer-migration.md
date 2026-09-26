# 003 Extraction And Consumer Migration

Status: active  
Owner: Tom  
Updated: 2026-07-27

## Admission

A candidate enters Longhorn when:

- behavior is useful to at least two current apps, or
- it is a stable mechanism with a strong greenfield case
- product policy can be separated from mechanism
- ownership and validation can move with it

Similar-looking UI alone is insufficient.

## Extraction Lane

1. Name donor behavior and authority.
2. Capture focused donor tests or fixtures.
3. Define the generic contract and rejected app-specific concerns.
4. Implement the smallest shared package.
5. Migrate one donor consumer.
6. Migrate or prove a second materially different consumer.
7. Remove superseded donor copies.
8. Record behavior deltas and closeout evidence.

## Consumer Safety

- Consumer repos are read-only during research.
- A migration card names every repo it may modify.
- Dirty unrelated work is preserved.
- Breaking changes stop for operator policy.
- Pre-1.0 migrations are clean; no silent compatibility layer.

## Ownership Transfer

- Before cutover, donor behavior is authoritative.
- After validated cutover, Longhorn contract and tests are authoritative.
- Consumer repos retain product configuration and adapters.
- A donor copy left active after cutover is drift, not fallback.

## Done

Library scaffolding alone is not extraction.

Done requires:

- shared implementation
- shared tests
- migrated real consumer
- second-consumer conformance or adoption
- donor duplicate removed or explicitly retained as product policy
- docs and authority map current
