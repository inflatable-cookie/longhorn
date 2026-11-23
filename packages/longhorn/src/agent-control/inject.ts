import { installAgentControlShim } from "./shim.ts";
import type { ShimWorld } from "./shim.ts";

// The standalone IIFE runs in the browser world carrying these DOM globals.
// oxlint-disable-next-line typescript/no-unsafe-type-assertion, quality/no-chained-type-assertions -- The injection host supplies the browser world, including its dynamically installed shim key.
const target = globalThis as unknown as ShimWorld;
installAgentControlShim(target);
