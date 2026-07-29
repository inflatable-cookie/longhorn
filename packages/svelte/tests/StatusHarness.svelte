<script lang="ts">
  import type {
    ClientStatus,
    StartableClientState,
  } from "../src/index.ts";

  let {
    state,
  }: {
    state: StartableClientState & { readonly status: ClientStatus };
  } = $props();
</script>

<span data-testid="status">{state.status.kind}</span>
