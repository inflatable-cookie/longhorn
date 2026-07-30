import type {
  SettingsActivationRequirement,
  SettingsApplyUnitId,
  SettingsEntryId,
  SettingsPageDefinition,
  SettingsRegistrySnapshot,
  SettingsRequestId,
  SettingsScopeId,
  SettingsScopeSnapshot,
} from "../generated/protocol.ts";
import {
  resolveSettingsDeepLink,
  searchSettingsRegistry,
  type SettingsRegistryProjection,
  type SettingsSearchResult,
} from "../registry.ts";
import type { SettingsClient } from "../client.ts";
import { createSettingsPageRenderContext } from "./context.ts";
import { SettingsPageUnavailableError } from "./errors.ts";
import type { SettingsPageSession } from "./page.svelte.ts";
import { SettingsSessionRuntime } from "./runtime.svelte.ts";
import type {
  SettingsGuard,
  SettingsGuardResolution,
  SettingsPageRenderContext,
  SettingsPageRenderer,
  SettingsRendererResolver,
  SettingsResetRequest,
  SettingsRoute,
  SettingsSessionStatus,
  SettingsUnitStatus,
} from "./types.ts";

export interface SettingsSessionOptions {
  readonly client: SettingsClient;
  readonly nextRequestId: () => SettingsRequestId;
  readonly initialRoute?: SettingsRoute;
  readonly onClose?: () => void;
  readonly onError?: (error: unknown) => void;
}

export class SettingsSession {
  readonly #options: SettingsSessionOptions;
  readonly #runtime: SettingsSessionRuntime;
  #guard = $state.raw<SettingsGuard | undefined>(undefined);
  #resetRequest = $state.raw<SettingsResetRequest | undefined>(undefined);

