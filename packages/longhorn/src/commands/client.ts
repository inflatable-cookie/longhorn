import { attempt, disposeAll } from "../core/listeners.ts";
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
  CommandSnapshotPort,
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

  async listen(invalidate: () => void): Promise<ReadonlyArray<CommandUnlisten>> {
    const results = await Promise.allSettled(
      [this.#ports.catalogue, this.#ports.keymap, this.#ports.availability].flatMap((port) =>
        registrationOf(port, invalidate),
      ),
    );
    const unlisten = results.flatMap((result) =>
      result.status === "fulfilled" ? [result.value] : [],
    );
    const failure = results.find(
      (result): result is PromiseRejectedResult => result.status === "rejected",
    );
    if (failure !== undefined) {
      await disposeAll(unlisten);
      throw failure.reason;
    }
    return unlisten;
  }

  execute(intent: CommandExecutionIntent): Promise<CommandExecutionOutcome> {
    return this.#ports.executor.execute(intent);
  }

  async preview(request: CommandKeymapPreview): Promise<CommandKeymapPreviewResult> {
    assertValidCommandKeymapPreview(request);
    const keymap = this.#ports.keymap;
    if (keymap.preview === undefined) {
      throw new CommandKeymapReadOnlyError();
    }
    const result = await keymap.preview(request);
    assertValidCommandKeymapPreviewResult(result);
    return result;
  }

  async commit(request: CommandKeymapCommit): Promise<CommandKeymapMutationResult> {
    assertValidCommandKeymapCommit(request);
    const keymap = this.#ports.keymap;
    if (keymap.commit === undefined) {
      throw new CommandKeymapReadOnlyError();
    }
    const result = await keymap.commit(request);
    assertValidCommandKeymapMutationResult(result);
    return result;
  }

  async reset(request: CommandKeymapReset): Promise<CommandKeymapMutationResult> {
    assertValidCommandKeymapReset(request);
    const keymap = this.#ports.keymap;
    if (keymap.reset === undefined) {
      throw new CommandKeymapReadOnlyError();
    }
    const result = await keymap.reset(request);
    assertValidCommandKeymapMutationResult(result);
    return result;
  }
}

/**
 * Starts one port's listener inside a promise, so its registration joins the
 * settled set whether it returns, resolves or throws synchronously.
 */
function registrationOf(
  port: CommandSnapshotPort<unknown>,
  invalidate: () => void,
): Array<Promise<CommandUnlisten>> {
  const listen = port.listen?.bind(port);
  return listen === undefined ? [] : [attempt(() => listen(invalidate))];
}

export class CommandKeymapReadOnlyError extends Error {
  constructor() {
    super("command keymap mutation is not composed");
    this.name = "CommandKeymapReadOnlyError";
  }
}
