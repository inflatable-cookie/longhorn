import type {
  ForkBranchPageCommand,
  ForkBranchPageSnapshot,
  ForkChangedEvent,
  ForkNavigationCommand,
  ForkNavigationResult,
  ForkPathPageCommand,
  ForkPathPageSnapshot,
  ForkSnapshot,
  HistoryPlanId,
} from "./generated/protocol.ts";

export type ForkHistoryUnlisten = () => void | Promise<void>;

export interface ForkHistoryPort {
  snapshot(): Promise<unknown>;
  path(command: ForkPathPageCommand): Promise<unknown>;
  branches(command: ForkBranchPageCommand): Promise<unknown>;
  navigate(command: ForkNavigationCommand): Promise<unknown>;
  listen?(listener: (event: unknown) => void): ForkHistoryUnlisten | Promise<ForkHistoryUnlisten>;
  nextPlanId(): HistoryPlanId;
}

export interface CheckedForkHistoryPort {
  snapshot(): Promise<ForkSnapshot>;
  path(command: ForkPathPageCommand): Promise<ForkPathPageSnapshot>;
  branches(command: ForkBranchPageCommand): Promise<ForkBranchPageSnapshot>;
  navigate(command: ForkNavigationCommand): Promise<ForkNavigationResult>;
  listen?(listener: (event: ForkChangedEvent) => void): Promise<ForkHistoryUnlisten>;
  nextPlanId(): HistoryPlanId;
}
