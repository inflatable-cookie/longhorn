<script lang="ts">
  import { NotificationPanel, NotificationToastHost } from "../../src/notifications/poodle.ts";
  import { useNotificationSession, type NotificationSession } from "../../src/notifications/svelte.ts";

  let { session, autoDismissMs = 6000 }: { session: NotificationSession; autoDismissMs?: number } = $props();
  useNotificationSession(() => session);
</script>

<NotificationPanel {session} />
<NotificationToastHost {session} {autoDismissMs} stickyTones={[]} />
