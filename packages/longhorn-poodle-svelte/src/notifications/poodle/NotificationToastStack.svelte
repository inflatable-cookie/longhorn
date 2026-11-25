<script lang="ts">
  import { ToastStack } from "@inflatable-cookie/poodle-svelte";
  import type { NotificationSession } from "../svelte/session.svelte.ts";
  import { invokeToastAction, projectToasts } from "./toasts.ts";

  let { session }: { session: NotificationSession } = $props();
  const items = $derived(projectToasts(session));
</script>

<ToastStack
  {items}
  onDismiss={(id) => session.dismissToast(id)}
  onAction={(id) => invokeToastAction(session, id)}
/>
