import fixtureValue from "../../../../fixtures/commands/protocol-v1.json";
import type {
  CommandAvailabilitySnapshot,
  CommandCatalogueSnapshot,
  CommandContextId,
  CommandEffectiveBinding,
  CommandKeyboardInput,
  CommandKeyboardMode,
  CommandKeyResolution,
  CommandKeymapLoadOutcome,
  CommandKeymapMutationResult,
  CommandKeymapPreviewResult,
  CommandKeymapSnapshot,
  CommandPlatform,
  CommandSearchHit,
  CommandShortcutRecord,
} from "../../src/commands/generated/protocol.ts";

export const fixture = fixtureValue as unknown as {
  protocolVersion: number;
  catalogue: CommandCatalogueSnapshot;
  requests: {
    preview: unknown;
    commit: unknown;
    reset: unknown;
  };
  events: {
    catalogueChanged: unknown;
    keymapChanged: unknown;
  };
  snapshots: Array<CommandKeymapSnapshot>;
  previewResults: Array<CommandKeymapPreviewResult>;
  loadOutcomes: Array<CommandKeymapLoadOutcome>;
  mutationResults: Array<CommandKeymapMutationResult>;
  semantics: {
    context: { revision: number; path: Array<CommandContextId> };
    search: Array<{ query: string; hits: Array<CommandSearchHit> }>;
    shortcuts: Array<{
      platform: CommandPlatform;
      commandId: string;
      shortcuts: Array<CommandShortcutRecord>;
    }>;
    keyboard: Array<{
      platform: CommandPlatform;
      input: CommandKeyboardInput;
      contextPath: Array<CommandContextId>;
      mode: CommandKeyboardMode;
      bindings: Array<CommandEffectiveBinding>;
      consumed: boolean;
      resolution: CommandKeyResolution;
    }>;
  };
  incompatibility: Record<string, unknown>;
};

export function availability(
  contextRevision = 1,
  registryGeneration = fixture.catalogue.registryGeneration,
): CommandAvailabilitySnapshot {
  return {
    registryGeneration,
    contextRevision,
    records: fixture.catalogue.commands.map(({ id }) => ({
      commandId: id,
      availability: { state: "available", reason: null },
    })),
  };
}
