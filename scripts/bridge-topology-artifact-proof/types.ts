export type ShapeName =
  | "minimal-shell"
  | "ordered-streams"
  | "jobs-and-service-failure"
  | "capability-authority"
  | "reconnecting-lifecycle";

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
  readonly declarations: Record<string, unknown>;
}
