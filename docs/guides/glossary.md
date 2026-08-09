# Glossary

Status: checked
Updated: 2026-08-08

The docs use a small set of internal terms. This page defines them in plain
language. If a word still confuses you, it belongs here.

## Terms

| Term | Plain meaning |
| --- | --- |
| authority | The single component that owns a durable domain — the only place writes to that domain may commit. "Product authority" means the app keeps its own policy; "transaction authority" means one coordinator makes several writes atomic; "bootstrap authority" means which system owns a domain at startup. |
| admission | The decision that something is allowed to proceed. Behavioral admission: is this behavior worth extracting (see donor)? Artifact admission: do the produced packages install cleanly outside this repo? |
| bounded | Deliberately limited, and the limit is part of the contract — bounded sessions, bounded retention, bounded debounce. Not a bug or an accident. |
| card | A numbered unit of planned work on the roadmaps (g01 and g02 runways, cards 001-159). Each card has an outcome, evidence, and closeout. |
| consumer | An app that adopts Longhorn packages. Proven consumers: Nucleus, Loophole, Soundcheck, Split-shell, Jetstream. |
| conformance | Proof that a consumer's behavior matches the shared contract — or a fixture that verifies it. |
| cutover | The step where a consumer switches a domain from its own implementation to Longhorn's. |
| donor | An existing app whose proven behavior was extracted into Longhorn. A donor implementation is evidence, not automatically the shared contract. |
| effigy | The repo's task runner. Every repo command goes through `effigy <selector>`; selectors are defined in `effigy.toml` at the repo root. The command cookbook lives in `scripts/README.md`. |
| freeze | Capturing exact current state (commits, fixtures, files, locks) before a change, so rollback is provable. |
| front door | The public entry docs — repo README, `docs/README.md`, and the adoption guides — that must state current truth. |
| generation / milestone | A generation is a coherent delivery program (g01: the first shared systems; g02: the current integrity and release runways). A milestone is a chunk of cards inside it. |
| proof | Evidence that something works — usually "packaged proof", i.e. tested on a real installed app, not just unit tests. |
| promotion | Moving a proven mechanism from prototype or spec into the shared contract and package set. |
| receipt | A typed record of a completed operation (mutation, backup, restore, transition) that later steps verify against. "Receipt-bound cleanup" means cleanup is allowed only for paths and digests named in a committed receipt. |
| seam | A stable extension point where the app injects its own implementation — e.g. the diagnostics seam, the Poodle drag seam, or a custom backup adapter. |
