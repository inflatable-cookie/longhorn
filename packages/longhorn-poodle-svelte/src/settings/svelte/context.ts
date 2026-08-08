import type {
  SettingsActivationRequirement,
  SettingsApplyUnitId,
  SettingsEntryId,
  SettingsOpaqueValue,
  SettingsPageDefinition,
  SettingsRecoveryState,
  SettingsScopeId,
  SettingsScopeSnapshot,
} from "@inflatable-cookie/longhorn/settings/protocol";
import type { SettingsPageSession } from "./page.svelte.ts";
import type {
  SettingsPageRenderContext,
  SettingsRoute,
} from "./types.ts";

export interface SettingsContextHost {
  readonly activationRequirements: readonly SettingsActivationRequirement[];
  readonly recovery: SettingsRecoveryState | null;
  scopeSnapshot(scopeId: SettingsScopeId): SettingsScopeSnapshot | undefined;
  requestReset(
    applyUnitId: SettingsApplyUnitId,
    entryIds: readonly SettingsEntryId[],
  ): Promise<void>;
}

export function createSettingsPageRenderContext(
  host: SettingsContextHost,
  page: SettingsPageDefinition,
  route: SettingsRoute,
  session: SettingsPageSession,
): SettingsPageRenderContext {
  return {
    page,
    route,
    get dirty() {
      return session.dirty;
    },
    get busy() {
      return session.busy;
    },
    get activationRequirements() {
      return host.activationRequirements;
    },
    get recovery() {
      return host.recovery;
    },
    snapshot: (scopeId) => host.scopeSnapshot(scopeId),
    unitStatus: (applyUnitId) => session.status(applyUnitId),
    draft: (applyUnitId) => session.draft(applyUnitId),
    change: (
      applyUnitId: SettingsApplyUnitId,
      intent: SettingsOpaqueValue,
    ) => session.change(applyUnitId, intent),
    apply: (applyUnitId) => session.apply(applyUnitId),
    cancel: () => session.cancel(),
    requestReset: (applyUnitId, entryIds) =>
      host.requestReset(applyUnitId, entryIds),
  };
}

