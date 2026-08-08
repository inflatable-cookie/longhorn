<script lang="ts">
  import { Surface } from "@inflatable-cookie/poodle-svelte";
  import {
    nativeContentViewport,
    useNativeContentSession,
    type NativeContentSession,
  } from "@inflatable-cookie/longhorn-native-content-svelte";

  let { session }: { session: NativeContentSession } = $props();
  useNativeContentSession(session);
</script>

<Surface asRole="region" label="Scene viewport" padding="none">
  <div class="jetstream-scene-viewport" use:nativeContentViewport={session}></div>
</Surface>
