# Nucleus Private Artifact Admission

Date: 2026-08-01
Card: 095
Outcome: `pass`

## What Changed

- Added `effigy proof:nucleus-private-artifacts`.
- Pinned clean Nucleus commit
  `df5af3da03f8392f948ff65d0a3cf16c36cb6c12`, which records both operator
  checks without changing donor application code.
- Pinned clean Poodle g12.018 commit
  `ef41f412ad7b45c2ee760c1da9bf41ef876855e8` and artifact set
  `ed9d800843a5d008a812a29000cbe2fcd3d619ea53e231627a1f253449c4d41d`.
- Recorded the exact Longhorn commit, selected-source tree digest, private
  sibling-source map, produced artifact identities, and rollback posture in
  selector output.
- Closed every Card 094 admission gate and admitted Card 096's bounded donor
  writes.

## Renderer Proof

The clean temporary Nucleus shell installs produced archives for:

- `@inflatable-cookie/longhorn-core`, `@inflatable-cookie/longhorn-config`, `@inflatable-cookie/longhorn-layout`
- `@inflatable-cookie/longhorn-svelte`, `@inflatable-cookie/longhorn-poodle`, `@inflatable-cookie/longhorn-tauri`
- `@inflatable-cookie/longhorn-native-content`, `@inflatable-cookie/longhorn-native-content-svelte`
- `@inflatable-cookie/poodle-headless`, `@inflatable-cookie/poodle-styles`, `@inflatable-cookie/poodle-svelte-tokens`
- `@inflatable-cookie/poodle-icons-lucide`, `@inflatable-cookie/poodle-svelte`

It uses Svelte 5.56.8, TypeScript 6.0.3, and Tauri API 2.11.1. Generated
config, layout, and native-content bindings are current. The public Poodle
`onSurfaceGeometryChange` contract compiles for Popover and Menu. Both mounted
Nucleus app-shell tests pass. No sibling alias, duplicate runtime,
`@inflatable-cookie/longhorn-surfaces`, or `@inflatable-cookie/longhorn-surface-transfer` resolves.

| Longhorn package | SHA-256 |
| --- | --- |
| `@inflatable-cookie/longhorn-core` | `b41c3585e48f8e538acaccc68209660e7be55962bdea0af443af924011fcc9f0` |
| `@inflatable-cookie/longhorn-config` | `b8285aa4d332a1a6348242703fedc3ae190e5a087c0a9f6ed1bda03c769a5407` |
| `@inflatable-cookie/longhorn-layout` | `f3ebecc3f6abab4404be23eb73a64c6db8185421754e7c5644754f997d8f1fa5` |
| `@inflatable-cookie/longhorn-svelte` | `024dd873ff3d99e99755d76046076476783f60228b2bf1f2ecef7439aae83ca3` |
| `@inflatable-cookie/longhorn-poodle` | `75cb6ed35ec623fc60db6af14daf9c0e923d45ad51bc68b2a2d2f5c3548ebb63` |
| `@inflatable-cookie/longhorn-native-content` | `aaaa3b842b4605c71f5e88bc56919ccf1b6e6475ccb928f52d19e171b24c8c22` |
| `@inflatable-cookie/longhorn-native-content-svelte` | `4ad53d146484b23be41862839f396821a54f6fbfe87f4a450d2d6154839c4913` |
| `@inflatable-cookie/longhorn-tauri` | `4df629c1bc5ebd889bdb29e100956f0a2327fc4fc7cee38843b45a086f7fa3c5` |

## Rust Proof

The proof inventories and archives eleven selected packages:

- core, config, Tauri config
- display, windowing, Tauri windowing, windowing config
- layout, layout config
- native content and Tauri child view

The Nucleus-shaped consumer compiles offline on Rust 1.85 with Tauri 2.11.5.
`longhorn-surfaces` and `longhorn-surface-windowing` are copied only as private
workspace support because a selected package's development metadata names
their manifests. Neither enters the normal consumer graph.

Private Rust source-archive hashes are run-local evidence, not stable release
identities. The selector emits every value and ties it to selected source tree
digest `6ec8a62ec7076f89ecdb76ffedebfb07db8b23e60f39d1f90ac84d8d5e611b61`.

A fresh unlocked Tauri 2.11 resolution selected transitive crates requiring
Rust 1.86-1.88. That graph cannot support an MSRV claim. The passing proof
starts from Longhorn's compatible lock, evolves Tauri to exactly 2.11.5
offline, then runs both check and tree with `--locked`. Its output records the
resulting lock SHA-256.
The admitted lock digest is
`b773f1e281f6200e21a2a1c98329dd38d36e0687b5a08253b81f2bfd829a24dd`.

## Admission

The exact private graph is admitted. Longhorn and Poodle remain unpublished;
registry ownership and public compatibility ranges stay in the later release
lane. Admission did not modify Poodle or Nucleus application sources.

The operator accepted cross-project layout retention and confirmed a previously
unseen project opens with Agent Chat only. Donor writes are admitted. The
previous Nucleus build, legacy stores, and donor source remain the rollback
set for Card 096.
