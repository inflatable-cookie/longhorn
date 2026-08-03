# 073 Fork-tree Clients And Bounded Projections

Status: complete
Owner: Tom
Roadmap: g01.017 batch 3
Governing refs: contracts 007, 008, 010, 012, and 013; Cards 066 and 072
Depends on: Card 072
Auto-start next card: yes

## Objective

Add metadata-only tree clients with a linear default and bounded on-demand
branch and path projections.

## Scope

- generated graph summary, branch page, and path page protocol
- linear past/current/future default
- explicit pagination and truncation evidence
- branch navigation commands and authoritative receipts
- framework-neutral checked client
- optional Tauri, Svelte, and public-Poodle edges
- listener-first refresh and stale authority rejection
- SSR, teardown, and package isolation

## Out Of Scope

- product payload transport
- generic branch visualization component in Poodle
- automatic alternate-path loading
- project version or collaboration UI

## Steps

1. Freeze bounded metadata projections and commands.
2. Generate and check Rust/TypeScript compatibility.
3. Implement direct and serialized clients.
4. Add narrow caller-aware Tauri assembly.
5. Add per-instance Svelte state and controlled Poodle composition.
6. Prove default imports load no alternate-path data.

## Acceptance Criteria

- default renderer state remains linear
- every alternate surface is bounded and explicit
- product payloads never cross the protocol
- authority epoch and graph revision invalidate stale work
- optional edges remain removable
- mounted teardown and SSR pass

## Evidence Required

- generated fixture and drift check
- pagination and truncation matrix
- direct/serialized conformance
- Svelte lifecycle and Poodle public-API proof
- dependency and payload audits

## Stop Conditions

- branch projection requires unbounded lineage duplication
- renderer needs product payloads
- a Poodle component must be copied

## Next Task

Card 074 proves isolated artifacts and closes g01.017.
