# Child-view Navigation Artifact Closeout

Date: 2026-08-03
Card: 134
Roadmap: g01.020

## Result

The retained child-view navigation primitive passes source-artifact,
dependency, protocol, packaged-host, and consumer-boundary gates. g01.020 is
complete. No registry release was published.

## Artifact Evidence

- `cargo package --list` passes for the child-view crate
- an isolated Nucleus-shaped Rust consumer compiles `current_url`, `navigate`,
  both outcomes, the receipt, and the runtime trait from staged source
- its Longhorn graph contains only core, native-content, and child-view
- fixture digest:
  `948fcd5481fd0df00dafc40575beb1aae76bff0a0ef6cf240639a005958f2b0c`
- generated TypeScript digest:
  `d1840dfe333b33f666389f7d7a1ebeea00deea7e0ba7c27d788ff9d3e0451a0e`
- neither common artifact contains navigation or URL-observation payloads
- the packaged Card 133 run supplies the real native execution evidence

## Boundary

Longhorn owns generation checks, policy admission, one retained-handle native
submission, readiness events, and exact receipts. Consumers own URL
normalization, command authorization, ordering, history, and product meaning.
Figmatic and Nucleus were audited read-only.

## Validation

- `effigy proof:native-content-artifacts`
- `effigy qa:child-view-navigation`
- `effigy qa`

## Next

Resume Figmatic g03.006 using its handoff. Nucleus may schedule its separate
adapter-control cleanup.
