# Private 0.1 Compatibility Candidate And Closeout

Date: 2026-08-02
Roadmap: g01.016
Card: 127
State: complete; g01.016 closed; Card 070 ready

## Result

Longhorn has one coordinated private `0.1.0` candidate from selected clean
source paths at `7a8cc7b407422c50edb194ae78f3b52daa59cdda` and exact Poodle
package paths at `a2b041bad8132e38ce7219cb6872861dcc5866e0`.

Two independent production passes yield identical bytes and inventories:

- 17 Longhorn TypeScript/Svelte tarballs: set `71549fdef28c21912ce0a85cb50b52fa9de142e46e6503ec48acd53992c5a084`
- five Poodle tarballs: set `a7e6351fd9eb92f3f43629afaf0e05a64839467ba68e8bba8852254787dbe2e8`
- 36 Rust inventories and private source archives: set `1d8d7a8f248bc3f7bce5f5d0bf4e9e584ddb65a36b2546abae0e985bd05c97d2`

The machine receipt is
`fixtures/release/card127/private-0-1-candidate-v1.json`. The human matrix is
`docs/reference/private-0-1-candidate.md`.

## Consumer Proof

Minimal, Nucleus, Loophole, Soundcheck, Bovine, Jetstream, and optional-server
roots install from produced artifacts outside sibling workspace resolution.
All seven renderer roots resolve one Svelte 5.56.8 and Tauri API 2.11.1
runtime. All seven Rust roots check offline on Rust 1.85 with the exact lock;
selected Tauri roots resolve 2.11.5. Optional edges match each declared graph.

The exact merged consumer commits remain read-only. Poodle contract-generation
work and Bovine documentation work lie outside selected candidate paths and are
untouched.

## Compatibility

- 12 generated protocol fixtures: exact v1 and zero drift
- Rust: edition 2024, Rust 1.85 exact lock
- repository TypeScript: 5.9.3
- isolated consumer TypeScript: 6.0.3
- Svelte and Tauri peers: exact proof plus package-specific declared ranges
- native platform claims: mechanism-specific, with Windows/Linux unproved or
  unsupported where recorded

All checked TypeScript and Rust packages remain version `0.1.0`. Every Rust
crate has `publish = false`; every TypeScript package has `private: true`.

## Release-gate Classification

Effigy uses explicit `workspace.package.version` discovery and two read-only
gates: the full private candidate proof and workspace QA. The following remain
public-release-only and deliberately unmet:

- npm/crates.io ownership and final public names
- registry-normalized Cargo packages
- final public ranges and hosted surfaces
- registry publication, Git tag, and hosted release execution

No package, tag, GitHub release, or consumer release is created.

## Validation

- deterministic candidate and seven isolated graphs: pass
- 12 generated-binding drift checks: pass
- human/machine receipt cross-check: pass
- configured read-only Effigy release gates: pass; 2/2 gates, no blockers

## Next

Execute Card 070. Card 069 already selected `Promote`; Card 127 satisfies the
package, compatibility, and linear-consumer checkpoint for the optional
fork-tree lane. Package publication still needs separate explicit operator
action.
