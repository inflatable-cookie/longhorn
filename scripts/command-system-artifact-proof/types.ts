export interface ArtifactIdentity {
  readonly name: string;
  readonly filename: string;
  readonly sha256: string;
}

export interface PoodleEvidenceFile {
  readonly artifactSetId: string;
  readonly artifacts: readonly ArtifactIdentity[];
}

export interface PoodleEvidence {
  readonly artifacts: readonly ArtifactIdentity[];
  readonly packDirectory: string;
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
}

export type ShapeName = "jetstream" | "loophole";
