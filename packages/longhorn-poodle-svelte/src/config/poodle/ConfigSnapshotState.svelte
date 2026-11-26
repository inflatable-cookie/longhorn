<script lang="ts">
  import { Button, Callout } from "@inflatable-cookie/poodle-svelte";

  interface Props {
    loadingTitle: string;
    error: string | null;
    retry: () => Promise<void>;
  }

  let { loadingTitle, error, retry }: Props = $props();
</script>

{#if error === null}
  <Callout tone="pending" title={loadingTitle} announceMode="polite" />
{:else}
  <Callout
    tone="danger"
    title="Settings could not be read"
    message={error}
    announceMode="assertive"
  >
    {#snippet actions()}
      <Button onClick={() => void retry()}>Retry</Button>
    {/snippet}
  </Callout>
{/if}
