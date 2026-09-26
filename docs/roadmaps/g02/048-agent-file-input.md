# g02.048 Agent File Input

Owner: Tom
Created: 2026-09-26
State: ready for Queue delivery — dispatch after g02.047 merges
Governing refs: contract 022 (`Tool Surface`, `Agent-Answerable Selection`,
amended 2026-09-26)
Depends on: g02.047 (shared shim, tool catalogue, and skill paths)
UI classification: none

## Outcome

An agent can supply file content to an HTML `<input type="file">` over MCP.
Soundcheck's manifest comparison input (`LibraryManifestSettings.svelte`,
which reads `input.files?.[0]` then `file.text()`) works for an agent without
the paste-JSON workaround, after consumer adoption.

## Context and confirmed decisions

Operator direction 2026-09-26: the agent supplies bytes inline; Longhorn never
reads a filesystem path for this tool. Soundcheck is the live acceptance
target, consumer-owned.

Agent `click` dispatches untrusted events, so clicking a file input opens no
panel and sets no files today. A file input needs real `File` objects, not a
path, so the pending-selection seam does not fit. The shim can build a
`DataTransfer`, add `File`s, assign `input.files`, and dispatch `input` and
`change` — the same shape as `type`.

## Work

1. Add `set_file_input { ref, webview?, files: [{ name, mediaType?,
   contentBase64 }] }` to the tool catalogue in `longhorn-agent-control`,
   with typed validation: non-empty names, valid base64, decoded total no
   more than 8 MiB, and at least one file.
2. In the shim, resolve the ref in the targeted webview; require an
   `<input type="file">`; reject more than one file unless `multiple`; check
   each file against the input's `accept` list (extensions and media types,
   including wildcards); then assign `files` and dispatch `input` and
   `change`. Mark agent origin as the other input tools do.
3. Wire it through the Tauri host like the other semantic tools, including
   child-webview targeting rules. It is not `evaluate`; a packaged
   `agent-control` build answers it.
4. Document it in the composition guide, crate README, and skill tool table
   (the skill check locks the table to the catalogue).

## Acceptance and review oracle

| Invariant | Required proof |
| --- | --- |
| Input receives files | shim fixture: `files` set, `input` and `change` fire, `file.text()` returns the supplied content, name and type preserved |
| Bounds hold | typed failures: non-file target, stale ref, closed child webview, `multiple` violated, `accept` mismatch, bad base64, over 8 MiB, empty list |
| No path authority | the tool has no path parameter; review confirms no filesystem read in the route |
| Packaged boundary holds | works without `agent-control-evaluate`; absent from a no-feature build per release-absence scan |
| Catalogue agrees | `effigy check:agent-control-skill` passes with the new tool |
| Consumer adoption (after merge, consumer-owned) | Soundcheck manifest compare loads a file supplied over MCP, unfocused |

Run focused agent-control Rust and TypeScript checks, the skill and
release-absence checks, then `effigy qa`.

## Dispatch manifest

- **Capability:** Rust/TypeScript, MCP tool and in-page shim; no UI design.
- **Owned paths:** `crates/longhorn-agent-control/src/{tools,handler}.rs`
  and tool tests, `crates/longhorn-tauri-agent-control/**`,
  `packages/longhorn/src/agent-control/**`,
  `packages/longhorn/tests/agent-control/**`, `skills/agent-control/**`,
  `docs/guides/agent-control-composition.md`, the release-absence and skill
  scripts, regenerated bindings / `docs/reference/api-surface.md`.
- **Reserved closeout:** as g02.047.
- **Concurrency:** serial after g02.047. Independent of g02.049.
- **Completion:** all fixture rows pass; the Soundcheck row is recorded as
  pending consumer adoption. No release is implied.
- **Escalation:** Chatterbox for a larger cap, a path-reading variant, or
  any need to open an OS panel.

## Stop conditions

Stop if WKWebView refuses `files` assignment from a constructed
`DataTransfer`, if `change` handlers cannot read the content, or if the route
would need filesystem access. Return the finding with the evidence to
Chatterbox.
