import type { PoodleRelease } from "../poodle-release.ts";
export interface ArtifactIdentity {
  readonly name: string;
  readonly filename: string;
  readonly sha256: string;
}

export interface ProofContext {
  readonly repoRoot: string;
  readonly proofRoot: string;
  readonly temporaryRoot: string;
  readonly artifacts: ReadonlyMap<string, string>;
  readonly poodle: PoodleRelease;
}

export type ShapeName = "jetstream" | "loophole";
