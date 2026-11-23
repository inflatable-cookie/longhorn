import type {
  NotificationChangedEvent,
  NotificationMutationCommand,
  NotificationRequestId,
  NotificationSnapshotQuery,
  NotificationSnapshotResponse,
  NotificationMutationResult,
} from "./generated/protocol.ts";

export type NotificationUnlisten = () => void | Promise<void>;

export interface NotificationPort {
  snapshot(query: NotificationSnapshotQuery): Promise<unknown>;
  mutate(command: NotificationMutationCommand): Promise<unknown>;
  listen?:
    | ((listener: (event: unknown) => void) => NotificationUnlisten | Promise<NotificationUnlisten>)
    | undefined;
  nextRequestId(): NotificationRequestId;
}

export interface CheckedNotificationPort {
  snapshot(query: NotificationSnapshotQuery): Promise<NotificationSnapshotResponse>;
  mutate(command: NotificationMutationCommand): Promise<NotificationMutationResult>;
  listen?(listener: (event: NotificationChangedEvent) => void): Promise<NotificationUnlisten>;
  nextRequestId(): NotificationRequestId;
}
