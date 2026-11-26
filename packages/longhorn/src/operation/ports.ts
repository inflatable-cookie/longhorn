import type { Unlisten } from "../core/transport.ts";
import type {
  OperationCancellationCommand,
  OperationChangedEvent,
  OperationMutationCommand,
  OperationRequestId,
  OperationSnapshotQuery,
  OperationSnapshotResponse,
  OperationMutationResult,
  OperationCancellationResult,
} from "./generated/protocol.ts";

export type OperationUnlisten = Unlisten;

export interface OperationPort {
  snapshot(query: OperationSnapshotQuery): Promise<unknown>;
  mutate(command: OperationMutationCommand): Promise<unknown>;
  cancel(command: OperationCancellationCommand): Promise<unknown>;
  listen?:
    | ((listener: (event: unknown) => void) => OperationUnlisten | Promise<OperationUnlisten>)
    | undefined;
  nextRequestId(): OperationRequestId;
}

export interface CheckedOperationPort {
  snapshot(query: OperationSnapshotQuery): Promise<OperationSnapshotResponse>;
  mutate(command: OperationMutationCommand): Promise<OperationMutationResult>;
  cancel(command: OperationCancellationCommand): Promise<OperationCancellationResult>;
  listen?(listener: (event: OperationChangedEvent) => void): Promise<OperationUnlisten>;
  nextRequestId(): OperationRequestId;
}
