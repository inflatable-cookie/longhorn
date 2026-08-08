import type {
  SettingsMutationTiming,
  SettingsPageDefinition,
  SettingsRegistrySnapshot,
  SettingsScopeSnapshot,
} from "@inflatable-cookie/longhorn-settings";

import settingsFixture from "./fixtures/settings-protocol-v1.json";

export type ProofShape = "bovine" | "soundcheck" | "loophole" | "nucleus";

const featureDefaults = {
  reset: false,
  import: false,
  backup: false,
  restore: false,
  confirmation: false,
};

export function createProofRegistry(shape: ProofShape): SettingsRegistrySnapshot {
  const base = structuredClone(
    settingsFixture.registry,
  ) as SettingsRegistrySnapshot;
  const declarations = declarationsFor(shape);
  return {
    ...base,
    generation: 48,
    digest: "4848484848484848484848484848484848484848484848484848484848484848",
    composedCapabilities: declarations.capabilities.map(({ id }) => id),
    ...declarations,
  };
}

export function createProofSnapshot(
  shape: ProofShape,
  recoveryRequired = false,
): SettingsScopeSnapshot {
  const snapshot = structuredClone(
    settingsFixture.snapshots[0],
  ) as SettingsScopeSnapshot;
  snapshot.scopeId = scopeIdFor(shape);
  snapshot.authority = {
    registryGeneration: 48,
    scopeRevision: 3,
    authorityToken: `authority:${shape}`,
  };
  snapshot.values[0]!.entryId = `${shape}:primary`;
  snapshot.values[1]!.entryId = `${shape}:managed`;
  snapshot.values[2]!.entryId = `${shape}:hidden`;
  snapshot.values[3]!.entryId = `${shape}:unsupported`;
  snapshot.recovery = recoveryRequired
    ? {
        code: "recoveryRequired",
        diagnostic: {
          codecVersion: 1,
          value: { owner: "restore-host" },
        },
      }
    : null;
  return snapshot;
}

function declarationsFor(shape: ProofShape) {
  switch (shape) {
    case "bovine":
      return productDeclarations(
        shape,
        [
          page("bovine:preferences", "bovine:preferences", "Preferences", [
            "bovine:preferences",
          ]),
        ],
        [{ id: "bovine:preferences", timing: "staged" }],
      );
    case "nucleus":
      return productDeclarations(
        shape,
        [
          page("nucleus:general", "nucleus:general", "General", [
            "nucleus:preferences",
          ]),
        ],
        [{ id: "nucleus:preferences", timing: "staged" }],
      );
    case "soundcheck":
      return soundcheckDeclarations();
    case "loophole":
      return loopholeDeclarations();
  }
}

function productDeclarations(
  shape: "bovine" | "nucleus",
  pages: SettingsPageDefinition[],
  units: readonly { id: string; timing: SettingsMutationTiming }[],
) {
  const moduleId = `${shape}:settings`;
  const scopeId = scopeIdFor(shape);
  return {
    modules: [{ id: moduleId, label: capitalize(shape), order: 0 }],
    sections: [
      { id: `${shape}:general`, moduleId, label: "General", order: 0 },
    ],
    pages,
    renderers: pages.map(({ rendererId }) => ({ id: rendererId, moduleId })),
    scopes: [{ id: scopeId, moduleId }],
    applyUnits: units.map(({ id, timing }) => ({
      id,
      moduleId,
      scopeId,
      timing,
      resetSupported: true,
    })),
    capabilities: [{ id: `${shape}:settings-capable`, moduleId }],
  };
}

