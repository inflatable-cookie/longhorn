import { installAgentControlShim } from "./shim.ts";
import type { ShimWorld } from "./shim.ts";

const target = globalThis as unknown as ShimWorld;
installAgentControlShim(target);
