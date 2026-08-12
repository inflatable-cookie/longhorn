import type {
  ForkBranchPageCommand,
  ForkBranchPageSnapshot,
  ForkChangedEvent,
  ForkContinuationPageCommand,
  ForkContinuationPageSnapshot,
  ForkDeleteContinuationCommand,
  ForkPruneCommand,
  ForkPruneResult,
  ForkNavigationCommand,
  ForkNavigationResult,
  ForkPathPageCommand,
  ForkPathPageSnapshot,
  ForkRemovalReceiptProjection,
  ForkSnapshot,
  HistoryPlanId,
} from "./generated/protocol.ts";

export type ForkHistoryUnlisten = () => void | Promise<void>;

export interface ForkHistoryPort {
  snapshot(): Promise<unknown>;
  path(command: ForkPathPageCommand): Promise<unknown>;
  branches(command: ForkBranchPageCommand): Promise<unknown>;
  continuations(command: ForkContinuationPageCommand): Promise<unknown>;
  deleteContinuation(command: ForkDeleteContinuationCommand): Promise<unknown>;
  prune(command: ForkPruneCommand): Promise<unknown>;
  navigate(command: ForkNavigationCommand): Promise<unknown>;
  listen?(listener: (event: unknown) => void): ForkHistoryUnlisten | Promise<ForkHistoryUnlisten>;
  nextPlanId(): HistoryPlanId;
}

export interface CheckedForkHistoryPort {
  snapshot(): Promise<ForkSnapshot>;
  path(command: ForkPathPageCommand): Promise<ForkPathPageSnapshot>;
  branches(command: ForkBranchPageCommand): Promise<ForkBranchPageSnapshot>;
  continuations(command: ForkContinuationPageCommand): Promise<ForkContinuationPageSnapshot>;
  deleteContinuation(command: ForkDeleteContinuationCommand): Promise<ForkRemovalReceiptProjection>;
  prune(command: ForkPruneCommand): Promise<ForkPruneResult>;
  navigate(command: ForkNavigationCommand): Promise<ForkNavigationResult>;
  listen?(listener: (event: ForkChangedEvent) => void): Promise<ForkHistoryUnlisten>;
  nextPlanId(): HistoryPlanId;
}
