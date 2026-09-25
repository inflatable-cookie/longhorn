# Agent-Answerable Selection Implementation

2026-09-25. Longhorn's contract 022 Tauri JS `open`/folder/`save` selection
seam merged through [PR #34](https://github.com/inflatable-cookie/longhorn/pull/34).
Worker head `fc9ad71470b6b8e66aea79e4977b6796b047f4e6` merged at
`3b3db6c7ff142a973b2268cad7f8cf26d7fa364a`; Queue closeout synchronized
`main` at `703012ab96deae46c8d593ec69b382febe95c06c`. Queue task
`716bcb17-a24e-43a0-aeff-f4e140a32a8c` is done for the Longhorn PR loop.

The implementation adds a bounded per-instance selection registry, MCP
`answer_selection` and `reject_selection`, the readable/subscribable
`longhorn://agent-control/selection` resource, Tauri
`longhorn_agent_control_begin_selection`, and the TypeScript
`bindFileSelection` entry point. Agent-originated calls wait for an MCP
answer before any native panel opens. Human calls continue through the
consumer-supplied plugin-dialog functions. `save` selects a target only;
Longhorn does not write it. The featureless build omits the seam.

The worker ran `effigy qa` green. Independent review comment `5832387981`
approved exact head `fc9ad714` after focused selection registry/MCP,
TypeScript origin/selection, and release-absence checks. Review found no
blocking change.

**Consumer evidence received 2026-09-25:** Figmatic g01.052 proof commit
`d77c3d16bb5f81bb6357f56c73e3ba64864f323b` on its retained pre-PR
branch used machine-local Effigy Cargo/Bun links to clean Longhorn
`db0b6baf`. From a fresh storage leaf, semantic MCP click on New project
published pending selection `sel_IMpvVWNBzRHT4T38` (`open`, directory,
single). `answer_selection` supplied an existing disposable folder; the
app opened project `db2d3251-f481-4a4c-8b15-6526de9bdcc1` at
Prepare/Designs with `focused: false`. No human picker or OS input was used.
Six focused JS tests, nine Rust tests, and dev Cargo check passed. Import
shares the call site and has a focused test but was not live-clicked.
Figmatic's evidence file is
`docs/logs/2026-09/25-154800-g01-052-agent-answerable-project-selection.md`
at that commit. This satisfies g02.045's source-linked live acceptance row;
the Figmatic branch is not merged.

**Publication gate:** Longhorn manifests still say `0.2.0`, and published
`v0.2.0` predates this implementation. Figmatic is blocked before PR on an
exact published npm version and Rust tag, then must remove source links and
run clean-install QA. A new tag/npm publish requires the operator's release
choice and gates. HTML file inputs and Rust-side pickers remain outside
this seam.
