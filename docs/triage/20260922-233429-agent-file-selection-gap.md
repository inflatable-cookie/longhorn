# HTML file input and Rust picker remain outside agent selection

Status: open — separate follow-up, not part of g02.045
Owner: Longhorn Chatterbox
Next check: after g02.045's Figmatic acceptance, or when a consumer reports
an agent-control blocker through either remaining mechanism

## Settled on 2026-09-22

The operator chose a Longhorn-owned selection seam for Tauri JS `open`,
directory, and `save`, available in dev and packaged `agent-control` builds.
Figmatic is the first live acceptance target. Contract 022 carries the
boundary; [g02.045](../roadmaps/g02/045-agent-answerable-file-selection.md)
is the ready Longhorn implementation card. Its path-returning calls need no
file-byte bridge. The former app-owned mock responder deferral in memo 024
is closed by that amendment.

## Still open

Soundcheck has one HTML `<input type="file">` in
`LibraryManifestSettings.svelte`. Its handler calls `input.files?.[0]` and
`file.text()`. This is a different browser mechanism: agent answer would
need a real `File` and a byte bridge, not just a path. The same screen
already accepts pasted JSON, so the input is not currently a live
agent-control blocker. The operator was open to including it if cheap;
reconnaissance shows it is a separate slice. Do not fold it into g02.045.

Nucleus also has one Rust-side `pick_file` call in
`config_operations/restore/commands.rs`. It does not cross the Tauri JS
dialog entry point and has no current acceptance request.

## Decision to revisit

If either mechanism blocks a consumer's live agent run, characterize its
bytes, trust boundary, and acceptance target before amending contract 022
or opening another card. No implementation is authorized by this note.
