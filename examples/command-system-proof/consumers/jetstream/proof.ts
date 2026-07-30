import {
  routeCommandKeyboardEvent,
  searchCommands,
} from "@longhorn/commands";

import {
  contextPath,
  createHarness,
  keyboardEvent,
} from "../../common.ts";

const state = createHarness("jetstream");
await state.controller.start();
if (state.controller.status.kind !== "ready") {
  throw new Error("Jetstream controller did not become ready");
}
const event = keyboardEvent("KeyO");
const route = routeCommandKeyboardEvent(event, {
  platform: "macOs",
  contextPath: contextPath("jetstream"),
  mode: "dispatch",
  bindings: state.controller.model!.keymap.bindings,
  commands: state.controller.model!.catalogue.commands,
  dispatcher: state.controller,
});
await route.execution;
const hits = searchCommands(
  state.controller.model!.catalogue.commands,
  "open",
);
await state.controller.stop();
if (
  route.resolution.kind !== "resolved" ||
  state.executions[0]?.invocation.commandId !== "jetstream:file.open" ||
  state.operations[0]?.kind !== "local" ||
  state.catalogue.listeners.size !== 0 ||
  state.keymap.listeners.size !== 0 ||
  state.availability.listeners.size !== 0
) {
  throw new Error("Jetstream semantic trace failed");
}

console.log(JSON.stringify({
  shape: "jetstream",
  contexts: contextPath("jetstream"),
  commandId: "jetstream:file.open",
  keyboard: {
    resolution: route.resolution.kind,
    consumed: route.consumed,
    prevented: event.counters.prevented,
    stopped: event.counters.stopped,
  },
  search: hits.map(({ record }) => record.id),
  executor: state.operations,
  teardown: {
    catalogueListeners: state.catalogue.listeners.size,
    keymapListeners: state.keymap.listeners.size,
    availabilityListeners: state.availability.listeners.size,
  },
}));
