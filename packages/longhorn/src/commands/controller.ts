import { ControllerLifecycle } from "../core/controller-lifecycle.ts";
import { CommandClient } from "./client.ts";
import type {
  CommandInvocation,
  CommandKeymapMutationResult,
  CommandKeymapPatch,
  CommandKeymapSnapshot,
  CommandPlatform,
  CommandSearchHit,
} from "./generated/protocol.ts";
import type {
  CommandDispatch,
  CommandExecutionOutcome,
  CommandExecutionSource,
  CommandPorts,
} from "./ports.ts";
import { joinCommandState, projectCommandSurfaces, searchCommands } from "./projectors.ts";
import type { CommandJoinedState, CommandSurfaceProjection } from "./projectors.ts";

export type CommandControllerStatus =
  | { readonly kind: "idle" }
  | { readonly kind: "loading" }
  | { readonly kind: "ready" }
  | {
      readonly kind: "recovery";
      readonly recovery: {
        readonly code: string;
        readonly detail: string;
        readonly sourcePreserved: boolean;
      };
    }
  | { readonly kind: "unavailable"; readonly detail: string }
  | { readonly kind: "failed"; readonly error: unknown };

export type CommandMutationStatus =
  | { readonly kind: "idle" }
  | { readonly kind: "dirty" }
  | { readonly kind: "previewing" }
  | { readonly kind: "committing" }
  | { readonly kind: "saved" }
  | { readonly kind: "conflict"; readonly detail: string }
  | { readonly kind: "failed"; readonly error: unknown };

export interface CommandControllerOptions {
  readonly ports: CommandPorts;
  readonly platform: CommandPlatform;
  readonly search?: (
    records: CommandJoinedState["catalogue"]["commands"],
    query: string,
  ) => Promise<ReadonlyArray<CommandSearchHit>>;
}

export class CommandController {
  readonly #client: CommandClient;
  readonly #platform: CommandPlatform;
  readonly #search;
  readonly #observers = new Set<() => void>();
  #status: CommandControllerStatus = { kind: "idle" };
  #mutation: CommandMutationStatus = { kind: "idle" };
  #model: CommandJoinedState | undefined;
  #projection: CommandSurfaceProjection | undefined;
  #searchHits: ReadonlyArray<CommandSearchHit> = [];
  #query = "";
  #draft: CommandKeymapPatch | undefined;
  #loadRevision = 0;
  #mutationRevision = 0;
  #searchRevision = 0;
  readonly #lifecycle = new ControllerLifecycle();

  constructor(options: CommandControllerOptions) {
    this.#client = new CommandClient(options.ports);
    this.#platform = options.platform;
    this.#search = options.search ?? (async (records, query) => searchCommands(records, query));
  }

  get status(): CommandControllerStatus {
    return this.#status;
  }

  get mutation(): CommandMutationStatus {
    return this.#mutation;
  }

  get model(): CommandJoinedState | undefined {
    return this.#model;
  }

  get projection(): CommandSurfaceProjection | undefined {
    return this.#projection;
  }

  get query(): string {
    return this.#query;
  }

  get searchHits(): ReadonlyArray<CommandSearchHit> {
    return this.#searchHits;
  }

  get draft(): CommandKeymapPatch | undefined {
    return this.#draft;
  }

  get dirty(): boolean {
    return this.#draft !== undefined;
  }

  observe(observer: () => void): () => void {
    this.#observers.add(observer);
    return () => this.#observers.delete(observer);
  }

