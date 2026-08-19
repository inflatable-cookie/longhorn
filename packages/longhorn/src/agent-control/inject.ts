import { installAgentControlShim, type ShimWorld } from "./shim.ts";

const target = globalThis as unknown as ShimWorld;
installAgentControlShim(target);
