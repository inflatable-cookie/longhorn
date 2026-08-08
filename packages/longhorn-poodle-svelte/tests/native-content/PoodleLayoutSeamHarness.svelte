<script lang="ts">
  import { Surface } from "@inflatable-cookie/poodle-svelte";

  import {
    nativeContentViewport,
    useNativeContentSession,
    type NativeContentSession,
  } from "../../src/native-content/index.ts";

  let {
    session,
    mechanism,
  }: {
    session: NativeContentSession;
    mechanism: "child_view" | "backing_surface";
  } = $props();

  useNativeContentSession(session);
</script>

<Surface
  asRole="region"
  label={mechanism === "child_view" ? "Native browser" : "Native renderer"}
  padding="none"
>
  <div
    data-testid="consumer-native-viewport"
    data-native-mechanism={mechanism}
    use:nativeContentViewport={session}
  ></div>
</Surface>
