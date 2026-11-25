<script lang="ts">
  import { ToastHost } from "@inflatable-cookie/poodle-svelte";
  import type { ToastHostPlacement, ToastHostStoreItem } from "@inflatable-cookie/poodle-svelte";

  import type { NotificationSession } from "../svelte/session.svelte.ts";
  import { invokeToastAction, notificationToastStore } from "./toasts.ts";

  let {
    session,
    autoDismissMs = 6000,
    stickyTones = ["danger"],
    placement = "bottom-end",
  }: {
    session: NotificationSession;
    autoDismissMs?: number;
    stickyTones?: Array<ToastHostStoreItem["tone"]>;
    placement?: ToastHostPlacement;
  } = $props();

  const store = $derived(notificationToastStore(session));
</script>

{#key session}
  <ToastHost
    {store}
    {autoDismissMs}
    {stickyTones}
    {placement}
    onAction={(id) => invokeToastAction(session, id)}
  />
{/key}
