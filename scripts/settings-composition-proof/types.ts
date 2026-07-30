export interface PoodleEvidenceFile {
  readonly artifactSetId: string;
  readonly artifacts: readonly {
    readonly name: string;
    readonly filename: string;
    readonly sha256: string;
  }[];
}

export interface PoodleEvidence {
  readonly artifacts: PoodleEvidenceFile["artifacts"];
  readonly packDirectory: string;
}

export interface ArtifactIdentity {
  readonly name: string;
  readonly filename: string;
  readonly sha256: string;
}

export interface PackageManifest {
  readonly name: string;
  readonly version?: string;
  dependencies: Record<string, string>;
  overrides?: Record<string, string>;
  peerDependencies?: Record<string, string>;
  peerDependenciesMeta?: Record<string, { optional?: boolean }>;
}

export interface ShapePolicy {
  readonly longhorn: readonly string[];
  readonly forbidden: readonly string[];
  readonly permissions: readonly string[];
  readonly host: "modal" | "window" | "panel";
  readonly pages: readonly string[];
}

export interface ProofContext {
  readonly repoRoot: string;
  readonly proofRoot: string;
  readonly temporaryRoot: string;
  readonly artifactRoot: string;
  readonly artifacts: ReadonlyMap<string, string>;
  readonly poodle: PoodleEvidence;
}
