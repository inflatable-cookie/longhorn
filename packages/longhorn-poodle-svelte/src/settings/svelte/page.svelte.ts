import type { SettingsClient } from "@inflatable-cookie/longhorn/settings";
import type {
  SettingsApplyUnitDefinition,
  SettingsApplyUnitId,
  SettingsEntryId,
  SettingsMutationResult,
  SettingsOpaqueValue,
  SettingsPageDefinition,
  SettingsRegistrySnapshot,
  SettingsRequestId,
} from "@inflatable-cookie/longhorn/settings/protocol";
import type { SettingsScopeState } from "./scope.svelte.ts";
import type { SettingsDraft, SettingsUnitStatus } from "./types.ts";
import {
  SettingsApplyUnitUnavailableError,
  SettingsDraftUnavailableError,
  SettingsMutationPendingError,
  SettingsPageApplyAmbiguityError,
  SettingsResetUnsupportedError,
  SettingsScopeUnavailableError,
} from "./errors.ts";

export interface SettingsPageSessionHost {
  readonly client: SettingsClient;
  registry(): SettingsRegistrySnapshot;
  scope(applyUnit: SettingsApplyUnitDefinition): SettingsScopeState;
  nextRequestId(): SettingsRequestId;
  onError(error: unknown): void;
}

