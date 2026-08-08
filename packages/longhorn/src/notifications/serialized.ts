import type { NotificationMutationCommand, NotificationSnapshotQuery } from "./generated/protocol.ts";
import type { NotificationPort, NotificationUnlisten } from "./ports.ts";

export class SerializedNotificationPort implements NotificationPort {
  constructor(readonly inner: NotificationPort) {}

  async snapshot(query: NotificationSnapshotQuery): Promise<unknown> {
    return clone(await this.inner.snapshot(clone(query)));
  }

  async mutate(command: NotificationMutationCommand): Promise<unknown> {
    return clone(await this.inner.mutate(clone(command)));
  }

  async listen(listener: (event: unknown) => void): Promise<NotificationUnlisten> {
    if (this.inner.listen === undefined) return () => {};
    return this.inner.listen((event) => listener(clone(event)));
  }

  nextRequestId(): string {
    return this.inner.nextRequestId();
  }
}

function clone<Value>(value: Value): Value {
  return JSON.parse(JSON.stringify(value)) as Value;
}
