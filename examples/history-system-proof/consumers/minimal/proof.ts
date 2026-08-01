import {
  createControllerHarness,
  equalJson,
  publicTrace,
  rendererFixture,
} from "../../common.ts";
import fixtureJson from "./fixture.json";

const fixture = rendererFixture(fixtureJson);
const harness = createControllerHarness(fixture);
await harness.controller.start();
if (
  harness.controller.status.kind !== "ready" ||
  !harness.controller.entries.some(({ position }) => position === "future")
) {
  throw new Error("minimal controller lost authoritative future entries");
}
await harness.controller.undo();
const trace = publicTrace(harness.controller);
if (!equalJson(trace, fixture.expectedPublicTrace)) {
  throw new Error(
    `minimal Rust and TypeScript traces diverged: ${JSON.stringify(trace)} != ${JSON.stringify(fixture.expectedPublicTrace)}`,
  );
}
await harness.controller.stop();
if (harness.counters.unlisten !== 1) {
  throw new Error("minimal controller leaked its history listener");
}

console.log(
  JSON.stringify({
    shape: "minimal",
    publicTrace: trace,
    transport: "direct",
    futureAuthoritative: true,
    teardown: {
      unlisten: harness.counters.unlisten,
    },
  }),
);
