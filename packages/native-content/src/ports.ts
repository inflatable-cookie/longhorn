import type {
  NativeContentChangedEvent,
  NativeContentConnectRequest,
  NativeContentConnectResult,
  NativeContentContentSizeDecisionRequest,
  NativeContentContentSizeDecisionResult,
  NativeContentDesiredUpdateRequest,
  NativeContentDesiredUpdateResult,
  NativeContentRequestId,
  NativeContentSnapshotRequest,
  NativeContentSnapshotResult,
} from "./generated/protocol.ts";

export type NativeContentUnlisten = () => void | Promise<void>;

export interface NativeContentPort {
  connect(request: NativeContentConnectRequest): Promise<unknown>;
  snapshot(request: NativeContentSnapshotRequest): Promise<unknown>;
  updateDesired(request: NativeContentDesiredUpdateRequest): Promise<unknown>;
  decideContentSize(
    request: NativeContentContentSizeDecisionRequest,
  ): Promise<unknown>;
  listen?(
    listener: (event: unknown) => void,
  ): NativeContentUnlisten | Promise<NativeContentUnlisten>;
  nextRequestId(): NativeContentRequestId;
}

export interface CheckedNativeContentPort {
  connect(
    request: NativeContentConnectRequest,
  ): Promise<NativeContentConnectResult>;
  snapshot(
    request: NativeContentSnapshotRequest,
  ): Promise<NativeContentSnapshotResult>;
  updateDesired(
    request: NativeContentDesiredUpdateRequest,
  ): Promise<NativeContentDesiredUpdateResult>;
  decideContentSize(
    request: NativeContentContentSizeDecisionRequest,
  ): Promise<NativeContentContentSizeDecisionResult>;
  listen?(
    listener: (event: NativeContentChangedEvent) => void,
  ): Promise<NativeContentUnlisten>;
  nextRequestId(): NativeContentRequestId;
}
