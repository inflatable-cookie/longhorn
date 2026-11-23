import { resolve } from "node:path";

export const repoRoot = resolve(import.meta.dir, "../..");

export function assert(condition: unknown, message: string): asserts condition {
  if (!condition) {
    throw new Error(message);
  }
}
