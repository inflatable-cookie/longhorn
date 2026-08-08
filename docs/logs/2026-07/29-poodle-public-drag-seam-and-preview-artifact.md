# Poodle Public Drag Seam And Preview Artifact

Date: 2026-07-29
Card: 038
State: complete

## Outcome

- promoted and completed Poodle `g12.016`
- added public typed external drag source and drop target seams to
  `@inflatable-cookie/poodle-svelte`
- began asynchronous host preparation before native `dragstart`
- made unready, superseded, cancelled, ended, eligible, and rejected paths
  explicit
- preserved Poodle-owned same-region reorder and visual drop affordances
- aligned five preview package manifests at `0.1.0`
- proved a clean tarball install and mounted interaction at Svelte `5.38.6`
- recorded an immutable artifact set for Card 039

## Public Boundary

Poodle owns pointer and HTML5 drag presentation, local reorder, component
markup, and drop affordances. The consumer owns opaque payload preparation,
session policy, and cross-window meaning.

`DockRegion.externalDragSource` prepares on primary pointerdown. A ready
preparation writes through its synchronous `start` callback during native
`dragstart`. An unready preparation cancels with `not-ready` and writes no
external payload. `end` and `cancel` are mutually exclusive.

`DockRegion.externalDropTarget.canDrop` drives the existing affordance and
`drop` receives accepted external drops only. Public types contain no Longhorn
or product-domain concepts.

## Exact Artifact

Artifact set:
`39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74`

Poodle evidence:
`~/Dev/projects/poodle/.artifacts/g12.016-A698XB/evidence.json`

Svelte peer range: `>=5.38.6 <6`

| Package | Tarball | SHA-256 |
| --- | --- | --- |
| `@inflatable-cookie/poodle-headless@0.1.0` | `poodle-headless-0.1.0.tgz` | `f6132a3fbb44f795bdc7775586e08438321124163678bf3b99ad68958923cfe2` |
| `@inflatable-cookie/poodle-styles@0.1.0` | `poodle-styles-0.1.0.tgz` | `9523011c14e00bbd57fe6bce95cf481d35ca5441d990449eddff7733de5bc0f9` |
| `@inflatable-cookie/poodle-svelte-tokens@0.1.0` | `poodle-svelte-tokens-0.1.0.tgz` | `59630dfacfcd802b221dfb6368a38b8f7f4217129a5f8482e1d2983648b8c175` |
| `@inflatable-cookie/poodle-icons-lucide@0.1.0` | `poodle-icons-lucide-0.1.0.tgz` | `88df4087c5cb2403b8da308cc59ad392d94be0fc16d81a72fa556e1f24cb8e70` |
| `@inflatable-cookie/poodle-svelte@0.1.0` | `poodle-svelte-0.1.0.tgz` | `d0ab2f25ba31050d8b2dbf903ea90f5714b7f1337635fc5bdafd40d239a0b75a` |

## Validation

- Poodle `effigy ci:web` passed: 272 core and 655 mounted component tests
- Poodle Svelte Check passed with zero errors
- Poodle contract/spec drift and docs gates passed
- exact clean consumer mounted `DockRegion`, emitted the public external
  payload, retained local reorder, and exposed an accessible region name
- no Vite alias, sibling source resolution, private selector, private MIME
  knowledge, registry release, or Poodle-to-Longhorn dependency

## Current State

Card 038 is complete. Card 039 is strict-ready against the exact artifact set
above and has not started.

## Next

Start Card 039 Poodle layout bindings.