  constructor(options: SettingsSessionOptions) {
    this.#options = options;
    this.#runtime = new SettingsSessionRuntime({
      client: options.client,
      nextRequestId: options.nextRequestId,
      initialRoute: options.initialRoute,
      onError: (error) => options.onError?.(error),
    });
  }

  get status(): SettingsSessionStatus {
    return this.#runtime.status;
  }

  get registry(): SettingsRegistrySnapshot | undefined {
    return this.#runtime.document.registry;
  }

  get navigation(): SettingsRegistryProjection | undefined {
    return this.#runtime.document.projection;
  }

  get route(): SettingsRoute | undefined {
    return this.#runtime.document.route;
  }

  get guard(): SettingsGuard | undefined {
    return this.#guard;
  }

  get resetRequest(): SettingsResetRequest | undefined {
    return this.#resetRequest;
  }

  get focusRevision(): number {
    return this.#runtime.document.focusRevision;
  }

  get currentPage(): SettingsPageDefinition | undefined {
    return this.#runtime.document.currentPage;
  }

  get currentRenderer(): SettingsPageRenderer | undefined {
    return this.#runtime.document.currentRenderer;
  }

  get currentPageSession(): SettingsPageSession | undefined {
    return this.#runtime.document.currentPageSession;
  }

  get currentContext(): SettingsPageRenderContext | undefined {
    const page = this.currentPage;
    const route = this.route;
    const session = this.currentPageSession;
    return page === undefined || route === undefined || session === undefined
      ? undefined
      : createSettingsPageRenderContext(this, page, route, session);
  }

  get dirty(): boolean {
    return this.currentPageSession?.dirty ?? false;
  }

  get busy(): boolean {
    return this.currentPageSession?.busy ?? false;
  }

  get draftCount(): number {
    return this.currentPageSession?.drafts.length ?? 0;
  }

  get canApplyCurrent(): boolean {
    return this.draftCount === 1 && !this.busy;
  }

  get activationRequirements(): readonly SettingsActivationRequirement[] {
    return this.#runtime.document.currentScopes().flatMap(
      ({ snapshot }) => snapshot?.activationRequirements ?? [],
    );
  }

  get recovery() {
    return (
      this.#runtime.document
        .currentScopes()
        .map(({ snapshot }) => snapshot?.recovery)
        .find((value) => value != null) ?? null
    );
  }

  get primaryUnitStatus(): SettingsUnitStatus {
    const page = this.currentPage;
    const session = this.currentPageSession;
    if (page === undefined || session === undefined) {
      return { kind: "idle" };
    }
    const statuses = page.writableApplyUnitIds.map((id) =>
      session.status(id),
    );
    return (
      statuses.find(({ kind }) => kind === "pending") ??
      statuses.find(({ kind }) => kind === "failed") ??
      statuses.find(({ kind }) => kind === "conflict") ??
      statuses.find(({ kind }) => kind === "rejected") ??
      statuses.find(({ kind }) => kind === "saved") ??
      { kind: "idle" }
    );
  }

  start(rendererResolver: SettingsRendererResolver): Promise<void> {
    return this.#runtime.start(rendererResolver);
  }

  async stop(): Promise<void> {
    this.#guard = undefined;
    this.#resetRequest = undefined;
    await this.#runtime.stop();
  }

  search(query: string): readonly SettingsSearchResult[] {
    return this.registry === undefined
      ? []
      : searchSettingsRegistry(this.registry, query);
  }

  async navigate(route: SettingsRoute): Promise<boolean> {
    resolveSettingsDeepLink(this.#runtime.document.requiredRegistry(), route);
    if (!this.#canLeave()) {
      this.#guard = { kind: "navigate", route };
      return false;
    }
    await this.#runtime.document.installRoute(route);
    return true;
  }

  requestClose(): boolean {
    if (!this.#canLeave()) {
      this.#guard = { kind: "close" };
      return false;
    }
    this.#options.onClose?.();
    return true;
  }

  async resolveGuard(resolution: SettingsGuardResolution): Promise<boolean> {
    const guard = this.#guard;
    const page = this.currentPageSession;
    if (guard === undefined) return true;
    if (resolution === "stay") {
      this.#guard = undefined;
      return false;
    }
    if (page === undefined) {
      this.#guard = undefined;
      return false;
    }
    if (page.busy) return false;
    if (resolution === "apply") {
      if (!(await page.applyOnlyDraft())) return false;
    } else {
      page.cancel();
    }
    this.#guard = undefined;
    if (guard.kind === "navigate") {
      await this.#runtime.document.installRoute(guard.route);
    } else {
      this.#options.onClose?.();
    }
    return true;
  }

  cancelCurrent(): void {
    this.currentPageSession?.cancel();
  }

  async applyCurrent(): Promise<boolean> {
    return (await this.currentPageSession?.applyOnlyDraft()) ?? true;
  }

  async requestReset(
    applyUnitId: SettingsApplyUnitId,
    entryIds: readonly SettingsEntryId[],
  ): Promise<void> {
    const page = this.currentPage;
    if (page === undefined) throw new SettingsPageUnavailableError();
    const request = {
      pageId: page.id,
      applyUnitId,
      entryIds: [...entryIds],
    };
    if (page.features.confirmation) {
      this.#resetRequest = request;
    } else {
      await this.#executeReset(request);
    }
  }

  async resolveReset(confirmed: boolean): Promise<void> {
    const request = this.#resetRequest;
    this.#resetRequest = undefined;
    if (confirmed && request !== undefined) {
      await this.#executeReset(request);
    }
  }

  scopeSnapshot(scopeId: SettingsScopeId): SettingsScopeSnapshot | undefined {
    return this.#runtime.document.scopeSnapshot(scopeId);
  }

  reconnect(): Promise<void> {
    return this.#runtime.reconnect();
  }

  #canLeave(): boolean {
    const page = this.currentPageSession;
    return page === undefined || (!page.dirty && !page.busy);
  }

  async #executeReset(request: SettingsResetRequest): Promise<void> {
    if (request.pageId !== this.currentPage?.id) {
      throw new SettingsPageUnavailableError();
    }
    await this.currentPageSession?.reset(
      request.applyUnitId,
      request.entryIds,
    );
  }
}
