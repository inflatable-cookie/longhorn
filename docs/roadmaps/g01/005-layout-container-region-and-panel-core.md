# g01.005 Layout Container, Region, And Panel Core

Status: blocked on `g01.001` and `g01.002`  
Owner: Tom  
Updated: 2026-07-27  
Governing refs: contract 002

## Outcome

Ship the Surface-independent layout state machine shared by Loophole and
Nucleus.

## Batches

### 1. Model and policy

- opaque layout-container, region, panel-definition, and panel-instance ids
- consumer-defined region families
- allowed-region and instance-count policy

### 2. State and mutations

- ordered tabs, active panel, sizing, collapse, visibility
- create, close, activate, reorder, and cross-region move
- deterministic normalization and rejected-placement behavior

### 3. Persistence and protocol

- registered configuration domain
- checked Rust/TypeScript snapshots and commands
- Loophole eight-region and Nucleus five-region fixtures

## Acceptance

- both consumer fixtures use the same resolver
- no Surface type enters the core package
- invalid placement never mutates durable state
- concurrent/partial layout updates cannot overwrite window geometry

