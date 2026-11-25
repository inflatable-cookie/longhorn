import type { ToastHostStore, ToastHostStoreItem } from "@inflatable-cookie/poodle-svelte";
import { readable } from "svelte/store";

import type { NotificationSession } from "../svelte/session.svelte.ts";

export function projectToasts(session: NotificationSession) {
  return session.toasts.map(({ action, description, id, title, tone }) => ({
    id,
    title,
    message: description,
    tone,
    actionLabel: action?.label ?? null,
  }));
}

export function notificationToastStore(session: NotificationSession): ToastHostStore {
  return {
    toasts: readable<Array<ToastHostStoreItem>>([], (set) => {
      const sync = () => set(projectToasts(session));
      sync();
      return session.observe(sync);
    }),
    dismiss: (id) => session.dismissToast(id),
  };
}

export function invokeToastAction(session: NotificationSession, id: string): void {
  const toast = session.toasts.find((candidate) => candidate.id === id);
  if (toast?.action !== undefined) {
    void session
      .invokeAction(toast.notificationId, toast.action.referenceId)
      .catch(() => undefined);
  }
}
