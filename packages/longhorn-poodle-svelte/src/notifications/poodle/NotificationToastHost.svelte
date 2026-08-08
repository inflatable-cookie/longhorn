<script lang="ts">
  import { ToastHost, type ToastHostPlacement, type ToastHostStore, type ToastHostStoreItem } from "@inflatable-cookie/poodle-svelte";
  import { readable } from "svelte/store";

  import type { NotificationSession } from "../svelte/session.svelte.ts";

  let {
    session,
    autoDismissMs = 6000,
    stickyTones = ["danger"],
    placement = "bottom-end",
  }: {
    session: NotificationSession;
    autoDismissMs?: number;
    stickyTones?: ToastHostStoreItem["tone"][];
    placement?: ToastHostPlacement;
  } = $props();

  const store: ToastHostStore = {
    toasts: readable<ToastHostStoreItem[]>([], (set) => {
      const sync = () => set(session.toasts.map((toast) => ({
        id: toast.id,
        title: toast.title,
        message: toast.description,
        tone: toast.tone,
        actionLabel: toast.action?.label ?? null,
      })));
      sync();
      return session.observe(sync);
    }),
    dismiss: (id) => session.dismissToast(id),
  };

  function act(id: string): void {
    const toast = session.toasts.find((candidate) => candidate.id === id);
    if (toast?.action !== undefined) void session.invokeAction(toast.notificationId, toast.action.referenceId).catch(() => undefined);
  }
</script>

<ToastHost {store} {autoDismissMs} {stickyTones} {placement} onAction={act} />
