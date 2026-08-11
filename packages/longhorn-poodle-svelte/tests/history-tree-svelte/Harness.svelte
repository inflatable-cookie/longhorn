<script lang="ts">
  // Rendered from the session directly. `ForkHistoryPanel` is gone -- Poodle's
  // HistoryCenter covers branches and entries, and this package no longer
  // ships a competing surface -- but the test was always about the session:
  // that it mounts, loads branches only when asked, and tears its listener
  // down. The one control it reaches for is reproduced here.
  import {
    useForkHistorySession,
    type ForkHistorySession,
  } from "../../src/history-tree/svelte.ts";

  let { session }: { session: ForkHistorySession } = $props();
  useForkHistorySession(session);
</script>

<button type="button" onclick={() => void session.loadBranches()}>Branches</button>
