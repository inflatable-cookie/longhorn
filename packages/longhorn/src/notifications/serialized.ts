import { wireCopy } from "../core/wire-copy.ts";
import type {
  NotificationMutationCommand,
  NotificationSnapshotQuery,
} from "./generated/protocol.ts";
import type { NotificationPort, NotificationUnlisten } from "./ports.ts";

export class SerializedNotificationPort implements NotificationPort {
  constructor(readonly inner: NotificationPort) {}

  async snapshot(query: NotificationSnapshotQuery): Promise<unknown> {
    return wireCopy(await this.inner.snapshot(wireCopy(query)));
  }

  async mutate(command: NotificationMutationCommand): Promise<unknown> {
    return wireCopy(await this.inner.mutate(wireCopy(command)));
  }

  async listen(listener: (event: unknown) => void): Promise<NotificationUnlisten> {
    if (this.inner.listen === undefined) {
      return () => {};
    }
    return this.inner.listen((event) => listener(wireCopy(event)));
  }

  nextRequestId(): string {
    return this.inner.nextRequestId();
  }
}
