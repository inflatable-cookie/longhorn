export type ArtifactIdentity = {
  name: string;
  version: string;
  filename: string;
  sha256: string;
};

export type RustArtifactIdentity = ArtifactIdentity & {
  inventoryFiles: number;
  inventorySha256: string;
  sourcePath: string;
};

export type PackageManifest = {
  name: string;
  version: string;
  private?: boolean;
  dependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
  peerDependenciesMeta?: Record<string, { optional?: boolean }>;
};

export type CandidateArtifacts = {
  typescript: {
    identities: ArtifactIdentity[];
    paths: Map<string, string>;
    setSha256: string;
  };
  poodle: {
    identities: ArtifactIdentity[];
    paths: Map<string, string>;
    setSha256: string;
  };
  rust: {
    identities: RustArtifactIdentity[];
    paths: Map<string, string>;
    setSha256: string;
  };
};

export type GraphDefinition = {
  name: string;
  repository: string | null;
  sourceCommit: string | null;
  sourceManifests: string[];
  typescript: string[];
  rust: string[];
};

export type GraphProof = {
  name: string;
  sourceCommit: string | null;
  sourceManifests: string[];
  typescriptPackages: string[];
  rustDirectPackages: string[];
  rustResolvedPackages: string[];
  oneSvelteRuntime: boolean;
  oneTauriApiRuntime: boolean;
  siblingResolution: boolean;
  lockedRust185: boolean;
};

export type FrozenSource = {
  branch: string;
  commit: string;
  selectedPathsClean: boolean;
  selectedPaths: string[];
};
