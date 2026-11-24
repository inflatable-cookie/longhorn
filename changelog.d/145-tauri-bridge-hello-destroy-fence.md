`longhorn-tauri-bridge` now tracks per-invocation `hello` ownership and
fences session insertion against interleaved `teardown` / `teardown_window`
calls, so a `hello` whose negotiation finishes after its caller window was
destroyed fails with `invalidSession` and cannot insert a stale session entry.
