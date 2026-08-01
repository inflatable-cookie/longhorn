<script lang="ts">
  import { NotificationPanel, NotificationToastHost } from "@longhorn/notifications/poodle";
  import { useNotificationSession, type NotificationSession } from "@longhorn/notifications/svelte";
  import { OperationPanel } from "@longhorn/operation/poodle";
  import { useOperationSession, type OperationSession } from "@longhorn/operation/svelte";

  let { operations, notifications }: { operations: OperationSession; notifications: NotificationSession } = $props();
  useOperationSession(operations);
  useNotificationSession(notifications);
</script>

<OperationPanel session={operations} title="Render queue" />
<NotificationPanel session={notifications} title="Activity" />
<NotificationToastHost session={notifications} autoDismissMs={1000} stickyTones={[]} />
