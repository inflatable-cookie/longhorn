<script lang="ts">
  import { HistoryPanel } from "@longhorn/history/poodle";
  import { useHistorySession } from "@longhorn/history/svelte";
  import type { HistorySession } from "@longhorn/history/svelte";

  let { session }: { session: HistorySession } = $props();
  useHistorySession(session);
</script>

<HistoryPanel {session} title="Loophole history" />
