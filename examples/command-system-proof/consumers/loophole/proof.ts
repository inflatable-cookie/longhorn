import {
  resolveCommandKeyboard,
  routeCommandKeyboardEvent,
  searchCommands,
  type CommandEffectiveBinding,
  type CommandKeymapPatch,
} from "@longhorn/commands";

import {
  availability,
  contextPath,
  createHarness,
  keyboardEvent,
  keymap,
} from "../../common.ts";

const state = createHarness("loophole");
await state.controller.start();
if (
  state.controller.status.kind !== "ready" ||
  state.controller.projection === undefined
) {
  throw new Error("Loophole controller did not become ready");
}

const semanticId = "loophole:transport.play";
const projection = state.controller.projection;
const identity = {
  palette: projection.palette.some(({ id }) => id === semanticId),
  shortcut: projection.shortcuts.some(
    ({ bindingId }) => bindingId === "loophole:binding.3",
  ),
  menu: projection.menu.some(({ id }) => id === semanticId),
  help: projection.help.some(({ id }) => id === semanticId),
  settings: projection.settings.some(({ id }) => id === semanticId),
};
if (Object.values(identity).some((present) => !present)) {
  throw new Error("semantic identity diverged across projections");
}

await state.controller.dispatch({ commandId: semanticId, source: "palette" });
await state.controller.dispatch({ commandId: semanticId, source: "menu" });
await state.controller.dispatch({ commandId: semanticId, source: "help" });
const keyboard = keyboardEvent("Space");
const keyboardRoute = routeCommandKeyboardEvent(keyboard, {
  platform: "macOs",
  contextPath: contextPath("loophole"),
  mode: "dispatch",
  bindings: state.controller.model!.keymap.bindings,
  commands: state.controller.model!.catalogue.commands,
  dispatcher: state.controller,
});
await keyboardRoute.execution;

const focusEvent = keyboardEvent("Space", { tagName: "INPUT" });
const focusRoute = routeCommandKeyboardEvent(focusEvent, {
  platform: "macOs",
  contextPath: contextPath("loophole"),
  mode: "dispatch",
  bindings: state.controller.model!.keymap.bindings,
  commands: state.controller.model!.catalogue.commands,
  dispatcher: state.controller,
});

const conflicting: CommandEffectiveBinding = {
  ...state.controller.model!.keymap.bindings.find(
    ({ invocation }) => invocation.commandId === semanticId,
  )!,
  id: "loophole:conflict",
  invocation: { commandId: "loophole:panel.close", arguments: {} },
};
const conflict = resolveCommandKeyboard({
  platform: "macOs",
  input: {
    chord: {
      code: "Space",
      modifiers: {
        control: false,
        alt: false,
        shift: false,
        meta: true,
      },
    },
    repeat: false,
    composing: false,
    editableText: false,
  },
  contextPath: contextPath("loophole"),
  mode: "dispatch",
  bindings: [...state.controller.model!.keymap.bindings, conflicting],
  commands: state.controller.model!.catalogue.commands,
});

let releaseOlder!: (value: ReturnType<typeof availability>) => void;
state.availability.loads.push(
  new Promise((resolve) => {
    releaseOlder = resolve;
  }),
  Promise.resolve(availability("loophole", 3)),
);
const olderRefresh = state.controller.refresh();
const newerRefresh = state.controller.refresh();
await newerRefresh;
releaseOlder(availability("loophole", 2));
await olderRefresh;
const staleAvailabilityRejected =
  state.controller.model!.availability.contextRevision === 3;

const priorKeymap = state.controller.model!.keymap;
const patch: CommandKeymapPatch = {
  activePresetId: null,
  clearOverrides: false,
  removeBindingIds: [],
  upsertOverrides: [],
};
state.keymap.preview = async () => ({
  status: "accepted",
  evidence: {
    registryGeneration: 1,
    keymapRevision: 1,
    activePresetId: "loophole:default",
    activePresetVersion: 1,
    patchDigest: "3".repeat(64),
  },
  snapshot: keymap("loophole"),
});
state.keymap.commit = async () => {
  throw new Error("injected persistence failure");
};
state.controller.stageKeymapPatch(patch);
const persisted = await state.controller.applyKeymapDraft();
const persistenceFailurePreserved =
  !persisted &&
  state.controller.mutation.kind === "failed" &&
  state.controller.model!.keymap === priorKeymap;

const hits = searchCommands(
  state.controller.model!.catalogue.commands,
  "play",
);
await state.controller.stop();

if (
  keyboardRoute.resolution.kind !== "resolved" ||
  focusRoute.resolution.kind !== "gated" ||
  focusRoute.resolution.gate !== "textInput" ||
  conflict.kind !== "conflict" ||
  !staleAvailabilityRejected ||
  !persistenceFailurePreserved ||
  state.executions.length !== 4 ||
  state.operations.some(
    (operation) =>
      operation.kind !== "typed-domain" ||
      operation.operation !== "transport.play",
  )
) {
  throw new Error("Loophole semantic trace failed");
}

console.log(JSON.stringify({
  shape: "loophole",
  contexts: contextPath("loophole"),
  semanticId,
  identity,
  search: hits.map(({ record }) => record.id),
  executionSources: state.executions.map(({ source }) => source),
  executor: state.operations,
  races: {
    staleAvailabilityRejected,
    focusGate: focusRoute.resolution.kind,
    conflict: conflict.kind,
    persistenceFailurePreserved,
  },
  teardown: {
    catalogueListeners: state.catalogue.listeners.size,
    keymapListeners: state.keymap.listeners.size,
    availabilityListeners: state.availability.listeners.size,
  },
}));
