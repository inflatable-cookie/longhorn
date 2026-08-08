import type { Snippet } from "svelte";

import type {
  SettingsActivationRequirement,
  SettingsApplyUnitId,
  SettingsAnchorId,
  SettingsConflict,
  SettingsMutationReceipt,
  SettingsOpaqueValue,
  SettingsPageDefinition,
  SettingsPageId,
  SettingsRecoveryState,
  SettingsRejection,
  SettingsRendererId,
  SettingsScopeId,
  SettingsScopeSnapshot,
} from "@inflatable-cookie/longhorn/settings/protocol";

export type SettingsSessionStatus =
  | { readonly kind: "idle" }
  | { readonly kind: "loading" }
  | { readonly kind: "ready" }
  | { readonly kind: "reconnecting" }
  | { readonly kind: "unsupported"; readonly reason: string }
  | { readonly kind: "failed"; readonly error: unknown };

export type SettingsScopeStatus =
  | { readonly kind: "idle" }
  | { readonly kind: "loading" }
  | { readonly kind: "ready" }
  | { readonly kind: "failed"; readonly error: unknown };

export type SettingsUnitStatus =
  | { readonly kind: "idle" }
  | { readonly kind: "pending" }
  | { readonly kind: "saved"; readonly receipt: SettingsMutationReceipt }
  | { readonly kind: "conflict"; readonly conflict: SettingsConflict }
  | { readonly kind: "rejected"; readonly rejection: SettingsRejection }
  | { readonly kind: "failed"; readonly error: unknown };

export interface SettingsRoute {
  readonly pageId: SettingsPageId;
  readonly anchorId?: SettingsAnchorId;
}

export interface SettingsDraft {
  readonly applyUnitId: SettingsApplyUnitId;
  readonly intent: SettingsOpaqueValue;
}

export type SettingsGuard =
  | { readonly kind: "navigate"; readonly route: SettingsRoute }
  | { readonly kind: "close" };

export type SettingsGuardResolution = "apply" | "discard" | "stay";

export interface SettingsResetRequest {
  readonly pageId: SettingsPageId;
  readonly applyUnitId: SettingsApplyUnitId;
  readonly entryIds: readonly string[];
}

export interface SettingsPageRenderContext {
  readonly page: SettingsPageDefinition;
  readonly route: SettingsRoute;
  readonly dirty: boolean;
  readonly busy: boolean;
  readonly activationRequirements: readonly SettingsActivationRequirement[];
  readonly recovery: SettingsRecoveryState | null;
  snapshot(scopeId: SettingsScopeId): SettingsScopeSnapshot | undefined;
  unitStatus(applyUnitId: SettingsApplyUnitId): SettingsUnitStatus;
  draft(applyUnitId: SettingsApplyUnitId): SettingsDraft | undefined;
  change(
    applyUnitId: SettingsApplyUnitId,
    intent: SettingsOpaqueValue,
  ): Promise<void>;
  apply(applyUnitId: SettingsApplyUnitId): Promise<void>;
  cancel(): void;
  requestReset(
    applyUnitId: SettingsApplyUnitId,
    entryIds: readonly string[],
  ): Promise<void>;
}

export type SettingsPageRenderer = Snippet<[SettingsPageRenderContext]>;

export type SettingsRendererResolver = (
  rendererId: SettingsRendererId,
  page: SettingsPageDefinition,
) => SettingsPageRenderer | undefined;

export type SettingsHostForm = "modal" | "window" | "panel";