export class SettingsPageSession {
  readonly #page: SettingsPageDefinition;
  readonly #host: SettingsPageSessionHost;
  #drafts = $state.raw<ReadonlyMap<SettingsApplyUnitId, SettingsDraft>>(
    new Map(),
  );
  #statuses = $state.raw<
    ReadonlyMap<SettingsApplyUnitId, SettingsUnitStatus>
  >(new Map());
  #generation = 0;

  constructor(page: SettingsPageDefinition, host: SettingsPageSessionHost) {
    this.#page = page;
    this.#host = host;
  }

  get dirty(): boolean {
    return this.#drafts.size > 0;
  }

  get busy(): boolean {
    return [...this.#statuses.values()].some(
      ({ kind }) => kind === "pending",
    );
  }

  get drafts(): readonly SettingsDraft[] {
    return [...this.#drafts.values()];
  }

  draft(applyUnitId: SettingsApplyUnitId): SettingsDraft | undefined {
    return this.#drafts.get(applyUnitId);
  }

  status(applyUnitId: SettingsApplyUnitId): SettingsUnitStatus {
    return this.#statuses.get(applyUnitId) ?? { kind: "idle" };
  }

  async change(
    applyUnitId: SettingsApplyUnitId,
    intent: SettingsOpaqueValue,
  ): Promise<void> {
    const unit = this.#applyUnit(applyUnitId);
    if (unit.timing === "immediate") {
      await this.#executeApply(unit, intent, false);
      return;
    }
    this.#drafts = new Map(this.#drafts).set(applyUnitId, {
      applyUnitId,
      intent,
    });
    this.#setStatus(applyUnitId, { kind: "idle" });
  }

  async apply(applyUnitId: SettingsApplyUnitId): Promise<void> {
    const unit = this.#applyUnit(applyUnitId);
    const draft = this.#drafts.get(applyUnitId);
    if (unit.timing !== "staged" || draft === undefined) {
      throw new SettingsDraftUnavailableError(applyUnitId);
    }
    await this.#executeApply(unit, draft.intent, true);
  }

  async applyOnlyDraft(): Promise<boolean> {
    if (this.#drafts.size !== 1) {
      if (this.#drafts.size === 0) {
        return true;
      }
      throw new SettingsPageApplyAmbiguityError(this.#drafts.size);
    }
    const draft = this.drafts[0]!;
    await this.apply(draft.applyUnitId);
    return this.status(draft.applyUnitId).kind === "saved";
  }

  cancel(): void {
    if (this.busy) {
      throw new SettingsMutationPendingError();
    }
    const affected = [...this.#drafts.keys()];
    this.#drafts = new Map();
    const statuses = new Map(this.#statuses);
    for (const applyUnitId of affected) {
      statuses.set(applyUnitId, { kind: "idle" });
    }
    this.#statuses = statuses;
  }

  async reset(
    applyUnitId: SettingsApplyUnitId,
    entryIds: readonly SettingsEntryId[],
  ): Promise<void> {
    const unit = this.#applyUnit(applyUnitId);
    this.#assertNotPending(applyUnitId);
    if (!unit.resetSupported) {
      throw new SettingsResetUnsupportedError(applyUnitId);
    }
    const registry = this.#host.registry();
    const scope = this.#host.scope(unit);
    const authority = scope.snapshot?.authority;
    if (authority === undefined) {
      throw new SettingsScopeUnavailableError(unit.scopeId);
    }
    const generation = this.#generation;
    this.#setStatus(applyUnitId, { kind: "pending" });
    try {
      const result = await this.#host.client.reset(registry, {
        protocolVersion: 1,
        requestId: this.#host.nextRequestId(),
        pageId: this.#page.id,
        applyUnitId,
        scopeId: unit.scopeId,
        authority,
        entryIds: [...entryIds],
      });
      this.#acceptResult(unit, result, generation, true);
    } catch (error) {
      this.#acceptFailure(applyUnitId, error, generation);
      throw error;
    }
  }

  stop(): void {
    ++this.#generation;
    this.#drafts = new Map();
    this.#statuses = new Map();
  }

  async #executeApply(
    unit: SettingsApplyUnitDefinition,
    intent: SettingsOpaqueValue,
    clearDraft: boolean,
  ): Promise<void> {
    this.#assertNotPending(unit.id);
    const registry = this.#host.registry();
    const scope = this.#host.scope(unit);
    const authority = scope.snapshot?.authority;
    if (authority === undefined) {
      throw new SettingsScopeUnavailableError(unit.scopeId);
    }
    const generation = this.#generation;
    this.#setStatus(unit.id, { kind: "pending" });
    try {
      const result = await this.#host.client.apply(registry, {
        protocolVersion: 1,
        requestId: this.#host.nextRequestId(),
        pageId: this.#page.id,
        applyUnitId: unit.id,
        scopeId: unit.scopeId,
        authority,
        intent,
      });
      this.#acceptResult(unit, result, generation, clearDraft);
    } catch (error) {
      this.#acceptFailure(unit.id, error, generation);
      throw error;
    }
  }

  #acceptResult(
    unit: SettingsApplyUnitDefinition,
    result: SettingsMutationResult,
    generation: number,
    clearDraft: boolean,
  ): void {
    if (generation !== this.#generation) {
      return;
    }
    if (result.status === "applied") {
      this.#host.scope(unit).accept(result.snapshot);
      if (clearDraft) {
        const drafts = new Map(this.#drafts);
        drafts.delete(unit.id);
        this.#drafts = drafts;
      }
      this.#setStatus(unit.id, {
        kind: "saved",
        receipt: result.receipt,
      });
      return;
    }
    if (result.status === "conflict") {
      this.#host.scope(unit).accept(result.snapshot);
      this.#setStatus(unit.id, {
        kind: "conflict",
        conflict: result.conflict,
      });
      return;
    }
    if (result.snapshot !== null) {
      this.#host.scope(unit).accept(result.snapshot);
    }
    this.#setStatus(unit.id, {
      kind: "rejected",
      rejection: result.rejection,
    });
  }

  #acceptFailure(
    applyUnitId: SettingsApplyUnitId,
    error: unknown,
    generation: number,
  ): void {
    if (generation !== this.#generation) {
      return;
    }
    this.#setStatus(applyUnitId, { kind: "failed", error });
    this.#host.onError(error);
  }

  #applyUnit(applyUnitId: SettingsApplyUnitId): SettingsApplyUnitDefinition {
    if (!this.#page.writableApplyUnitIds.includes(applyUnitId)) {
      throw new SettingsApplyUnitUnavailableError(applyUnitId);
    }
    const unit = this.#host
      .registry()
      .applyUnits.find(({ id }) => id === applyUnitId);
    if (unit === undefined) {
      throw new SettingsApplyUnitUnavailableError(applyUnitId);
    }
    return unit;
  }

  #setStatus(
    applyUnitId: SettingsApplyUnitId,
    status: SettingsUnitStatus,
  ): void {
    this.#statuses = new Map(this.#statuses).set(applyUnitId, status);
  }

  #assertNotPending(applyUnitId: SettingsApplyUnitId): void {
    if (this.status(applyUnitId).kind === "pending") {
      throw new SettingsMutationPendingError();
    }
  }
}
