<script lang="ts">
  // A consumer's worth of markup, and no more.
  //
  // This used to render the `HistoryPanel` this package shipped. The panel is
  // gone -- Poodle's HistoryCenter is the history surface now, and a framework
  // that also ships one competes with the design system -- but these tests were
  // never about the panel. They are about the session: that two mounts stay
  // independent, that listeners are torn down, that filtering narrows entries
  // and that acting on one drives a revision.
  //
  // So the harness renders the controls the tests reach for, straight off the
  // session. The queries are unchanged, which is the point: dropping the panel
  // did not cost a single assertion.
  import {
    useHistorySession,
    type HistorySession,
  } from "../../src/history/svelte.ts";

  let { session }: { session: HistorySession } = $props();
  useHistorySession(session);

  const undoLabel = $derived(
    session.snapshot?.summary.nextUndoLabel
      ? `Undo ${session.snapshot.summary.nextUndoLabel}`
      : "Undo",
  );
</script>

<button type="button" disabled={!session.canUndo} onclick={() => void session.undo()}>
  {undoLabel}
</button>

<input
  type="search"
  aria-label="Filter history"
  value={session.filter}
  oninput={(event) => {
    session.setFilter((event.currentTarget as HTMLInputElement).value);
  }}
/>

<ul>
  {#each session.entries as entry (entry.entryId)}
    <li>
      <button
        type="button"
        disabled={session.navigationPending}
        onclick={() => {
          if (entry.position !== "current") void session.checkout(entry.entryId);
        }}
      >
        {entry.label}
      </button>
    </li>
  {/each}
</ul>
