export type ShapeName =
  | "bovine"
  | "jetstream"
  | "soundcheck"
  | "nucleus"
  | "loophole";

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
}

export interface ShapeDeclaration {
  readonly imports: readonly string[];
  readonly tauriPermissions: readonly string[];
  readonly serviceOwnership:
    | "ownedLocal"
    | "externalLocal"
    | "externalRemote"
    | null;
  readonly maximumQueryRetries: number;
}

export interface TopologyDeclarations {
  readonly schema: "longhorn.bridge-topology-declarations.v1";
  readonly shapes: Readonly<Record<ShapeName, ShapeDeclaration>>;
}

export interface ProofContext {
  readonly repoRoot: string;
  readonly proofRoot: string;
  readonly temporaryRoot: string;
  readonly artifacts: ReadonlyMap<string, string>;
  readonly declarations: TopologyDeclarations;
}
