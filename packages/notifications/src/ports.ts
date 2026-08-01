import type {
  NotificationChangedEvent,
  NotificationMutationCommand,
  NotificationRequestId,
  NotificationSnapshotQuery,
} from "./generated/protocol.ts";

export type NotificationUnlisten = () => void | Promise<void>;

export interface NotificationPort {
  snapshot(query: NotificationSnapshotQuery): Promise<unknown>;
  mutate(command: NotificationMutationCommand): Promise<unknown>;
  listen?(listener: (event: unknown) => void): NotificationUnlisten | Promise<NotificationUnlisten>;
  nextRequestId(): NotificationRequestId;
}

export interface CheckedNotificationPort {
  snapshot(query: NotificationSnapshotQuery): Promise<import("./generated/protocol.ts").NotificationSnapshotResponse>;
  mutate(command: NotificationMutationCommand): Promise<import("./generated/protocol.ts").NotificationMutationResult>;
  listen?(listener: (event: NotificationChangedEvent) => void): Promise<NotificationUnlisten>;
  nextRequestId(): NotificationRequestId;
}
