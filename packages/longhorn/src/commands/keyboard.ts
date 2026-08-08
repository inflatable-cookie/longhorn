import type {
  CommandBindingCandidate,
  CommandDiscoveryRecord,
  CommandEffectiveBinding,
  CommandInvocation,
  CommandKeyChord,
  CommandKeyboardInput,
  CommandKeyboardMode,
  CommandKeyResolution,
  CommandKeymapConflict,
  CommandPlatform,
} from "./generated/protocol.ts";
import { shortcutLabel } from "./projectors.ts";
import type {
  CommandExecutionOutcome,
  CommandExecutionSource,
} from "./ports.ts";

export interface KeyboardEventLike {
  readonly code: string;
  readonly ctrlKey: boolean;
  readonly altKey: boolean;
  readonly shiftKey: boolean;
  readonly metaKey: boolean;
  readonly repeat: boolean;
  readonly isComposing?: boolean;
  readonly target?: unknown;
  preventDefault(): void;
  stopPropagation(): void;
}

export interface CommandKeyboardResolutionOptions {
  readonly platform: CommandPlatform;
  readonly input: CommandKeyboardInput;
  readonly contextPath: readonly string[];
  readonly mode: CommandKeyboardMode;
  readonly bindings: readonly CommandEffectiveBinding[];
  readonly commands: readonly CommandDiscoveryRecord[];
  readonly reserved?: (platform: CommandPlatform, chord: CommandKeyChord) => boolean;
}

export interface CommandKeyboardDispatch {
  dispatchInvocation(
    invocation: CommandInvocation,
    source: CommandExecutionSource,
  ): Promise<CommandExecutionOutcome>;
}

export interface RouteKeyboardEventOptions
  extends Omit<CommandKeyboardResolutionOptions, "input"> {
  readonly dispatcher: CommandKeyboardDispatch;
  readonly editableTarget?: (target: unknown) => boolean;
  readonly onCapture?: (chord: CommandKeyChord, label: string) => void;
}

export interface CommandKeyboardRouteResult {
  readonly consumed: boolean;
  readonly resolution: CommandKeyResolution;
  readonly execution?: Promise<CommandExecutionOutcome>;
}

export function normalizeKeyboardEvent(
  event: KeyboardEventLike,
  editableTarget: (target: unknown) => boolean = isEditableCommandTarget,
): CommandKeyboardInput {
  return {
    chord: {
      code: event.code,
      modifiers: {
        control: event.ctrlKey,
        alt: event.altKey,
        shift: event.shiftKey,
        meta: event.metaKey,
      },
    },
    repeat: event.repeat,
    composing: event.isComposing === true,
    editableText: editableTarget(event.target),
  };
}

export function resolveCommandKeyboard(
  options: CommandKeyboardResolutionOptions,
): CommandKeyResolution {
  const { input } = options;
  if (!validPhysicalCode(input.chord.code)) return { kind: "unbound" };
  if (input.repeat) return gated("repeat");
  if (input.composing) return gated("composition");
  if (options.reserved?.(options.platform, input.chord) === true) {
    return gated("reserved");
  }
  if (options.mode === "capture") {
    return {
      kind: "captured",
      chord: input.chord,
      label: shortcutLabel(input.chord, options.platform),
    };
  }

  const positions = new Map(
    options.contextPath.map((contextId, index) => [contextId, index]),
  );
  const matches = options.bindings
    .flatMap((binding) => {
      const specificity = positions.get(binding.contextId);
      if (
        specificity === undefined ||
        !includesPlatform(binding.platform, options.platform) ||
        !sameChord(resolveBindingChord(binding, options.platform), input.chord)
      ) {
        return [];
      }
      return [{ binding, specificity }];
    })
    .sort(
      (left, right) =>
        right.specificity - left.specificity ||
        left.binding.id.localeCompare(right.binding.id),
    );
  if (matches.length === 0) return { kind: "unbound" };

  const winningSpecificity = matches[0]!.specificity;
  const winningContext = matches[0]!.binding.contextId;
  const winning = matches.filter(
    ({ specificity }) => specificity === winningSpecificity,
  );
  const distinctInvocations = uniqueInvocations(
    winning.map(({ binding }) => binding.invocation),
  );
  const conflict = distinctInvocations.length > 1;
  const representativeId = matches[0]!.binding.id;
  const candidates: CommandBindingCandidate[] = matches.map(
    ({ binding, specificity }) => ({
      bindingId: binding.id,
      source: binding.source,
      matchedContextId: binding.contextId,
      specificity,
      invocation: binding.invocation,
      disposition:
        specificity < winningSpecificity
          ? { kind: "shadowed", byContextId: winningContext }
          : conflict
            ? { kind: "conflict" }
            : binding.id === representativeId
              ? { kind: "winner" }
              : { kind: "equivalent" },
    }),
  );

  if (conflict) {
    const record: CommandKeymapConflict = {
      platform: options.platform,
      chord: input.chord,
      contextId: winningContext,
      bindingIds: winning.map(({ binding }) => binding.id),
      invocations: distinctInvocations,
    };
    return { kind: "conflict", conflict: record, candidates };
  }

  const binding = matches[0]!.binding;
  const declaration = options.commands.find(
    ({ id }) => id === binding.invocation.commandId,
  );
  if (input.editableText && declaration?.textInputPolicy === "blocked") {
    return { kind: "gated", gate: "textInput", candidates };
  }
  return {
    kind: "resolved",
    winner: {
      bindingId: binding.id,
      matchedContextId: binding.contextId,
      invocation: binding.invocation,
    },
    candidates,
  };
}

