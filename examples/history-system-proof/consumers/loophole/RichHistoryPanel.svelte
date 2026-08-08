<script lang="ts">
  import { HistoryPanel } from "@inflatable-cookie/longhorn-poodle-svelte/history/poodle";
  import { useHistorySession } from "@inflatable-cookie/longhorn-poodle-svelte/history/svelte";
  import type { HistorySession } from "@inflatable-cookie/longhorn-poodle-svelte/history/svelte";

  let { session }: { session: HistorySession } = $props();
  useHistorySession(session);
</script>

<HistoryPanel {session} title="Loophole history" />
