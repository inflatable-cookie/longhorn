import type {
  HistoryChangedEvent,
  HistoryNavigationCommand,
  HistoryNavigationResult,
  HistoryPageCommand,
  HistoryPageSnapshot,
  HistoryPlanId,
  HistorySnapshot,
} from "./generated/protocol.ts";

export type HistoryUnlisten = () => void | Promise<void>;

export interface HistoryPort {
  snapshot(): Promise<unknown>;
  page(command: HistoryPageCommand): Promise<unknown>;
  navigate(command: HistoryNavigationCommand): Promise<unknown>;
  listen?(
    listener: (event: unknown) => void,
  ): HistoryUnlisten | Promise<HistoryUnlisten>;
  nextPlanId(): HistoryPlanId;
}

export interface CheckedHistoryPort {
  snapshot(): Promise<HistorySnapshot>;
  page(command: HistoryPageCommand): Promise<HistoryPageSnapshot>;
  navigate(command: HistoryNavigationCommand): Promise<HistoryNavigationResult>;
  listen?(
    listener: (event: HistoryChangedEvent) => void,
  ): Promise<HistoryUnlisten>;
  nextPlanId(): HistoryPlanId;
}
