# Private 0.1 Compatibility Candidate

Status: checked private candidate
Version: `0.1.0`
Date: 2026-08-02
Machine receipt:
[`private-0-1-candidate-v1.json`](../../fixtures/release/card127/private-0-1-candidate-v1.json)

This is an installable local compatibility candidate, not a registry, tag, or
hosted release. Working package names and registry ownership remain unresolved.

## How To Obtain The Artifacts

The candidate is the *only* documented way to consume Longhorn today. There is
no npm, crates.io, or hosted download.

- The machine receipt is
  [`fixtures/release/card127/private-0-1-candidate-v1.json`](../../fixtures/release/card127/private-0-1-candidate-v1.json):
  it records every artifact set, checksum, frozen source commit, and consumer
  graph.
- The receipt is produced (and re-producible) by the private-candidate
  generation command documented in `scripts/README.md`; verification runs
  read-only through `effigy release gates`.
- TypeScript and Svelte artifacts are private npm-compatible tarballs. Rust
  artifacts are Cargo inventories plus commit-derived source archives (not
  registry-normalized `.crate` files).
- Consumer graphs install from these artifacts with the exact lock, outside
  sibling workspace resolution — that is what the consumer matrix below
  proves. There is no one-line install command yet; the per-graph install
  recipe is part of the receipt.

Known limitation: a fresh clone of this repo cannot currently install its
root TypeScript dependencies, because the root `package.json` pins Poodle to
machine-local build artifacts. The fix is scheduled for a Poodle release
(reference `PAPERCUTS.md`); until then the candidate artifacts are produced
from the frozen commits listed below, not from a fresh clone.

## Current State Note

This candidate is frozen at its 2026-08-02 source state: 17 TypeScript
packages and 36 Rust crates. The live tree has since grown (18 TypeScript
packages, 41 Rust crates, plus the g02 update and licensing crates). Treat
this receipt as the *proven* point-in-time artifact set, and the
[generated API surface](api-surface.md) as the *current* inventory.

## Frozen Sources

| Source | Commit | Candidate boundary |
| --- | --- | --- |
| Longhorn | `7a8cc7b407422c50edb194ae78f3b52daa59cdda` | all 17 TypeScript package trees, all 36 Rust crate trees, workspace manifests and locks |
| Poodle | `a2b041bad8132e38ce7219cb6872861dcc5866e0` | five selected package trees; unrelated contract-generation work excluded |
| Nucleus | `c0aeeed84ea76e7c8dbcdcfd7943d0f9a5cb10d9` | merged no-Surface app manifests |
| Loophole | `7bf096d4d17f77194c47456f5c59661fe3c4495f` | merged Aura, Echo storage, and Pulse manifests |
| Soundcheck | `e8970aa212e9464f53264f58f6628cd98254fc30` | merged desktop and workspace manifests |
| Split-shell | `8b3c19e8d1e96ce81d1dbbaf4769c9e72648dd28` | merged minimal graph manifests; unrelated docs work excluded |
| Jetstream | `e9a54daacdec1f5c6573687a5543e9ffb2dae2b0` | merged editor graph manifests |

Selected paths are clean against each commit. No consumer repository is
written during candidate production.

## Artifact Sets

| Family | Count | Set SHA-256 | Form |
| --- | ---: | --- | --- |
| Longhorn TypeScript/Svelte | 17 | `71549fdef28c21912ce0a85cb50b52fa9de142e46e6503ec48acd53992c5a084` | private npm-compatible tarballs |
| Poodle | 5 | `a7e6351fd9eb92f3f43629afaf0e05a64839467ba68e8bba8852254787dbe2e8` | private npm-compatible tarballs |
| Longhorn Rust | 36 | `1d8d7a8f248bc3f7bce5f5d0bf4e9e584ddb65a36b2546abae0e985bd05c97d2` | Cargo inventories plus commit-derived private source archives |

Every tarball, source archive, inventory, filename, version, and checksum is
produced twice and compared byte-for-byte. Rust source archives are not
registry-normalized `.crate` files. Normalized Cargo packaging remains a
public-release gate.

## Consumer Matrix

| Graph | TypeScript | Direct Rust | Resolved Rust | Boundary |
| --- | ---: | ---: | ---: | --- |
| minimal | 4 | 6 | 6 | config/settings only |
| Nucleus | 11 | 24 | 24 | workspace and child view; no Surface/transfer graph |
| Loophole | 12 | 23 | 23 | full layout/Surface/transfer and linear history |
| Soundcheck | 6 | 14 | 14 | storage/settings/window/operation/isolated window |
| Split-shell | 4 | 6 | 6 | minimal config/settings graph |
| Jetstream | 6 | 6 | 6 | bridge/commands/backing surface |
| optional server | 5 | 8 | 8 | valid local config/settings plus selected bridge |

Each renderer installs from produced tarballs outside sibling workspace
resolution. Each graph has one Svelte `5.56.8` runtime and one Tauri API
`2.11.1` runtime. Each Rust root checks offline with Rust `1.85.0`, the exact
candidate lock, and Tauri `2.11.5` where selected. The greenfield optional-
server proof separately checks `--no-default-features` local authority.

## Compatibility

- Longhorn package version: coordinated `0.1.0`
- repository TypeScript: `5.9.3`
- isolated consumer TypeScript: `6.0.3`
- Svelte: exact proof at `5.56.8`; package peer ranges remain package-specific
- Poodle: the exact five-package artifact set above, not a public version range
- protocols: 12 checked fixtures, exact v1 negotiation
- native platforms: mechanism-specific claims in the
  [compatibility guide](../guides/compatibility-and-upgrades.md)

Compatibility failures remain explicit. No protocol or package mismatch may
fall back to a handwritten DTO, generic command bus, sibling workspace, or
second runtime.

## Deferred Public-release Gates

- npm and crates.io name ownership
- final public package names and compatibility ranges
- registry-normalized Cargo packages
- public changelog/version bump policy and hosted release surfaces
- registry publication, Git tag, and hosted release execution

The Effigy release configuration runs only read-only private-candidate and
workspace gates. Mutating prepare/execute commands remain outside standing
authority.
