import {
  assertValidCommandAvailabilitySnapshot,
  assertValidCommandCatalogue,
  assertValidCommandKeymapCommit,
  assertValidCommandKeymapLoadOutcome,
  assertValidCommandKeymapMutationResult,
  assertValidCommandKeymapPreview,
  assertValidCommandKeymapPreviewResult,
  assertValidCommandKeymapReset,
} from "./validation.ts";
import type {
  CommandAvailabilitySnapshot,
  CommandCatalogueSnapshot,
  CommandKeymapCommit,
  CommandKeymapLoadOutcome,
  CommandKeymapMutationResult,
  CommandKeymapPreview,
  CommandKeymapPreviewResult,
  CommandKeymapReset,
} from "./generated/protocol.ts";
import type {
  CommandExecutionIntent,
  CommandExecutionOutcome,
  CommandPorts,
  CommandUnlisten,
} from "./ports.ts";

export interface CommandClientLoad {
  readonly catalogue: CommandCatalogueSnapshot;
  readonly keymap: CommandKeymapLoadOutcome;
  readonly availability: CommandAvailabilitySnapshot;
}

export class CommandClient {
  readonly #ports: CommandPorts;

  constructor(ports: CommandPorts) {
    this.#ports = ports;
  }

  nextRequestId(): string {
    return this.#ports.nextRequestId();
  }

  async load(): Promise<CommandClientLoad> {
    const [catalogue, keymap, availability] = await Promise.all([
      this.#ports.catalogue.load(),
      this.#ports.keymap.load(),
      this.#ports.availability.load(),
    ]);
    assertValidCommandCatalogue(catalogue);
    assertValidCommandKeymapLoadOutcome(keymap);
    assertValidCommandAvailabilitySnapshot(availability);
    return { catalogue, keymap, availability };
  }

  async listen(invalidate: () => void): Promise<readonly CommandUnlisten[]> {
    const results = await Promise.allSettled(
      [
        this.#ports.catalogue,
        this.#ports.keymap,
        this.#ports.availability,
      ].flatMap((port) =>
        port.listen === undefined ? [] : [port.listen(invalidate)],
      ),
    );
    const unlisten = results.flatMap((result) =>
      result.status === "fulfilled" ? [result.value] : [],
    );
    const failure = results.find(
      (result): result is PromiseRejectedResult =>
        result.status === "rejected",
    );
    if (failure !== undefined) {
      await Promise.allSettled(unlisten.map((dispose) => dispose()));
      throw failure.reason;
    }
    return unlisten;
  }

  execute(intent: CommandExecutionIntent): Promise<CommandExecutionOutcome> {
    return this.#ports.executor.execute(intent);
  }

  async preview(
    request: CommandKeymapPreview,
  ): Promise<CommandKeymapPreviewResult> {
    assertValidCommandKeymapPreview(request);
    const preview = this.#ports.keymap.preview;
    if (preview === undefined) throw new CommandKeymapReadOnlyError();
    const result = await preview(request);
    assertValidCommandKeymapPreviewResult(result);
    return result;
  }

  async commit(
    request: CommandKeymapCommit,
  ): Promise<CommandKeymapMutationResult> {
    assertValidCommandKeymapCommit(request);
    const commit = this.#ports.keymap.commit;
    if (commit === undefined) throw new CommandKeymapReadOnlyError();
    const result = await commit(request);
    assertValidCommandKeymapMutationResult(result);
    return result;
  }

  async reset(
    request: CommandKeymapReset,
  ): Promise<CommandKeymapMutationResult> {
    assertValidCommandKeymapReset(request);
    const reset = this.#ports.keymap.reset;
    if (reset === undefined) throw new CommandKeymapReadOnlyError();
    const result = await reset(request);
    assertValidCommandKeymapMutationResult(result);
    return result;
  }
}

export class CommandKeymapReadOnlyError extends Error {
  constructor() {
    super("command keymap mutation is not composed");
    this.name = "CommandKeymapReadOnlyError";
  }
}
