import { assertImportsAbsent, assertPackageAbsent, splitForbidden } from "../consumer-absence.ts";
import { randomUUID } from "node:crypto";
import { join, resolve } from "node:path";
import {
  cp,
  lstat,
  mkdir,
  readFile,
  readdir,
  realpath,
  writeFile,
} from "node:fs/promises";

import {
  assertExactSet,
  fileDependency,
  parseTrace,
  run,
  testCount,
} from "./shared.ts";
import { POODLE_ARTIFACT_SET } from "./artifacts.ts";
import type {
  PackageManifest,
  ProofContext,
  ShapeName,
} from "./types.ts";

const policies = {
  jetstream: {
    longhorn: ["@inflatable-cookie/longhorn"],
    imports: ["@inflatable-cookie/longhorn/commands"],
    permissions: ["allow-longhorn-command-read"],
    forbidden: ["@inflatable-cookie/longhorn/core", "@inflatable-cookie/longhorn/settings", "@inflatable-cookie/longhorn/config", "@inflatable-cookie/longhorn/bridge", "@inflatable-cookie/longhorn-tauri"],
    mountedTests: 0,
  },
  loophole: {
    longhorn: ["@inflatable-cookie/longhorn-poodle-svelte", "@inflatable-cookie/longhorn"],
    imports: ["@inflatable-cookie/longhorn-poodle-svelte/commands/poodle", "@inflatable-cookie/longhorn-poodle-svelte/commands/svelte", "@inflatable-cookie/longhorn/commands", "@inflatable-cookie/longhorn/settings"],
    permissions: [
      "allow-longhorn-command-read",
      "allow-longhorn-command-mutate",
      "core:event:allow-listen",
      "core:event:allow-unlisten",
    ],
    forbidden: ["@inflatable-cookie/longhorn/config", "@inflatable-cookie/longhorn/bridge", "@inflatable-cookie/longhorn-tauri"],
    mountedTests: 1,
  },
} as const;

export async function verifyConsumers(context: ProofContext) {
  return Promise.all(
    (["jetstream", "loophole"] as const).map((shape) =>
      verifyConsumer(context, shape)
    ),
  );
}

