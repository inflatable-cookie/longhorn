<script lang="ts">
  // What a consumer composes for itself.
  //
  // This used to render `HistoryPanel` from longhorn-poodle-svelte. That panel
  // is gone: Poodle's HistoryCenter is the history surface, and the framework
  // no longer ships a competing one. The proof is better for it -- it now
  // exercises what a consumer actually does, which is bind its own controls to
  // the session rather than accept a pre-composed panel.
  import {
    useHistorySession,
    type HistorySession,
  } from "@inflatable-cookie/longhorn-poodle-svelte/history/svelte";

  let { session }: { session: HistorySession } = $props();
  useHistorySession(() => session);

  const undoLabel = $derived(
    session.snapshot?.summary.nextUndoLabel
      ? `Undo ${session.snapshot.summary.nextUndoLabel}`
      : "Undo",
  );
  const redoLabel = $derived(
    session.snapshot?.summary.nextRedoLabel
      ? `Redo ${session.snapshot.summary.nextRedoLabel}`
      : "Redo",
  );
</script>

<button type="button" disabled={!session.canUndo} onclick={() => void session.undo()}>
  {undoLabel}
</button>
<button type="button" disabled={!session.canRedo} onclick={() => void session.redo()}>
  {redoLabel}
</button>

<input
  type="search"
  aria-label="Filter history"
  value={session.filter}
  oninput={(event) => {
    session.setFilter((event.currentTarget as HTMLInputElement).value);
  }}
/>
