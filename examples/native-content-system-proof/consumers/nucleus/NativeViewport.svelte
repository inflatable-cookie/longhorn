<script lang="ts">
  import { Surface } from "@poodle/svelte";
  import {
    nativeContentViewport,
    useNativeContentSession,
    type NativeContentSession,
  } from "@longhorn/native-content-svelte";

  let { session }: { session: NativeContentSession } = $props();
  useNativeContentSession(session);
</script>

<Surface asRole="region" label="Browser panel" padding="none">
  <div class="nucleus-browser-viewport" use:nativeContentViewport={session}></div>
</Surface>