async function verifyConsumer(context: ProofContext, shape: ShapeName) {
  const policy = policies[shape];
  const source = join(context.proofRoot, "consumers", shape);
  const stage = join(
    context.temporaryRoot,
    `consumer-${shape}-${randomUUID()}`,
  );
  await mkdir(join(stage, "consumers"), { recursive: true });
  await cp(source, join(stage, "consumers", shape), { recursive: true });
  for (const filename of ["common.ts", "consumer-tsconfig.json"]) {
    await cp(join(context.proofRoot, filename), join(stage, filename));
  }
  if (shape === "loophole") {
    for (const filename of ["vitest.config.ts", "setup.ts"]) {
      await cp(join(context.proofRoot, filename), join(stage, filename));
    }
  }

  const manifest = JSON.parse(
    await readFile(join(source, "package.json"), "utf8"),
  ) as PackageManifest;
  manifest.dependencies = rewriteDependencies(
    manifest.dependencies,
    context.artifacts,
  );
  const allArtifacts = new Map(context.artifacts);
  for (const artifact of context.poodle.artifacts) {
    allArtifacts.set(
      artifact.name,
      resolve(context.poodle.packDirectory, artifact.filename),
    );
  }
  manifest.overrides = Object.fromEntries(
    [...allArtifacts].map(([name, path]) => [name, fileDependency(path)]),
  );
  await writeFile(
    join(stage, "package.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );

  await run(["bun", "install", "--ignore-scripts"], stage);
  await run(
    ["bun", "x", "tsc", "-p", "consumer-tsconfig.json"],
    stage,
  );
  let mountedTests = 0;
  if (shape === "loophole") {
    await run(
      [
        "bun",
        "x",
        "svelte-check",
        "--tsconfig",
        "./consumer-tsconfig.json",
      ],
      stage,
    );
    mountedTests = testCount(
      await run(
        ["bun", "x", "vitest", "run", "--config", "./vitest.config.ts"],
        stage,
      ),
    );
  }
  if (mountedTests !== policy.mountedTests) {
    throw new Error(`${shape} mounted test count mismatch: ${mountedTests}`);
  }
  const trace = parseTrace(
    await run(["bun", `consumers/${shape}/proof.ts`], stage),
  );

  const installedLonghorn = (
    await installedScope(stage, "@inflatable-cookie")
  ).filter((name) => name === "longhorn" || name.startsWith("longhorn-"));
  assertExactSet(
    `${shape} installed Longhorn packages`,
    installedLonghorn.map((name) => `@inflatable-cookie/${name}`),
    policy.longhorn,
  );
  const artifactResolution = [];
  for (const name of policy.longhorn) {
    artifactResolution.push(await assertArtifactInstall(stage, name));
  }
  // Card 164: a domain is a subpath now, so install-absence for it would
  // pass vacuously. Packages keep install-absence; subpaths become
  // import-absence. See scripts/consumer-absence.ts.
  const forbidden = splitForbidden(policy.forbidden);
  for (const name of forbidden.packages) {
    await assertPackageAbsent(stage, name);
  }
  await assertImportsAbsent(stage, forbidden.imports);
  if (shape === "loophole") {
    for (const artifact of context.poodle.artifacts) {
      await assertArtifactInstall(stage, artifact.name);
    }
    const svelte = await installedPackage(stage, "svelte");
    if (svelte.manifest.version !== "5.38.6") {
      throw new Error("Loophole installed unexpected Svelte version");
    }
    await assertSingleSvelteRuntime(stage);
  } else {
    const poodle = (await installedScope(stage, "@inflatable-cookie")).filter((name) =>
      name.startsWith("poodle-")
    );
    if (poodle.length !== 0) {
      throw new Error("Jetstream acquired Poodle packages");
    }
  }

  const capability = JSON.parse(
    await readFile(join(stage, "consumers", shape, "capability.json"), "utf8"),
  ) as { readonly permissions: readonly string[] };
  assertExactSet(
    `${shape} permissions`,
    capability.permissions,
    policy.permissions,
  );
  assertExactSet(
    `${shape} imports`,
    await longhornImports(stage),
    policy.imports,
  );
  const lock = await readFile(join(stage, "bun.lock"), "utf8");
  if (
    lock.includes("workspace:") ||
    lock.includes("link:") ||
    lock.includes(resolve(context.repoRoot, "packages")) ||
    lock.includes("/Dev/projects/poodle/packages/")
  ) {
    throw new Error(`${shape} lockfile contains workspace/source resolution`);
  }

  return {
    shape,
    longhornPackages: policy.longhorn,
    imports: policy.imports,
    permissions: policy.permissions,
    forbiddenPackagesAbsent: policy.forbidden,
    artifactResolution,
    poodleArtifactSet:
      shape === "loophole" ? POODLE_ARTIFACT_SET : null,
    mountedTests,
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

async function installedScope(
  stage: string,
  scope: string,
): Promise<readonly string[]> {
  try {
    return (await readdir(join(stage, "node_modules", scope))).sort();
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return [];
    throw error;
  }
}

async function assertArtifactInstall(stage: string, name: string) {
  const installed = await installedPackage(stage, name);
  const sourceRoots = [
    "/Dev/projects/longhorn/packages/",
    "/Dev/projects/poodle/packages/",
  ];
  if (sourceRoots.some((root) => installed.realPath.includes(root))) {
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


async function assertSingleSvelteRuntime(stage: string): Promise<void> {
  const manifests = (await readdir(join(stage, "node_modules"), {
    recursive: true,
  })).filter(
    (path) =>
      path === "svelte/package.json" ||
      path.endsWith("/node_modules/svelte/package.json"),
  );
  if (manifests.length !== 1) {
    throw new Error(`expected one Svelte runtime, found ${manifests.length}`);
  }
}

async function longhornImports(stage: string): Promise<readonly string[]> {
  const files = (await readdir(stage, { recursive: true }))
    .filter(
      (path) =>
        /\.(ts|svelte)$/.test(path) &&
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
