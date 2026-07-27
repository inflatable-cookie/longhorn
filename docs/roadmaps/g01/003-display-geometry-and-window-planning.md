# g01.003 Display, Geometry, And Window Planning

Status: blocked on `g01.002`  
Owner: Tom  
Updated: 2026-07-27
Governing refs: contract 009

## Outcome

Extract pure display correlation, coordinate, geometry, and desired-window
planning without Tauri or Surface dependencies.

## Batches

### 1. Geometry model

- stable ids and typed logical/physical positions and sizes
- scale conversion, intersection, containment, clamping, minimum visibility
- deterministic normalization and property tests

### 2. Display inventory

- observed versus known display records
- arrangement signatures and correlation strategy seam
- labels, main/built-in hints, missing-display recovery

### 3. Window planner

- configured target and ordered fallbacks
- geometry memory per display
- pure desired plan and live-versus-desired diff
- no-Surface and hosted-Surface fixtures

## Acceptance

- Loophole, Nucleus, and Soundcheck geometry cases pass
- coordinate types prevent silent logical/physical mixing
- missing and rearranged displays resolve deterministically
- package graph has no Tauri, Svelte, Poodle, or product dependency
