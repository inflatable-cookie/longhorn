<script lang="ts">
  import { OperationPanel } from "../../src/operation/poodle.ts";
  import { useOperationSession, type OperationSession } from "../../src/operation/svelte.ts";

  let {
    session,
    shape,
  }: {
    session: OperationSession;
    shape: "soundcheck" | "loophole";
  } = $props();
  useOperationSession(session);
</script>

<OperationPanel
  {session}
  title={shape === "soundcheck" ? "Scan activity" : "Render queue"}
  activeTitle={shape === "soundcheck" ? "Scanning" : "Queued and rendering"}
  recentTitle={shape === "soundcheck" ? "Completed scans" : "Recent renders"}
>
  {#snippet detail(operation)}
    <p>{shape} detail for {operation.operationId}</p>
  {/snippet}
</OperationPanel>
