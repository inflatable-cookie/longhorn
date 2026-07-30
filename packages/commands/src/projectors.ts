import type {
  CommandAvailability,
  CommandAvailabilitySnapshot,
  CommandCatalogueSnapshot,
  CommandDiscoveryRecord,
  CommandEffectiveBinding,
  CommandId,
  CommandKeyChord,
  CommandKeymapConflict,
  CommandKeymapSnapshot,
  CommandPlatform,
  CommandSearchHit,
  CommandShortcutRecord,
} from "./generated/protocol.ts";

export interface CommandJoinedState {
  readonly catalogue: CommandCatalogueSnapshot;
  readonly keymap: CommandKeymapSnapshot;
  readonly availability: CommandAvailabilitySnapshot;
}

export interface CommandProjectionRecord {
  readonly id: CommandId;
  readonly label: string;
  readonly description: string | null;
  readonly categoryPath: readonly string[];
  readonly keywords: readonly string[];
  readonly icon: string | null;
  readonly availability: CommandAvailability;
  readonly shortcuts: readonly CommandShortcutRecord[];
}

export interface CommandSettingsRecord extends CommandProjectionRecord {
  readonly bindings: readonly CommandEffectiveBinding[];
}

export interface CommandSurfaceProjection {
  readonly palette: readonly CommandProjectionRecord[];
  readonly menu: readonly CommandProjectionRecord[];
  readonly help: readonly CommandProjectionRecord[];
  readonly settings: readonly CommandSettingsRecord[];
  readonly shortcuts: readonly CommandShortcutRecord[];
  readonly reverseLookup: ReadonlyMap<CommandId, readonly CommandShortcutRecord[]>;
  readonly conflicts: readonly CommandKeymapConflict[];
}

const AVAILABLE: CommandAvailability = {
  state: "available",
  reason: null,
};

export function joinCommandState(
  catalogue: CommandCatalogueSnapshot,
  keymap: CommandKeymapSnapshot,
  availability: CommandAvailabilitySnapshot,
): CommandJoinedState {
  if (
    keymap.registryGeneration !== catalogue.registryGeneration ||
    availability.registryGeneration !== catalogue.registryGeneration
  ) {
    throw new CommandProjectionConsistencyError(
      "catalogue, keymap, and availability generations differ",
    );
  }
  if (keymap.registryDigest !== catalogue.registryDigest) {
    throw new CommandProjectionConsistencyError(
      "catalogue and keymap registry digests differ",
    );
  }
  const known = new Set(catalogue.commands.map(({ id }) => id));
  const records = new Set<string>();
  for (const record of availability.records) {
    if (!known.has(record.commandId) || records.has(record.commandId)) {
      throw new CommandProjectionConsistencyError(
        `availability has unknown or duplicate command ${record.commandId}`,
      );
    }
    records.add(record.commandId);
  }
  if (records.size !== known.size) {
    throw new CommandProjectionConsistencyError(
      "availability is not complete for the sealed catalogue",
    );
  }
  return { catalogue, keymap, availability };
}

export function projectCommandSurfaces(
  state: CommandJoinedState,
  platform: CommandPlatform,
): CommandSurfaceProjection {
  const availability = new Map(
    state.availability.records.map((record) => [
      record.commandId,
      record.availability,
    ]),
  );
  const reverseLookup = new Map<CommandId, readonly CommandShortcutRecord[]>();
  const shortcuts: CommandShortcutRecord[] = [];
  for (const command of state.catalogue.commands) {
    const records = shortcutsForCommand(state.keymap, command.id, platform);
    reverseLookup.set(command.id, records);
    shortcuts.push(...records);
  }
  const project = (record: CommandDiscoveryRecord): CommandProjectionRecord => ({
    id: record.id,
    label: record.label,
    description: record.description,
    categoryPath: record.categoryPath,
    keywords: record.keywords,
    icon: record.icon,
    availability: availability.get(record.id) ?? AVAILABLE,
    shortcuts: reverseLookup.get(record.id) ?? [],
  });
  const visible = state.catalogue.commands.filter(
    ({ visibility }) => !visibility.hidden,
  );
  return {
    palette: visible
      .filter(({ visibility }) => visibility.palette)
      .map(project)
      .filter(({ availability }) => availability.state !== "hidden"),
    menu: visible
      .filter(({ visibility }) => visibility.menu)
      .map(project)
      .filter(({ availability }) => availability.state !== "hidden"),
    help: visible
      .filter(({ visibility }) => visibility.help)
      .map(project)
      .filter(({ availability }) => availability.state !== "hidden"),
    settings: visible
      .filter(({ visibility }) => visibility.settings)
      .map((record) => ({
        ...project(record),
        bindings: state.keymap.bindings.filter(
          ({ invocation }) => invocation.commandId === record.id,
        ),
      })),
    shortcuts,
    reverseLookup,
    conflicts: state.keymap.conflicts,
  };
}