export function routeCommandKeyboardEvent(
  event: KeyboardEventLike,
  options: RouteKeyboardEventOptions,
): CommandKeyboardRouteResult {
  const resolution = resolveCommandKeyboard({
    ...options,
    input: normalizeKeyboardEvent(event, options.editableTarget),
  });
  const consumed =
    resolution.kind === "captured" || resolution.kind === "resolved";
  if (consumed) {
    event.preventDefault();
    event.stopPropagation();
  }
  if (resolution.kind === "captured") {
    options.onCapture?.(resolution.chord, resolution.label);
    return { consumed, resolution };
  }
  if (resolution.kind === "resolved") {
    return {
      consumed,
      resolution,
      execution: options.dispatcher.dispatchInvocation(
        resolution.winner.invocation,
        "keyboard",
      ),
    };
  }
  return { consumed, resolution };
}

export function isEditableCommandTarget(target: unknown): boolean {
  if (typeof target !== "object" || target === null) return false;
  const record = target as {
    readonly isContentEditable?: boolean;
    readonly tagName?: unknown;
    readonly role?: unknown;
    getAttribute?(name: string): string | null;
  };
  if (record.isContentEditable === true) return true;
  const tag =
    typeof record.tagName === "string" ? record.tagName.toLowerCase() : "";
  if (tag === "input" || tag === "textarea" || tag === "select") return true;
  const role =
    typeof record.getAttribute === "function"
      ? record.getAttribute("role")
      : typeof record.role === "string"
        ? record.role
        : null;
  return role === "textbox" || role === "searchbox";
}

function gated(
  gate: "repeat" | "composition" | "reserved",
): CommandKeyResolution {
  return { kind: "gated", gate, candidates: [] };
}

function includesPlatform(
  scope: CommandEffectiveBinding["platform"],
  platform: CommandPlatform,
): boolean {
  return scope === "any" || scope === platform;
}

function resolveBindingChord(
  binding: CommandEffectiveBinding,
  platform: CommandPlatform,
): CommandKeyChord {
  return {
    code: binding.trigger.code,
    modifiers: {
      control:
        binding.trigger.modifiers.control ||
        (binding.trigger.modifiers.primary &&
          (platform === "windows" || platform === "linux")),
      alt: binding.trigger.modifiers.alt,
      shift: binding.trigger.modifiers.shift,
      meta:
        binding.trigger.modifiers.meta ||
        (binding.trigger.modifiers.primary && platform === "macOs"),
    },
  };
}

function sameChord(left: CommandKeyChord, right: CommandKeyChord): boolean {
  return (
    left.code === right.code &&
    left.modifiers.control === right.modifiers.control &&
    left.modifiers.alt === right.modifiers.alt &&
    left.modifiers.shift === right.modifiers.shift &&
    left.modifiers.meta === right.modifiers.meta
  );
}

function uniqueInvocations(
  invocations: readonly CommandInvocation[],
): CommandInvocation[] {
  const keyed = new Map<string, CommandInvocation>();
  for (const invocation of invocations) {
    keyed.set(invocationKey(invocation), invocation);
  }
  return [...keyed.values()].sort(compareInvocation);
}

function invocationKey(invocation: CommandInvocation): string {
  const argumentsKey = Object.entries(invocation.arguments)
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([key, value]) => [key, value]);
  return JSON.stringify([invocation.commandId, argumentsKey]);
}

function compareInvocation(
  left: CommandInvocation,
  right: CommandInvocation,
): number {
  const id = compareText(left.commandId, right.commandId);
  if (id !== 0) return id;
  const leftArguments = Object.entries(left.arguments).sort(([a], [b]) =>
    compareText(a, b),
  );
  const rightArguments = Object.entries(right.arguments).sort(([a], [b]) =>
    compareText(a, b),
  );
  const length = Math.min(leftArguments.length, rightArguments.length);
  for (let index = 0; index < length; index += 1) {
    const [leftId, leftValue] = leftArguments[index]!;
    const [rightId, rightValue] = rightArguments[index]!;
    const field = compareText(leftId, rightId);
    if (field !== 0) return field;
    const value = compareArgumentValue(leftValue, rightValue);
    if (value !== 0) return value;
  }
  return leftArguments.length - rightArguments.length;
}

function compareArgumentValue(
  left: CommandInvocation["arguments"][string],
  right: CommandInvocation["arguments"][string],
): number {
  const ranks = {
    boolean: 0,
    number: 1,
    integer: 2,
    string: 3,
    enum: 4,
  } as const;
  const rank = ranks[left.kind] - ranks[right.kind];
  if (rank !== 0) return rank;
  if (
    (left.kind === "number" || left.kind === "integer") &&
    (right.kind === "number" || right.kind === "integer")
  ) {
    return left.value - right.value;
  }
  if (left.kind === "boolean" && right.kind === "boolean") {
    return Number(left.value) - Number(right.value);
  }
  return compareText(String(left.value), String(right.value));
}

function compareText(left: string, right: string): number {
  const leftValues = [...left];
  const rightValues = [...right];
  const length = Math.min(leftValues.length, rightValues.length);
  for (let index = 0; index < length; index += 1) {
    const difference =
      leftValues[index]!.codePointAt(0)! -
      rightValues[index]!.codePointAt(0)!;
    if (difference !== 0) return difference;
  }
  return leftValues.length - rightValues.length;
}

function validPhysicalCode(code: string): boolean {
  return (
    code !== "Unidentified" &&
    code.length > 0 &&
    code.length <= 64 &&
    /^[A-Z][A-Za-z0-9]*$/u.test(code)
  );
}
