<script lang="ts">
  import { HistoryPanel } from "../../src/history/poodle.ts";
  import { useHistorySession, type HistorySession } from "../../src/history/svelte.ts";

  let { session }: { session: HistorySession } = $props();
  useHistorySession(session);
</script>

<HistoryPanel {session} />
