# Native-content Artifact Proof And Closeout

Date: 2026-08-01
Card: 093
Roadmap: g01.018

## Result

Produced native-content artifacts preserve the promoted split graph. Four
offline Rust consumers and three isolated renderer consumers pass. Rust and
renderer traces agree for child view, isolated window, and backing surface.
Fresh packaged macOS lifecycle and teardown reruns preserve the recorded
support limits.

## Artifact Evidence

Rust source inventories cover `longhorn-core`, `longhorn-native-content`,
`longhorn-tauri-native-content-child-view`,
`longhorn-native-content-isolated-window`, and
`longhorn-native-content-backing-surface`.

Each inventory follows a successful `cargo package --list --allow-dirty` and
compiles from an exact private source archive in an offline temporary
workspace on Rust 1.85. The proof emits the digest of each recreated archive;
those run-local hashes are evidence, not stable artifact identities.
Registry-normalized Cargo packaging remains a release gate.

TypeScript packages are packed at `0.1.0`: `@longhorn/core`,
`@longhorn/native-content`, and `@longhorn/native-content-svelte`. Consumer
locks contain no workspace, link, or sibling source resolution. Mounted
consumers use exact Poodle artifact set
`39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74`.

## Graph And Semantic Evidence

| Shape | Rust graph | Renderer graph | Result |
| --- | --- | --- | --- |
| Pure | core, native-content | none | child-view baseline trace passes |
| Nucleus | pure + child-view | core, native-content, native-content-svelte, Svelte/Poodle | exact child-view parity and mounted public-layout compilation |
| Soundcheck | pure + isolated-window | core, native-content | exact isolated-window parity; no Svelte/Poodle |
| Jetstream | pure + backing-surface | core, native-content, native-content-svelte, Svelte/Poodle | exact backing-surface parity and mounted public-layout compilation |

Unselected mechanisms and browser, plugin, GPU, Signal, semantic-input, raw
handle, private-Poodle, and product payloads remain absent. Nucleus and
Jetstream capability files contain only protocol reads/mutations plus core
event listening. Soundcheck needs no renderer capability.

## Packaged Support Ledger

| Mechanism | Fresh macOS bundle result | Windows | Linux |
| --- | --- | --- | --- |
| Child view | seven pass; focus/visibility observation unknown; live scale switch unmet | unproved | unproved |
| Isolated window | 11/11 pass | unsupported | unsupported |
| Backing surface | ten pass; live scale transition unmet | unsupported | unsupported |

All three production `.app` bundles were rebuilt and launched from their
bundle executable paths. Lifecycle, replacement/detach, stale-generation,
host-destruction, and teardown evidence passed. The host exposed no additional
native scale, so mixed-display behavior remains unproved and unsimulated.

## Migration Prerequisites

| Consumer | Artifact gate | Admitted work | Remaining blocker |
| --- | --- | --- | --- |
| Nucleus | pass | migration planning | browser construction/source URL; navigation, popup, download, permission, and data-store policy; panel/overlay visibility policy; remote-child capabilities |
| Soundcheck | pass | after g01.014 and g01.015 | Signal/plugin ABI and authorization; helper/process ownership and teardown; media and native content policy |
| Jetstream | pass | after g01.014 and g01.015 | native storage/WGPU construction; renderer/world/camera/picking/gizmo/frame loop; semantic input mapping |
| Loophole | not applicable | no current native-content migration | no native-content dependency proved |

No donor repository was written. Artifact readiness does not transfer product
authority or authorize cutover.

## Prototype Disposition

Cards 082-085 remain retained, non-publishable evidence. Their manifests stay
outside production workspace membership. No production consumer resolves a
prototype and no prototype API becomes compatibility authority.

## Validation

- `effigy proof:native-content-artifacts`
- three fresh packaged macOS builds, bundle-path lifecycle runs, and focused
  evidence verifiers
- focused Northstar and package checks
- full `effigy qa`

## Closeout

Card 093 and g01.018 are complete. Nucleus is the earliest
consumer admitted to migration planning. Its native browser cutover remains
blocked until g01.014 records the exact consumer-owned browser policy map.
