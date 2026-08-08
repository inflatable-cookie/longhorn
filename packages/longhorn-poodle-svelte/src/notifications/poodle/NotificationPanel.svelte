<script lang="ts">
  import { Button, Callout, InlineListSection, Stack, StatusIndicator } from "@inflatable-cookie/poodle-svelte";
  import type { Snippet } from "svelte";

  import type { NotificationRecordProjection } from "@inflatable-cookie/longhorn/notifications/protocol";
  import type { NotificationSession } from "../svelte/session.svelte.ts";
  import { notificationStatusLabel, notificationStatusTone } from "./projectors.ts";

  let {
    session,
    title = "Notifications",
    emptyMessage = "No notifications.",
    detail,
  }: {
    session: NotificationSession;
    title?: string;
    emptyMessage?: string;
    detail?: Snippet<[NotificationRecordProjection]>;
  } = $props();

  const failure = $derived(session.status.kind === "failed" ? (session.status.error instanceof Error ? session.status.error.message : String(session.status.error)) : null);

  function toggle(record: NotificationRecordProjection): void {
    session.select(session.selectedNotificationId === record.notificationId ? undefined : record.notificationId);
    if (record.readState === "unseen") void session.markSeen(record.notificationId).catch(() => undefined);
  }
</script>

<Stack gap="md" ariaLabel={title}>
  {#if failure}
    <Callout tone="danger" title="Notifications unavailable" message={failure} announceMode="polite" />
  {:else}
    {#if session.commandRejection}
      <Callout tone="warning" title="Notification action rejected" message={session.commandRejection.rejection.detail} announceMode="polite" />
    {:else if session.commandFailure}
      <Callout tone="danger" title="Notification action failed" message={session.commandFailure.error instanceof Error ? session.commandFailure.error.message : String(session.commandFailure.error)} announceMode="polite" />
    {/if}

    <InlineListSection {title} items={session.records} count={session.snapshot?.retainedCount ?? session.records.length} {emptyMessage} framed={false}>
      {#snippet item(notification)}
        <Stack gap="sm">
          <Stack direction="row" gap="sm" justify="between">
            <Button variant={session.selectedNotificationId === notification.notificationId ? "primary" : "secondary"} pressed={session.selectedNotificationId === notification.notificationId} onClick={() => toggle(notification)}>
              {notification.draft.title}
            </Button>
            <StatusIndicator status={notificationStatusTone(notification.draft.severity)} label={notificationStatusLabel(notification)} />
          </Stack>
          <span>{notification.draft.summary}</span>
          <Stack direction="row" gap="sm">
            {#each notification.draft.actions as action (action.referenceId)}
              <Button variant="ghost" loading={session.isPending(notification.notificationId, "action")} disabled={session.isPending(notification.notificationId, "action")} onClick={() => void session.invokeAction(notification.notificationId, action.referenceId).catch(() => undefined)}>{action.label}</Button>
            {/each}
            <Button variant="ghost" loading={session.isPending(notification.notificationId, "dismiss")} disabled={session.isPending(notification.notificationId, "dismiss")} onClick={() => void session.dismiss(notification.notificationId).catch(() => undefined)}>Dismiss</Button>
          </Stack>
          {#if detail && session.selectedNotificationId === notification.notificationId}
            {@render detail(notification)}
          {/if}
        </Stack>
      {/snippet}
    </InlineListSection>

    {#if session.hasMore}
      <Button variant="secondary" onClick={() => void session.loadMore().catch(() => undefined)}>Load older</Button>
    {/if}
  {/if}
</Stack>