function soundcheckDeclarations() {
  const productModule = "soundcheck:settings";
  const storageModule = "longhorn:config-operations";
  return {
    modules: [
      { id: productModule, label: "Soundcheck", order: 0 },
      { id: storageModule, label: "Storage", order: 700 },
    ],
    sections: [
      {
        id: "soundcheck:general",
        moduleId: productModule,
        label: "General",
        order: 0,
      },
      {
        id: "longhorn:storage-and-backup",
        moduleId: storageModule,
        label: "Storage & Backups",
        order: 0,
      },
    ],
    pages: [
      page(
        "soundcheck:audio",
        "soundcheck:product",
        "Audio",
        ["soundcheck:preferences"],
        productModule,
        "soundcheck:general",
      ),
      operationPage("longhorn:storage", "longhorn:config.storage", "Storage", 0),
      operationPage("longhorn:backup", "longhorn:config.backup", "Backups", 10),
      operationPage(
        "longhorn:restore",
        "longhorn:config.restore",
        "Restore & Recovery",
        20,
      ),
    ],
    renderers: [
      { id: "soundcheck:product", moduleId: productModule },
      { id: "longhorn:config.storage", moduleId: storageModule },
      { id: "longhorn:config.backup", moduleId: storageModule },
      { id: "longhorn:config.restore", moduleId: storageModule },
    ],
    scopes: [{ id: "soundcheck:preferences", moduleId: productModule }],
    applyUnits: [
      {
        id: "soundcheck:preferences",
        moduleId: productModule,
        scopeId: "soundcheck:preferences",
        timing: "immediate" as const,
        resetSupported: true,
      },
    ],
    capabilities: [
      { id: "soundcheck:settings-capable", moduleId: productModule },
      { id: "longhorn:config.storage-diagnostics", moduleId: storageModule },
      { id: "longhorn:config.backup-inventory", moduleId: storageModule },
      { id: "longhorn:config.restore-inspection", moduleId: storageModule },
    ],
  };
}

function loopholeDeclarations() {
  const moduleId = "loophole:settings";
  const sectionId = "loophole:general";
  const scopeId = "loophole:preferences";
  const entries = [
    ["loophole:application", "Application", ["loophole:application"]],
    [
      "loophole:appearance",
      "Appearance",
      ["loophole:appearance", "loophole:studio"],
    ],
    ["loophole:hardware", "Hardware", []],
    ["loophole:keybindings", "Keybindings", []],
  ] as const;
  return {
    modules: [{ id: moduleId, label: "Loophole", order: 0 }],
    sections: [{ id: sectionId, moduleId, label: "Application", order: 0 }],
    pages: entries.map(([id, label, writableApplyUnitIds], order) =>
      page(id, id, label, writableApplyUnitIds, moduleId, sectionId, order * 10),
    ),
    renderers: entries.map(([id]) => ({ id, moduleId })),
    scopes: [{ id: scopeId, moduleId }],
    applyUnits: [
      {
        id: "loophole:application",
        moduleId,
        scopeId,
        timing: "immediate" as const,
        resetSupported: true,
      },
      {
        id: "loophole:appearance",
        moduleId,
        scopeId,
        timing: "staged" as const,
        resetSupported: true,
      },
      {
        id: "loophole:studio",
        moduleId,
        scopeId,
        timing: "staged" as const,
        resetSupported: true,
      },
    ],
    capabilities: [{ id: "loophole:settings-capable", moduleId }],
  };
}

function page(
  id: string,
  rendererId: string,
  label: string,
  writableApplyUnitIds: readonly string[],
  moduleId = `${id.split(":")[0]}:settings`,
  sectionId = `${id.split(":")[0]}:general`,
  order = 0,
): SettingsPageDefinition {
  const scope = scopeIdFor(id.split(":")[0] as ProofShape);
  return {
    id,
    moduleId,
    sectionId,
    rendererId,
    label,
    keywords: [label.toLowerCase()],
    order,
    anchors: [],
    requiredCapabilities: [`${id.split(":")[0]}:settings-capable`],
    readableScopeIds: writableApplyUnitIds.length > 0 ? [scope] : [],
    writableApplyUnitIds: [...writableApplyUnitIds],
    features: { ...featureDefaults, reset: writableApplyUnitIds.length > 0 },
  };
}

function operationPage(
  id: string,
  rendererId: string,
  label: string,
  order: number,
): SettingsPageDefinition {
  return {
    id,
    moduleId: "longhorn:config-operations",
    sectionId: "longhorn:storage-and-backup",
    rendererId,
    label,
    keywords: [label.toLowerCase()],
    order,
    anchors: [],
    requiredCapabilities: [
      id === "longhorn:storage"
        ? "longhorn:config.storage-diagnostics"
        : id === "longhorn:backup"
          ? "longhorn:config.backup-inventory"
          : "longhorn:config.restore-inspection",
    ],
    readableScopeIds: [],
    writableApplyUnitIds: [],
    features: {
      ...featureDefaults,
      backup: id === "longhorn:backup",
      restore: id === "longhorn:restore",
      confirmation: true,
    },
  };
}

function scopeIdFor(shape: ProofShape | string): string {
  return `${shape}:preferences`;
}

function capitalize(value: string): string {
  return value[0]!.toUpperCase() + value.slice(1);
}
