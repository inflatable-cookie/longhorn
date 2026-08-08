import type {
  OperationCancellationCommand,
  OperationChangedEvent,
  OperationMutationCommand,
  OperationRequestId,
  OperationSnapshotQuery,
} from "./generated/protocol.ts";

export type OperationUnlisten = () => void | Promise<void>;

export interface OperationPort {
  snapshot(query: OperationSnapshotQuery): Promise<unknown>;
  mutate(command: OperationMutationCommand): Promise<unknown>;
  cancel(command: OperationCancellationCommand): Promise<unknown>;
  listen?(
    listener: (event: unknown) => void,
  ): OperationUnlisten | Promise<OperationUnlisten>;
  nextRequestId(): OperationRequestId;
}

export interface CheckedOperationPort {
  snapshot(query: OperationSnapshotQuery): Promise<import("./generated/protocol.ts").OperationSnapshotResponse>;
  mutate(command: OperationMutationCommand): Promise<import("./generated/protocol.ts").OperationMutationResult>;
  cancel(command: OperationCancellationCommand): Promise<import("./generated/protocol.ts").OperationCancellationResult>;
  listen?(
    listener: (event: OperationChangedEvent) => void,
  ): Promise<OperationUnlisten>;
  nextRequestId(): OperationRequestId;
}
