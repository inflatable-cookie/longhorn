import type { ConnectionFailureReporter } from "../core/connection.ts";
import type { Unlisten } from "../core/transport.ts";
import type {
  HistoryChangedEvent,
  HistoryNavigationCommand,
  HistoryNavigationResult,
  HistoryPageCommand,
  HistoryPageSnapshot,
  HistoryPlanId,
  HistorySnapshot,
} from "./generated/protocol.ts";

export type HistoryUnlisten = Unlisten;

export interface HistoryPort {
  snapshot(): Promise<unknown>;
  page(command: HistoryPageCommand): Promise<unknown>;
  navigate(command: HistoryNavigationCommand): Promise<unknown>;
  listen?:
    | ((listener: (event: unknown) => void) => HistoryUnlisten | Promise<HistoryUnlisten>)
    | undefined;
  nextPlanId(): HistoryPlanId;
}

export interface CheckedHistoryPort {
  snapshot(): Promise<HistorySnapshot>;
  page(command: HistoryPageCommand): Promise<HistoryPageSnapshot>;
  navigate(command: HistoryNavigationCommand): Promise<HistoryNavigationResult>;
  listen?(
    listener: (event: HistoryChangedEvent) => void,
    onFailure?: ConnectionFailureReporter,
  ): Promise<HistoryUnlisten>;
  nextPlanId(): HistoryPlanId;
}
