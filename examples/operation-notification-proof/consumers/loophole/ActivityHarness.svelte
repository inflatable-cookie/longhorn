<script lang="ts">
  import { NotificationPanel, NotificationToastHost } from "@inflatable-cookie/longhorn-notifications/poodle";
  import { useNotificationSession, type NotificationSession } from "@inflatable-cookie/longhorn-notifications/svelte";
  import { OperationPanel } from "@inflatable-cookie/longhorn-operation/poodle";
  import { useOperationSession, type OperationSession } from "@inflatable-cookie/longhorn-operation/svelte";

  let { operations, notifications }: { operations: OperationSession; notifications: NotificationSession } = $props();
  useOperationSession(operations);
  useNotificationSession(notifications);
</script>

<OperationPanel session={operations} title="Render queue" />
<NotificationPanel session={notifications} title="Activity" />
<NotificationToastHost session={notifications} autoDismissMs={1000} stickyTones={[]} />
