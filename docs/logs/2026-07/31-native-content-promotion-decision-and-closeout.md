# Native-content Promotion Decision And Closeout

Date: 2026-07-31
Card: 086
Roadmap: g01.013

## Decision

Selected `Promote`.

Promote one pure coordination kernel, three independently selectable host
mechanism layers, a checked TypeScript client, and a per-instance Svelte
lifecycle. Keep Poodle as a public consumer layout seam. Add no
Poodle-specific native-content package.

## Evidence

| Gate | Measured result | Decision |
| --- | --- | --- |
| shared vocabulary | Card 082: 21 passing pure tests; child, isolated, and backing traces are lossless | promote pure kernel |
| isolation | every prototype graph excludes unselected mechanisms and donor stacks | keep three adapter packages separate |
| child view | packaged macOS proof passes core lifecycle/security checks; focus/visibility may be unknown | promote macOS-first; Windows/Linux unproved |
| isolated window | packaged macOS 11/11 matrix passes | promote macOS-only; Windows/Linux unsupported |
| backing surface | packaged macOS clip/input/lifecycle proof passes 10 checks | promote macOS-only; Windows/Linux unsupported |
| scale | deterministic 1x/2x passes; one attached 2x display prevented live switch proof for child/backing | exclude mixed-display support claim |
| authority | no browser, plugin, GPU, semantic input, raw handle, placement, or private Poodle boundary leak | contract 017 holds |

Unknown native focus or visibility is retained as truth, not converted to a
failure or fabricated observation. Live scale-transition evidence remains a
support limit, not a reason to merge mechanisms or discard the reusable pure
boundary.

## Production Graph

- `longhorn-native-content`
- `longhorn-tauri-native-content-child-view`
- `longhorn-native-content-isolated-window`
- `longhorn-native-content-backing-surface`
- `@longhorn/native-content`
- `@longhorn/native-content-svelte`

The names are fixed for g01.018 planning and remain pre-release registry names.
Production implementation does not occur in Card 086.

## Prototype Disposition

Retain `prototypes/native-content*` unchanged as non-workspace,
non-publishable evidence until Card 093 production artifact proof and the first
consumer cutover. The prototypes do not create API or compatibility authority.

## Migration Prerequisites

| Consumer | Shared prerequisites | Retained authority |
| --- | --- | --- |
| Nucleus | Cards 087, 088, 089, 092, 093 | browser/security/navigation/visibility policy |
| Soundcheck | Cards 087, 090, 093 | Signal/plugin ABI, helper, authorization, unsafe-unload policy |
| Jetstream | Cards 087, 088, 091, 092, 093 | WGPU renderer, native storage, semantic input |
| Loophole | none in current migration | no native-content migration invented |

## Roadmap Result

g01.013 closes. g01.018 Cards 087-093 cover production kernel, generated
client, three mechanisms, Svelte composition, artifact proof, support claims,
and the adoption gate. Only Card 087 is ready.
