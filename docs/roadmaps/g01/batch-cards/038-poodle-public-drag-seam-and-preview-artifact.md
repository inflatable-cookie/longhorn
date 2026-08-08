# 038 Poodle Public Drag Seam And Preview Artifact

Status: complete (2026-07-29)
Owner: Tom
Roadmap: g01.007 batch 2
Governing refs: contracts 011-013; research memo 011
Depends on: Card 036
Named external repo: `~/Dev/projects/poodle`
Auto-start next card: no

## Objective

Land and prove the public Poodle extension and artifact boundary required by
Longhorn adapters without transferring Poodle authority into Longhorn.

## Scope

- Poodle-local contract and roadmap promotion
- public `DockRegion` pre-drag, start, end, eligibility, and external-drop seam
- asynchronous Longhorn session preparation before native dragstart
- same-window reorder compatibility
- public component and type exports
- aligned preview package metadata
- exact packed-artifact install and mounted interaction proof
- recorded exact artifact identity for Longhorn private adapter tests

## Public Behavior

Poodle continues to own pointer and HTML5 drag presentation, local reorder,
drop affordances, and component markup. A consumer extension can prepare an
opaque external transfer before dragstart, write its public payload during
dragstart, decide external eligibility, receive a drop, and cancel or finish
on drag end.

If preparation is not ready, Poodle does not advertise a cross-window payload.
No extension needs generated ids, CSS classes, private MIME data, or DOM
reverse engineering.

## Out Of Scope

- a Poodle dependency on Longhorn
- Longhorn protocol types in Poodle
- Longhorn package implementation
- consumer migration
- public registry release
- redesign of unrelated Poodle components

## Steps

1. Use Poodle's Northstar spine to promote the exact public extension contract.
2. Preserve existing Tabs, DockRegion, and SplitView controlled APIs.
3. Add typed preparation, lifecycle, eligibility, and external-drop hooks.
4. Keep local Poodle reorder and its visual affordances intact.
5. Align component, token, icon, headless, and style preview metadata needed by
   one packed install.
6. Export the new seam through public package entries.
7. Add race, cancel, local reorder, external drop, and accessibility fixtures.
8. Pack and install the exact preview artifact outside sibling source
   resolution.
9. Record the artifact identity and supported Svelte floor for Card 039.

## Acceptance Criteria

- Poodle-local contract approves the public API before implementation
- preparation completes before native payload write
- unready preparation cannot emit an external payload
- superseded, cancelled, and ended preparation is observable
- local same-region reorder remains Poodle-owned
- external eligibility drives the existing visual drop affordance
- no public hook exposes Longhorn types or product policy
- install proof uses no Vite alias, private selector, or sibling source
- exact preview versions and Svelte peer metadata are coherent

## Evidence Required

- Poodle contract and roadmap links
- public API and event-order table
- preparation race and cancellation matrix
- local and external drag mounted fixtures
- package manifest and export audit
- packed artifact identity and clean install proof
- Poodle validation report

## Result

- Poodle `g12.016` promoted and implemented the public typed external drag
  source and drop target boundary.
- Async preparation begins on primary pointerdown. Native `dragstart` writes a
  payload only when preparation is ready.
- Cancellation, end, target eligibility, external drop, local reorder, and
  accessible-name behavior have mounted coverage.
- Poodle retains local interaction and visual authority. No public type names
  Longhorn, host sessions, windows, Surfaces, or product policy.
- The five preview packages install and mount outside sibling source
  resolution at Svelte `5.38.6`.

## Exact Artifact

Artifact set:
`39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74`

Poodle evidence:
`~/Dev/projects/poodle/.artifacts/g12.016-A698XB/evidence.json`

Supported Poodle Svelte peer range: `>=5.38.6 <6`

| Package | SHA-256 |
| --- | --- |
| `@inflatable-cookie/poodle-headless@0.1.0` | `f6132a3fbb44f795bdc7775586e08438321124163678bf3b99ad68958923cfe2` |
| `@inflatable-cookie/poodle-styles@0.1.0` | `9523011c14e00bbd57fe6bce95cf481d35ca5441d990449eddff7733de5bc0f9` |
| `@inflatable-cookie/poodle-svelte-tokens@0.1.0` | `59630dfacfcd802b221dfb6368a38b8f7f4217129a5f8482e1d2983648b8c175` |
| `@inflatable-cookie/poodle-icons-lucide@0.1.0` | `88df4087c5cb2403b8da308cc59ad392d94be0fc16d81a72fa556e1f24cb8e70` |
| `@inflatable-cookie/poodle-svelte@0.1.0` | `d0ab2f25ba31050d8b2dbf903ea90f5714b7f1337635fc5bdafd40d239a0b75a` |

## Validation

- Poodle `effigy ci:web` passed: 272 core tests, 655 mounted component tests,
  zero Svelte errors, and all contract/spec drift checks
- Poodle `effigy test:svelte-pack-install` passed against the exact artifact
  set at the Svelte floor
- Poodle `effigy docs:check` and `git diff --check` passed
- Poodle's existing Svelte accessibility/reactivity warnings remain outside
  this seam

## Stop Conditions

- no Poodle-local contract authorizes the API
- the seam requires a Longhorn dependency
- native drag cannot wait for safe session preparation
- local reorder regresses
- a packable preview requires public registry release
- the change expands beyond drag extension and package coherence

## Next Task

Card 039 is ready against the exact artifact above. Do not start it
automatically.
