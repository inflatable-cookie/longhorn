interface PoodlePackage {
  readonly name: string;
  readonly version: string;
  readonly integrity: string;
}

interface PoodleRelease {
  readonly version: string;
  readonly packages: ReadonlyArray<PoodlePackage>;
}

export interface ArtifactIdentity {
  readonly name: string;
  readonly filename: string;
  readonly sha256: string;
}

export interface ShapePolicy {
  readonly longhorn: ReadonlyArray<string>;
  readonly forbiddenImports: ReadonlyArray<string>;
  readonly permissions: ReadonlyArray<string>;
  readonly pages: ReadonlyArray<string>;
}

export interface ProofContext {
  readonly repoRoot: string;
  readonly proofRoot: string;
  readonly temporaryRoot: string;
  readonly artifactRoot: string;
  readonly artifacts: ReadonlyMap<string, string>;
  readonly poodle: PoodleRelease;
}
