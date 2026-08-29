<script lang="ts">
  import { Surface } from "@inflatable-cookie/poodle-svelte";
  import {
    nativeContentViewport,
    useNativeContentSession,
    type NativeContentSession,
  } from "@inflatable-cookie/longhorn-poodle-svelte/native-content";

  let { session }: { session: NativeContentSession } = $props();
  useNativeContentSession(() => session);
</script>

<Surface asRole="region" label="Browser panel" padding="none">
  <div class="nucleus-browser-viewport" use:nativeContentViewport={session}></div>
</Surface>
