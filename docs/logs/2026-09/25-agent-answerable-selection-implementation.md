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

**Open consumer gate:** Figmatic has not yet replaced its folder-picker
call sites or run the fresh-leaf New/Import project flow through MCP with
this source candidate. The Figmatic Chatterbox owns that consumer route.
The Longhorn package manifests still say `0.2.0`, and the published
`v0.2.0` artifacts predate this implementation. A source link can support
consumer acceptance; a new tag/npm publish needs an operator release
choice after the evidence. HTML file inputs and Rust-side pickers remain
outside this seam.
