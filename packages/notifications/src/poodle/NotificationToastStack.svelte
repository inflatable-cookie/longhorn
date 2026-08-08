<script lang="ts">
  import { ToastStack, type ToastItem } from "@inflatable-cookie/poodle-svelte";

  import type { NotificationSession } from "../svelte/session.svelte.ts";

  let { session }: { session: NotificationSession } = $props();

  const items = $derived<ToastItem[]>(session.toasts.map((toast) => ({
    id: toast.id,
    title: toast.title,
    message: toast.description,
    tone: toast.tone,
    actionLabel: toast.action?.label ?? null,
  })));

  function act(id: string): void {
    const toast = session.toasts.find((candidate) => candidate.id === id);
    if (toast?.action !== undefined) void session.invokeAction(toast.notificationId, toast.action.referenceId).catch(() => undefined);
  }
</script>

<ToastStack {items} onDismiss={(id) => session.dismissToast(id)} onAction={act} />
