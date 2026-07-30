import type {
  CommandArguments,
  CommandAvailabilitySnapshot,
  CommandCatalogueSnapshot,
  CommandInvocation,
  CommandKeymapCommit,
  CommandKeymapLoadOutcome,
  CommandKeymapMutationResult,
  CommandKeymapPreview,
  CommandKeymapPreviewResult,
  CommandKeymapReset,
  CommandRequestId,
} from "./generated/protocol.ts";

export type CommandUnlisten = () => void | Promise<void>;

export interface CommandSnapshotPort<Snapshot> {
  load(): Promise<Snapshot>;
  listen?(invalidate: () => void): CommandUnlisten | Promise<CommandUnlisten>;
}

export interface CommandCataloguePort
  extends CommandSnapshotPort<CommandCatalogueSnapshot> {}

export interface CommandKeymapPort
  extends CommandSnapshotPort<CommandKeymapLoadOutcome> {
  preview?(request: CommandKeymapPreview): Promise<CommandKeymapPreviewResult>;
  commit?(request: CommandKeymapCommit): Promise<CommandKeymapMutationResult>;
  reset?(request: CommandKeymapReset): Promise<CommandKeymapMutationResult>;
}

export interface CommandAvailabilityPort
  extends CommandSnapshotPort<CommandAvailabilitySnapshot> {}

export type CommandExecutionSource =
  | "palette"
  | "keyboard"
  | "menu"
  | "help"
  | "consumer";

export interface CommandExecutionIntent {
  readonly requestId: CommandRequestId;
  readonly registryGeneration: number;
  readonly observedContextRevision: number;
  readonly invocation: CommandInvocation;
  readonly source: CommandExecutionSource;
}

export type CommandExecutionOutcome =
  | { readonly status: "succeeded"; readonly evidence?: unknown }
  | {
      readonly status:
        | "unknownCommand"
        | "staleRegistry"
        | "invalidArguments"
        | "unavailable"
        | "unauthorized"
        | "cancelled"
        | "rejected"
        | "failed"
        | "indeterminate";
      readonly evidence?: unknown;
    };

export interface CommandExecutorPort {
  execute(intent: CommandExecutionIntent): Promise<CommandExecutionOutcome>;
}

export interface CommandPorts {
  readonly catalogue: CommandCataloguePort;
  readonly keymap: CommandKeymapPort;
  readonly availability: CommandAvailabilityPort;
  readonly executor: CommandExecutorPort;
  readonly nextRequestId: () => CommandRequestId;
}

export interface CommandDispatch {
  readonly commandId: string;
  readonly arguments?: CommandArguments;
  readonly source: CommandExecutionSource;
}
