<script lang="ts">
  import {
    useClientState,
    type StartableClientState,
  } from "../../src/bindings/index.ts";

  let { state }: { state: StartableClientState } = $props();
  useClientState({
    start: () => state.start(),
    stop: () => state.stop(),
  });
</script>

<span data-testid="mounted">mounted</span>
