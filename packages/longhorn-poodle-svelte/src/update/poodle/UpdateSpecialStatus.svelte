<script lang="ts">
  import { Button, Callout } from "@inflatable-cookie/poodle-svelte";

  let {
    special,
    retry,
    canRetry,
    pending,
  }: {
    special: { kind: "unavailable" } | { kind: "malformed"; detail: string | null };
    retry: () => void;
    canRetry: boolean;
    pending: boolean;
  } = $props();
</script>

{#if special.kind === "unavailable"}
  <Callout tone="info" title="No update available for this system" />
{:else}
  <Callout tone="warning" title="The downloaded update was invalid" message={special.detail}>
    {#snippet actions()}
      {#if canRetry}
        <Button variant="secondary" disabled={pending} onClick={retry}>Download again</Button>
      {/if}
    {/snippet}
  </Callout>
{/if}