  start(): Promise<void> {
    return this.#lifecycle.start({
      loading: () => this.#setStatus({ kind: "loading" }),
      connect: async ({ isCurrent, own }) => {
        const unlisten = await this.#client.listen(() => {
          if (isCurrent()) {
            void this.refresh();
          }
        });
        await own(...unlisten);
        if (isCurrent()) {
          await this.refresh();
        }
      },
      failed: (error) => this.#setStatus({ kind: "failed", error }),
    });
  }

  stop(): Promise<void> {
    return this.#lifecycle.stop(() => {
      this.#loadRevision += 1;
      this.#searchRevision += 1;
      this.#mutationRevision += 1;
      this.#model = undefined;
      this.#projection = undefined;
      this.#searchHits = [];
      this.#query = "";
      this.#draft = undefined;
      this.#mutation = { kind: "idle" };
      this.#setStatus({ kind: "idle" });
    }, "ignore");
  }

  async refresh(): Promise<void> {
    const revision = ++this.#loadRevision;
    try {
      const loaded = await this.#client.load();
      if (!this.#lifecycle.active || revision !== this.#loadRevision) {
        return;
      }
      if (loaded.keymap.status === "recovery") {
        this.#model = undefined;
        this.#projection = undefined;
        this.#setStatus({
          kind: "recovery",
          recovery: loaded.keymap.recovery,
        });
        return;
      }
      if (loaded.keymap.status === "unavailable") {
        this.#model = undefined;
        this.#projection = undefined;
        this.#setStatus({
          kind: "unavailable",
          detail: loaded.keymap.detail,
        });
        return;
      }
      this.#install(
        joinCommandState(loaded.catalogue, loaded.keymap.snapshot, loaded.availability),
      );
      this.#setStatus({ kind: "ready" });
      await this.search(this.#query);
    } catch (error) {
      if (this.#lifecycle.active && revision === this.#loadRevision) {
        this.#setStatus({ kind: "failed", error });
      }
    }
  }

  async search(query: string): Promise<ReadonlyArray<CommandSearchHit>> {
    this.#query = query;
    const model = this.#model;
    const revision = ++this.#searchRevision;
    if (model === undefined) {
      this.#searchHits = [];
      this.#notify();
      return [];
    }
    const hits = await this.#search(model.catalogue.commands, query);
    if (revision !== this.#searchRevision || model !== this.#model) {
      return hits;
    }
    const visible = new Set(this.#projection?.palette.map(({ id }) => id));
    this.#searchHits = hits.filter(({ record }) => visible.has(record.id));
    this.#notify();
    return this.#searchHits;
  }

  async dispatch(dispatch: CommandDispatch): Promise<CommandExecutionOutcome> {
    return this.dispatchInvocation(
      {
        commandId: dispatch.commandId,
        arguments: dispatch.arguments ?? {},
      },
      dispatch.source,
    );
  }

  async dispatchInvocation(
    invocation: CommandInvocation,
    source: CommandExecutionSource,
  ): Promise<CommandExecutionOutcome> {
    const model = this.#requiredModel();
    return this.#client.execute({
      requestId: this.#client.nextRequestId(),
      registryGeneration: model.catalogue.registryGeneration,
      observedContextRevision: model.availability.contextRevision,
      invocation,
      source,
    });
  }

  stageKeymapPatch(patch: CommandKeymapPatch): void {
    this.#mutationRevision += 1;
    this.#draft = patch;
    this.#mutation = { kind: "dirty" };
    this.#notify();
  }

  cancelKeymapDraft(): void {
    this.#mutationRevision += 1;
    this.#draft = undefined;
    this.#mutation = { kind: "idle" };
    this.#notify();
  }

  async applyKeymapDraft(): Promise<boolean> {
    const model = this.#requiredModel();
    const patch = this.#draft;
    if (patch === undefined) {
      return true;
    }
    const base = model.keymap;
    const operationRevision = ++this.#mutationRevision;
    this.#mutation = { kind: "previewing" };
    this.#notify();
    try {
      const preview = await this.#client.preview({
        registryGeneration: model.catalogue.registryGeneration,
        keymapRevision: base.state.revision,
        activePresetId: base.state.activePresetId,
        activePresetVersion: base.activePresetVersion,
        patch,
      });
      if (operationRevision !== this.#mutationRevision) {
        return false;
      }
      if (preview.status !== "accepted") {
        this.#applyMutationSnapshot(preview.snapshot);
        this.#mutation = {
          kind: "conflict",
          detail:
            preview.status === "rejected"
              ? preview.rejection.detail
              : "Keybindings changed elsewhere",
        };
        this.#notify();
        return false;
      }
      this.#mutation = { kind: "committing" };
      this.#notify();
      if (operationRevision !== this.#mutationRevision) {
        return false;
      }
      const result = await this.#client.commit({
        requestId: this.#client.nextRequestId(),
        evidence: preview.evidence,
        patch,
      });
      if (operationRevision !== this.#mutationRevision) {
        return false;
      }
      return this.#finishMutation(result);
    } catch (error) {
      if (operationRevision === this.#mutationRevision) {
        this.#mutation = { kind: "failed", error };
        this.#notify();
      }
      return false;
    }
  }

  async resetKeymap(): Promise<boolean> {
    const model = this.#requiredModel();
    const base = model.keymap;
    const operationRevision = ++this.#mutationRevision;
    this.#mutation = { kind: "committing" };
    this.#notify();
    if (operationRevision !== this.#mutationRevision) {
      return false;
    }
    try {
      const result = await this.#client.reset({
        requestId: this.#client.nextRequestId(),
        registryGeneration: model.catalogue.registryGeneration,
        keymapRevision: base.state.revision,
        activePresetId: base.state.activePresetId,
        activePresetVersion: base.activePresetVersion,
      });
      if (operationRevision !== this.#mutationRevision) {
        return false;
      }
      return this.#finishMutation(result);
    } catch (error) {
      if (operationRevision === this.#mutationRevision) {
        this.#mutation = { kind: "failed", error };
        this.#notify();
      }
      return false;
    }
  }

  #finishMutation(result: CommandKeymapMutationResult): boolean {
    this.#applyMutationSnapshot(result.snapshot);
    if (result.status === "applied") {
      this.#draft = undefined;
      this.#mutation = { kind: "saved" };
      this.#notify();
      return true;
    }
    this.#mutation = {
      kind: "conflict",
      detail:
        result.status === "rejected" ? result.rejection.detail : "Keybindings changed elsewhere",
    };
    this.#notify();
    return false;
  }

  #applyMutationSnapshot(snapshot: CommandKeymapSnapshot): void {
    const model = this.#model;
    if (model === undefined) {
      return;
    }
    if (snapshot.registryGeneration !== model.catalogue.registryGeneration) {
      void this.refresh();
      return;
    }
    this.#install(joinCommandState(model.catalogue, snapshot, model.availability));
  }

  #install(model: CommandJoinedState): void {
    const current = this.#model;
    if (
      current !== undefined &&
      (model.catalogue.registryGeneration < current.catalogue.registryGeneration ||
        (model.catalogue.registryGeneration === current.catalogue.registryGeneration &&
          (model.keymap.state.revision < current.keymap.state.revision ||
            model.availability.contextRevision < current.availability.contextRevision)))
    ) {
      return;
    }
    const projection = projectCommandSurfaces(model, this.#platform);
    this.#model = model;
    this.#projection = projection;
  }

  #requiredModel(): CommandJoinedState {
    if (this.#model === undefined) {
      throw new CommandControllerUnavailableError();
    }
    return this.#model;
  }

  #setStatus(status: CommandControllerStatus): void {
    this.#status = status;
    this.#notify();
  }

  #notify(): void {
    for (const observer of this.#observers) {
      observer();
    }
  }
}

export class CommandControllerUnavailableError extends Error {
  constructor() {
    super("command controller has no joined authoritative state");
    this.name = "CommandControllerUnavailableError";
  }
}
