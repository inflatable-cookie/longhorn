import type { PoodleRelease } from "../poodle-release.ts";
export type Shape = "minimal-operation" | "soundcheck" | "loophole" | "notification-only";

export interface ArtifactIdentity {
  readonly name: string;
  readonly filename: string;
  readonly sha256: string;
}

export interface PackageManifest {
  readonly name: string;
  dependencies: Record<string, string>;
  overrides?: Record<string, string>;
}

export interface ProofContext {
  readonly repoRoot: string;
  readonly proofRoot: string;
  readonly temporaryRoot: string;
  readonly artifacts: ReadonlyMap<string, string>;
  readonly poodle: PoodleRelease;
  readonly nativeTraces: Record<Shape, Record<string, unknown>>;
}
