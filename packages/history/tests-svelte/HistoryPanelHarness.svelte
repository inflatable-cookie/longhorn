<script lang="ts">
  import { HistoryPanel } from "../src/poodle.ts";
  import { useHistorySession, type HistorySession } from "../src/svelte.ts";

  let { session }: { session: HistorySession } = $props();
  useHistorySession(session);
</script>

<HistoryPanel {session} />
