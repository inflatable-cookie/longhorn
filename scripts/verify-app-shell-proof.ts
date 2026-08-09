import { assertImportsAbsent, assertPackageAbsent, splitForbidden } from "./consumer-absence.ts";
import { poodleEvidence } from "./poodle-evidence.ts";
import { createHash, randomUUID } from "node:crypto";
import {
  cp,
  lstat,
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  realpath,
  rm,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");
const proofRoot = join(repoRoot, "examples/app-shell-proof");
const expectedPoodleSet = poodleEvidence().artifactSetId;
const poodleEvidencePath = resolve(
  process.env.POODLE_CARD_038_EVIDENCE ?? poodleEvidence().evidencePath,
);

const longhornPackages = [
  ["@inflatable-cookie/longhorn", "longhorn"],
  ["@inflatable-cookie/longhorn-poodle-svelte", "longhorn-poodle-svelte"],
] as const;

const shapes = {
  "split-shell": {
    longhorn: ["@inflatable-cookie/longhorn", "@inflatable-cookie/longhorn-poodle-svelte"],
    forbidden: ["@inflatable-cookie/longhorn/layout", "@inflatable-cookie/longhorn-poodle-svelte/poodle", "@inflatable-cookie/longhorn/surfaces", "@inflatable-cookie/longhorn/transfer", "@inflatable-cookie/longhorn/surface-transfer"],
    regions: 0,
  },
  nucleus: {
    longhorn: ["@inflatable-cookie/longhorn", "@inflatable-cookie/longhorn-poodle-svelte"],
    forbidden: ["@inflatable-cookie/longhorn/surfaces", "@inflatable-cookie/longhorn/surface-transfer"],
    regions: 5,
  },
  loophole: {
    longhorn: longhornPackages.map(([name]) => name),
    forbidden: [],
    regions: 8,
  },
} as const;

const temporaryRoot = await mkdtemp(join(tmpdir(), "longhorn-shell-proof-"));
const artifactRoot = join(temporaryRoot, "artifacts");
await mkdir(artifactRoot);

try {
  const poodleEvidence = await readPoodleEvidence();
  const artifactPaths = new Map<string, string>();
  const artifactIdentities: ArtifactIdentity[] = [];

  for (const [name, directory] of longhornPackages) {
    const filename = `${name.replace("@", "").replace("/", "-")}-0.1.0.tgz`;
    const path = join(artifactRoot, filename);
    await run(
      [
        "bun",
        "pm",
        "pack",
        "--destination",
        artifactRoot,
        "--ignore-scripts",
        "--quiet",
      ],
      join(repoRoot, "packages", directory),
    );
    artifactPaths.set(name, path);
    artifactIdentities.push(await inspectArtifact(name, path));
  }

  for (const artifact of poodleEvidence.artifacts) {
    artifactPaths.set(
      artifact.name,
      resolve(poodleEvidence.packDirectory, artifact.filename),
    );
  }

  const shapeReports = [];
  for (const [shape, policy] of Object.entries(shapes)) {
    const source = join(proofRoot, shape);
    const stage = join(temporaryRoot, `consumer-${shape}-${randomUUID()}`);
    await mkdir(stage);
    await cp(join(source, "src"), join(stage, "src"), { recursive: true });
    await cp(join(source, "capability.json"), join(stage, "capability.json"));
    for (const filename of ["tsconfig.json", "vitest.config.ts", "setup.ts"]) {
      await cp(join(proofRoot, "common", filename), join(stage, filename));
    }

    const manifest = JSON.parse(
      await readFile(join(source, "package.json"), "utf8"),
    ) as PackageManifest;
    manifest.dependencies = rewriteDependencies(
      manifest.dependencies,
      artifactPaths,
    );
    manifest.overrides = Object.fromEntries(
      [...artifactPaths].map(([name, path]) => [name, fileDependency(path)]),
    );
    await writeFile(
      join(stage, "package.json"),
      `${JSON.stringify(manifest, null, 2)}\n`,
    );

    await run(["bun", "install", "--ignore-scripts"], stage);
    await run(
      ["bun", "x", "svelte-check", "--tsconfig", "./tsconfig.json"],
      stage,
    );
    const testOutput = await run(
      ["bun", "x", "vitest", "run", "--config", "./vitest.config.ts"],
      stage,
    );

    const resolved = [];
    for (const name of policy.longhorn) {
      resolved.push(await assertArtifactInstall(stage, name, repoRoot));
    }
    const poodleResolved = [];
    for (const artifact of poodleEvidence.artifacts) {
      poodleResolved.push(
        await assertArtifactInstall(stage, artifact.name, repoRoot),
      );
    }
    // Card 164: a domain is a subpath now, so install-absence for it would
    // pass vacuously. Packages keep install-absence; subpaths become
    // import-absence. See scripts/consumer-absence.ts.
    const forbidden = splitForbidden(policy.forbidden);
    for (const name of forbidden.packages) {
      await assertPackageAbsent(stage, name);
    }
    await assertImportsAbsent(stage, forbidden.imports);
    const svelte = await installedPackage(stage, "svelte");
    if (svelte.manifest.version !== "5.38.6") {
      throw new Error(
        `${shape} installed unexpected Svelte ${svelte.manifest.version}`,
      );
    }
    await assertSingleSvelteRuntime(stage);
    const capability = JSON.parse(
      await readFile(join(stage, "capability.json"), "utf8"),
    ) as {
      readonly permissions: readonly string[];
    };
    assertCapabilityPolicy(shape, capability.permissions);
    const lock = await readFile(join(stage, "bun.lock"), "utf8");
    if (
      lock.includes("workspace:") ||
      lock.includes("link:") ||
      lock.includes(resolve(repoRoot, "packages"))
    ) {
      throw new Error(`${shape} lockfile contains workspace/source resolution`);
    }

    shapeReports.push({
      shape,
      longhornPackages: policy.longhorn,
      forbiddenPackagesAbsent: policy.forbidden,
      poodleArtifactSet: expectedPoodleSet,
      svelte: svelte.manifest.version,
      regions: policy.regions,
      capabilityPermissions: capability.permissions,
      artifactResolution: resolved,
      poodleArtifactResolution: poodleResolved,
      mountedTests: testCount(testOutput),
      cleanInstall: true,
      siblingSourceAliases: false,
    });
  }

  console.log(
    JSON.stringify(
      {
        schema: "longhorn.app-shell-artifact-proof.v1",
        poodleArtifactSet: expectedPoodleSet,
        longhornArtifacts: artifactIdentities,
        shapes: shapeReports,
        duplicatePeerRuntime: false,
        upwardOptionalEdge: false,
      },
      null,
      2,
    ),
  );
} finally {
  if (process.env.KEEP_APP_SHELL_PROOF === "1") {
    console.error(`retained proof workspace: ${temporaryRoot}`);
  } else {
    await rm(temporaryRoot, { recursive: true, force: true });
  }
}

async function readPoodleEvidence(): Promise<PoodleEvidence> {
  const raw = JSON.parse(
    await readFile(poodleEvidencePath, "utf8"),
  ) as PoodleEvidenceFile;
  if (raw.artifactSetId !== expectedPoodleSet) {
    throw new Error(
      `Poodle artifact set mismatch: ${raw.artifactSetId}`,
    );
  }
  const packDirectory = join(resolve(poodleEvidencePath, ".."), "packs");
  const membership = [];
  for (const artifact of raw.artifacts) {
    const path = join(packDirectory, artifact.filename);
    const sha256 = await digest(path);
    if (sha256 !== artifact.sha256) {
      throw new Error(`${artifact.name} Poodle artifact digest mismatch`);
    }
    membership.push(`${artifact.name}:${sha256}`);
  }
  const setId = createHash("sha256")
    .update(membership.join("\n"))
    .digest("hex");
  if (setId !== expectedPoodleSet) {
    throw new Error(`Poodle artifact membership mismatch: ${setId}`);
  }
  return {
    artifacts: raw.artifacts,
    packDirectory,
  };
}

async function inspectArtifact(
  name: string,
  path: string,
): Promise<ArtifactIdentity> {
  const listing = await run(["tar", "-tzf", path], repoRoot);
  if (
    listing.includes("node_modules/") ||
    listing.includes("/tests/") ||
    listing.includes("workspace:")
  ) {
    throw new Error(`${name} artifact contains non-package proof material`);
  }
  const packedManifest = await run(
    ["tar", "-xOzf", path, "package/package.json"],
    repoRoot,
  );
  if (
    packedManifest.includes("workspace:") ||
    packedManifest.includes("link:")
  ) {
    throw new Error(`${name} artifact contains workspace dependency aliases`);
  }
  return {
    name,
    filename: basename(path),
    sha256: await digest(path),
  };
}

function rewriteDependencies(
  dependencies: Record<string, string>,
  artifacts: ReadonlyMap<string, string>,
): Record<string, string> {
  return Object.fromEntries(
    Object.entries(dependencies).map(([name, version]) => [
      name,
      artifacts.has(name) ? fileDependency(artifacts.get(name)!) : version,
    ]),
  );
}

function fileDependency(path: string): string {
  return `file:${path}`;
}

async function assertArtifactInstall(
  stage: string,
  name: string,
  sourceRoot: string,
): Promise<{ name: string; version: string }> {
  const installed = await installedPackage(stage, name);
  const sourcePackages = join(sourceRoot, "packages");
  if (
    installed.realPath.startsWith(sourcePackages) ||
    installed.realPath.includes("/Dev/projects/poodle/packages/")
  ) {
    throw new Error(`${name} resolved to sibling source: ${installed.realPath}`);
  }
  if (installed.manifest.version !== "0.1.0") {
    throw new Error(`${name} installed unexpected version`);
  }
  const entry = await lstat(installed.path);
  if (!entry.isDirectory() && !entry.isSymbolicLink()) {
    throw new Error(`${name} has an invalid install entry`);
  }
  return { name, version: installed.manifest.version };
}

async function assertSingleSvelteRuntime(stage: string): Promise<void> {
  const nodeModules = join(stage, "node_modules");
  const manifests = (await readdir(nodeModules, { recursive: true }))
    .filter(
      (path) =>
        path === "svelte/package.json" ||
        path.endsWith("/node_modules/svelte/package.json"),
    );
  if (manifests.length !== 1) {
    throw new Error(
      `expected one Svelte runtime, found ${manifests.length}: ${manifests.join(", ")}`,
    );
  }
}

async function installedPackage(stage: string, name: string) {
  const path = join(stage, "node_modules", ...name.split("/"));
  const manifest = JSON.parse(
    await readFile(join(path, "package.json"), "utf8"),
  ) as { readonly name: string; readonly version: string };
  if (manifest.name !== name) {
    throw new Error(`installed package identity mismatch for ${name}`);
  }
  return {
    path,
    realPath: await realpath(path),
    manifest,
  };
}


function assertCapabilityPolicy(
  shape: string,
  permissions: readonly string[],
): void {
  const permissionSet = new Set(permissions);
  const base = permissionSet.has("allow-longhorn-transfer-base");
  const surfaces = permissionSet.has("allow-longhorn-transfer-surface");
  if (shape === "split-shell" && (base || surfaces)) {
    throw new Error("Split-shell capability includes transfer authority");
  }
  if (shape === "nucleus" && surfaces) {
    throw new Error("Nucleus capability includes Surface transfer authority");
  }
  if (shape === "loophole" && (!base || !surfaces)) {
    throw new Error("Loophole capability omits full transfer authority");
  }
}

async function digest(path: string): Promise<string> {
  return createHash("sha256")
    .update(await readFile(path))
    .digest("hex");
}

async function run(command: string[], cwd: string): Promise<string> {
  const process = Bun.spawn(command, {
    cwd,
    stdout: "pipe",
    stderr: "pipe",
  });
  const [exitCode, stdout, stderr] = await Promise.all([
    process.exited,
    new Response(process.stdout).text(),
    new Response(process.stderr).text(),
  ]);
  if (exitCode !== 0) {
    throw new Error(
      `${command.join(" ")} failed in ${cwd}\n${stdout}\n${stderr}`,
    );
  }
  return `${stdout}\n${stderr}`;
}

function testCount(output: string): number {
  const match = output.match(/Tests\s+(\d+) passed/);
  return match ? Number(match[1]) : 0;
}

interface PackageManifest {
  dependencies: Record<string, string>;
  overrides?: Record<string, string>;
}

interface PoodleEvidenceFile {
  readonly artifactSetId: string;
  readonly artifacts: readonly {
    readonly name: string;
    readonly filename: string;
    readonly sha256: string;
  }[];
}

interface PoodleEvidence {
  readonly artifacts: PoodleEvidenceFile["artifacts"];
  readonly packDirectory: string;
}

interface ArtifactIdentity {
  readonly name: string;
  readonly filename: string;
  readonly sha256: string;
}
