import { randomUUID } from "node:crypto";
import { join, resolve } from "node:path";
import {
  cp,
  mkdir,
  readFile,
  readdir,
  realpath,
  writeFile,
} from "node:fs/promises";

import {
  assertExactSet,
  fileDependency,
  run,
} from "./shared.ts";
import type {
  PackageManifest,
  ProofContext,
  ShapeName,
} from "./types.ts";

const shapes: readonly ShapeName[] = [
  "bovine",
  "jetstream",
  "soundcheck",
  "nucleus",
  "loophole",
];

export async function verifyConsumers(context: ProofContext) {
  const reports = [];
  for (const shape of shapes) {
    reports.push(await verifyConsumer(context, shape));
  }
  return reports;
}

async function verifyConsumer(context: ProofContext, shape: ShapeName) {
  const source = join(context.proofRoot, "consumers", shape);
  const stage = join(
    context.temporaryRoot,
    `consumer-${shape}-${randomUUID()}`,
  );
  await mkdir(join(stage, "consumers", shape), { recursive: true });
  for (const filename of [
    "declarations.json",
    "common.ts",
    `${shape}.ts`,
  ]) {
    await cp(join(context.proofRoot, filename), join(stage, filename));
  }
  if (shape === "bovine") {
    await cp(
      join(context.proofRoot, "protocol-artifact.ts"),
      join(stage, "protocol-artifact.ts"),
    );
  }
  if (shape === "loophole") {
    await cp(
      join(context.proofRoot, "lifecycle.ts"),
      join(stage, "lifecycle.ts"),
    );
  }
  await cp(
    join(source, "proof.ts"),
    join(stage, "consumers", shape, "proof.ts"),
  );
  await cp(join(source, "capability.json"), join(stage, "capability.json"));
  await cp(
    join(context.proofRoot, "consumer-tsconfig.json"),
    join(stage, "tsconfig.json"),
  );

  const manifest = JSON.parse(
    await readFile(join(source, "package.json"), "utf8"),
  ) as PackageManifest;
  manifest.dependencies = rewriteDependencies(
    manifest.dependencies,
    context.artifacts,
  );
  manifest.overrides = Object.fromEntries(
    [...context.artifacts].map(([name, path]) => [
      name,
      fileDependency(path),
    ]),
  );
  await writeFile(
    join(stage, "package.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );

  await run(["bun", "install", "--ignore-scripts"], stage);
  await run(["bun", "x", "tsc", "-p", "tsconfig.json"], stage);
  const proofOutput = await run(
    ["bun", `consumers/${shape}/proof.ts`],
    stage,
  );
  const trace = JSON.parse(
    proofOutput.split("\n").find((line) => line.trim().startsWith("{")) ??
      "",
  ) as Record<string, unknown>;

  const installed = (
    await readdir(join(stage, "node_modules", "@inflatable-cookie"))
  )
    .filter((name) => name === "longhorn" || name.startsWith("longhorn-"))
    .map((name) => `@inflatable-cookie/${name}`)
    .sort();
  assertExactSet(
    `${shape} installed Longhorn packages`,
    installed,
    ["@inflatable-cookie/longhorn", "@inflatable-cookie/longhorn-tauri"],
  );
  const artifactResolution = [];
  for (const name of ["@inflatable-cookie/longhorn", "@inflatable-cookie/longhorn-tauri"]) {
    artifactResolution.push(await assertArtifactInstall(stage, name));
  }
  const tauriApi = await installedPackage(stage, "@tauri-apps/api");
  if (tauriApi.manifest.version !== "2.10.1") {
    throw new Error(`${shape} installed unexpected Tauri API version`);
  }

  const declaration = context.declarations.shapes[shape];
  const capability = JSON.parse(
    await readFile(join(stage, "capability.json"), "utf8"),
  ) as { readonly permissions: readonly string[] };
  assertExactSet(
    `${shape} permissions`,
    capability.permissions,
    declaration.tauriPermissions,
  );
  const imports = await longhornImports(stage);
  assertExactSet(`${shape} imports`, imports, declaration.imports);
  const hasEvents = imports.includes("@inflatable-cookie/longhorn-tauri/bridge-events");
  const hasEventPermissions = capability.permissions.includes(
    "core:event:allow-listen",
  ) && capability.permissions.includes("core:event:allow-unlisten");
  if (hasEvents !== hasEventPermissions) {
    throw new Error(`${shape} event imports and permissions diverged`);
  }
  const hasSupervision = imports.includes("@inflatable-cookie/longhorn/bridge/supervision");
  if (hasSupervision !== (declaration.serviceOwnership !== null)) {
    throw new Error(`${shape} supervision import and ownership diverged`);
  }

  const lock = await readFile(join(stage, "bun.lock"), "utf8");
  if (
    lock.includes("workspace:") ||
    lock.includes("link:") ||
    lock.includes(resolve(context.repoRoot, "packages"))
  ) {
    throw new Error(`${shape} lockfile contains workspace/source resolution`);
  }

  return {
    shape,
    imports,
    permissions: capability.permissions,
    serviceOwnership: declaration.serviceOwnership,
    maximumQueryRetries: declaration.maximumQueryRetries,
    artifactResolution,
    tauriApi: tauriApi.manifest.version,
    trace,
    cleanInstall: true,
    siblingSourceAliases: false,
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

async function assertArtifactInstall(stage: string, name: string) {
  const installed = await installedPackage(stage, name);
  if (installed.realPath.includes("/Dev/projects/longhorn/packages/")) {
    throw new Error(`${name} resolved to sibling source: ${installed.realPath}`);
  }
  if (installed.manifest.version !== "0.1.0") {
    throw new Error(`${name} installed unexpected version`);
  }
  return { name, version: installed.manifest.version };
}

async function installedPackage(stage: string, name: string) {
  const path = join(stage, "node_modules", ...name.split("/"));
  const manifest = JSON.parse(
    await readFile(join(path, "package.json"), "utf8"),
  ) as { readonly name: string; readonly version: string };
  if (manifest.name !== name) {
    throw new Error(`installed package identity mismatch for ${name}`);
  }
  return { realPath: await realpath(path), manifest };
}

async function longhornImports(stage: string): Promise<readonly string[]> {
  const files = (await readdir(stage, { recursive: true }))
    .filter(
      (path) =>
        path.endsWith(".ts") &&
        !path.startsWith("node_modules/"),
    );
  const imports = new Set<string>();
  for (const path of files) {
    const source = await readFile(join(stage, path), "utf8");
    for (const match of source.matchAll(
      /from\s+["'](@inflatable-cookie\/longhorn(?:[/-][^"']*)?)["']/g,
    )) {
      imports.add(match[1]!);
    }
  }
  return [...imports].sort();
}