export function searchCommands(
  records: readonly CommandDiscoveryRecord[],
  query: string,
): readonly CommandSearchHit[] {
  const terms = query.split(/\s+/u).filter(Boolean).map((term) => term.toLowerCase());
  return records
    .flatMap((record) => {
      const score = scoreRecord(record, terms);
      return score === undefined ? [] : [{ record, score }];
    })
    .sort(
      (left, right) =>
        left.score - right.score ||
        compareUnicodeScalars(
          left.record.label.toLowerCase(),
          right.record.label.toLowerCase(),
        ) ||
        compareUnicodeScalars(left.record.id, right.record.id),
    );
}

export function shortcutsForCommand(
  keymap: CommandKeymapSnapshot,
  commandId: CommandId,
  platform: CommandPlatform,
): readonly CommandShortcutRecord[] {
  return keymap.bindings
    .filter(
      (binding) =>
        binding.invocation.commandId === commandId &&
        includesPlatform(binding.platform, platform),
    )
    .map((binding) => {
      const chord = resolveTrigger(binding, platform);
      return {
        bindingId: binding.id,
        source: binding.source,
        contextId: binding.contextId,
        chord,
        label: shortcutLabel(chord, platform),
      };
    });
}

export function shortcutLabel(
  chord: CommandKeyChord,
  platform: CommandPlatform,
): string {
  const key = keyLabel(chord.code);
  if (platform === "macOs") {
    return [
      chord.modifiers.control ? "⌃" : "",
      chord.modifiers.alt ? "⌥" : "",
      chord.modifiers.shift ? "⇧" : "",
      chord.modifiers.meta ? "⌘" : "",
      key,
    ].join("");
  }
  return [
    chord.modifiers.control ? "Ctrl" : undefined,
    chord.modifiers.alt ? "Alt" : undefined,
    chord.modifiers.shift ? "Shift" : undefined,
    chord.modifiers.meta ? "Meta" : undefined,
    key,
  ]
    .filter((part): part is string => part !== undefined)
    .join("+");
}

export class CommandProjectionConsistencyError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "CommandProjectionConsistencyError";
  }
}

function scoreRecord(
  record: CommandDiscoveryRecord,
  terms: readonly string[],
): number | undefined {
  if (terms.length === 0) return 0;
  const label = record.label.toLowerCase();
  const description = record.description?.toLowerCase() ?? "";
  const keywords = record.keywords.map((keyword) => keyword.toLowerCase());
  let score = 0;
  for (const term of terms) {
    const termScore =
      label === term
        ? 0
        : label.startsWith(term)
          ? 10
          : label.includes(term)
            ? 20
            : keywords.some((keyword) => keyword === term)
              ? 30
              : keywords.some((keyword) => keyword.startsWith(term))
                ? 40
                : keywords.some((keyword) => keyword.includes(term))
                  ? 50
                  : record.categoryPath.some((category) => category.includes(term))
                    ? 60
                    : record.id.includes(term)
                      ? 70
                      : description.includes(term)
                        ? 80
                        : undefined;
    if (termScore === undefined) return undefined;
    score = Math.min(0xffff_ffff, score + termScore);
  }
  return score;
}

function includesPlatform(
  scope: CommandEffectiveBinding["platform"],
  platform: CommandPlatform,
): boolean {
  return scope === "any" || scope === platform;
}

function resolveTrigger(
  binding: CommandEffectiveBinding,
  platform: CommandPlatform,
): CommandKeyChord {
  const { modifiers } = binding.trigger;
  return {
    code: binding.trigger.code,
    modifiers: {
      control:
        modifiers.control ||
        (modifiers.primary && (platform === "windows" || platform === "linux")),
      alt: modifiers.alt,
      shift: modifiers.shift,
      meta: modifiers.meta || (modifiers.primary && platform === "macOs"),
    },
  };
}

function keyLabel(code: string): string {
  if (code.length === 4 && code.startsWith("Key")) return code.slice(3);
  if (code.length === 6 && code.startsWith("Digit")) return code.slice(5);
  return (
    {
      ArrowUp: "↑",
      ArrowDown: "↓",
      ArrowLeft: "←",
      ArrowRight: "→",
      Escape: "Esc",
    }[code] ?? code
  );
}

function compareUnicodeScalars(left: string, right: string): number {
  const leftValues = [...left];
  const rightValues = [...right];
  const length = Math.min(leftValues.length, rightValues.length);
  for (let index = 0; index < length; index += 1) {
    const leftCode = leftValues[index]!.codePointAt(0)!;
    const rightCode = rightValues[index]!.codePointAt(0)!;
    if (leftCode !== rightCode) return leftCode - rightCode;
  }
  return leftValues.length - rightValues.length;
}
