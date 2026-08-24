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

import { POODLE_RELEASE } from "./artifacts.ts";
// Longhorn's own coordinated version. Poodle carries its own, released
// separately, so the two cannot be one literal any more.
const LONGHORN_VERSION = "0.1.0";
import {
  assertExactSet,
  equalJson,
  fileDependency,
  parseTrace,
  run,
  testCount,
} from "./shared.ts";
import type { PackageManifest, ProofContext, Shape } from "./types.ts";

const shapes: readonly Shape[] = [
  "minimal-operation",
  "soundcheck",
  "loophole",
  "notification-only",
];

const policies = {
  "minimal-operation": {
    longhorn: ["@inflatable-cookie/longhorn"],
    imports: ["@inflatable-cookie/longhorn/operation"],
    permissions: [],
    forbidden: ["@inflatable-cookie/longhorn/bridge", "@inflatable-cookie/longhorn/commands", "@inflatable-cookie/longhorn/config", "@inflatable-cookie/longhorn/history", "@inflatable-cookie/longhorn/notifications", "@inflatable-cookie/longhorn/settings", "@inflatable-cookie/longhorn-tauri"],
    mountedTests: 0,
  },
  soundcheck: {
    longhorn: ["@inflatable-cookie/longhorn-tauri", "@inflatable-cookie/longhorn"],
    imports: ["@inflatable-cookie/longhorn-tauri/operation", "@inflatable-cookie/longhorn/operation"],
    permissions: [
      "allow-longhorn-operation-read",
      "allow-longhorn-operation-manage",
      "allow-longhorn-operation-cancel",
      "core:event:allow-listen",
      "core:event:allow-unlisten",
    ],
    forbidden: ["@inflatable-cookie/longhorn/bridge", "@inflatable-cookie/longhorn/commands", "@inflatable-cookie/longhorn/config", "@inflatable-cookie/longhorn/history", "@inflatable-cookie/longhorn/notifications", "@inflatable-cookie/longhorn/settings"],
    mountedTests: 0,
  },
  loophole: {
    longhorn: ["@inflatable-cookie/longhorn-poodle-svelte", "@inflatable-cookie/longhorn-tauri", "@inflatable-cookie/longhorn"],
    imports: ["@inflatable-cookie/longhorn-poodle-svelte/notifications/poodle", "@inflatable-cookie/longhorn-poodle-svelte/notifications/svelte", "@inflatable-cookie/longhorn-poodle-svelte/operation/poodle", "@inflatable-cookie/longhorn-poodle-svelte/operation/svelte", "@inflatable-cookie/longhorn-tauri/notifications", "@inflatable-cookie/longhorn-tauri/operation", "@inflatable-cookie/longhorn/bridge", "@inflatable-cookie/longhorn/notifications", "@inflatable-cookie/longhorn/operation", "@inflatable-cookie/longhorn/operation/bridge"],
    permissions: [
      "allow-longhorn-operation-read",
      "allow-longhorn-operation-manage",
      "allow-longhorn-operation-cancel",
      "read-notifications",
      "manage-notifications",
      "core:event:allow-listen",
      "core:event:allow-unlisten",
    ],
    forbidden: ["@inflatable-cookie/longhorn/commands", "@inflatable-cookie/longhorn/config", "@inflatable-cookie/longhorn/history", "@inflatable-cookie/longhorn/settings"],
    mountedTests: 1,
  },
  "notification-only": {
    longhorn: ["@inflatable-cookie/longhorn-tauri", "@inflatable-cookie/longhorn"],
    imports: ["@inflatable-cookie/longhorn-tauri/notifications", "@inflatable-cookie/longhorn/notifications"],
    permissions: [
      "read-notifications",
      "manage-notifications",
      "core:event:allow-listen",
      "core:event:allow-unlisten",
    ],
    forbidden: ["@inflatable-cookie/longhorn/bridge", "@inflatable-cookie/longhorn/commands", "@inflatable-cookie/longhorn/config", "@inflatable-cookie/longhorn/history", "@inflatable-cookie/longhorn/operation", "@inflatable-cookie/longhorn/settings"],
    mountedTests: 0,
  },
} as const;

export async function verifyConsumers(context: ProofContext) {
  return Promise.all(shapes.map((shape) => verifyConsumer(context, shape)));
}

async function verifyConsumer(context: ProofContext, shape: Shape) {
  const policy = policies[shape];
  const source = join(context.proofRoot, "consumers", shape);
  const stage = join(context.temporaryRoot, `consumer-${shape}-${randomUUID()}`);
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

  const native = context.nativeTraces[shape];
  await writeFile(
    join(stage, "consumers", shape, "fixture.json"),
    `${JSON.stringify(native.rendererFixture, null, 2)}\n`,
  );

  const manifest = JSON.parse(
    await readFile(join(source, "package.json"), "utf8"),
  ) as PackageManifest;
  manifest.dependencies = rewriteDependencies(manifest.dependencies, context.artifacts);
  // Only Longhorn's own packs are overridden onto paths. Poodle is a published
  // dependency, so the staged consumer resolves it from the registry exactly as
  // a real consumer does -- which is what the pack indirection was standing in
  // for before Poodle shipped.
  manifest.overrides = Object.fromEntries(
    [...context.artifacts].map(([name, path]) => [name, fileDependency(path)]),
  );
  await writeFile(join(stage, "package.json"), `${JSON.stringify(manifest, null, 2)}\n`);

  await run(["bun", "install", "--ignore-scripts"], stage);
  await run(["bun", "x", "tsc", "-p", "consumer-tsconfig.json"], stage);
  let mountedTests = 0;
  if (shape === "loophole") {
    await run(
      ["bun", "x", "svelte-check", "--tsconfig", "./consumer-tsconfig.json"],
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

  const trace = parseTrace(await run(["bun", `consumers/${shape}/proof.ts`], stage));
  if (!equalJson(trace.publicTrace, native.publicTrace)) {
    throw new Error(`${shape} renderer trace diverged from native trace`);
  }

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
    artifactResolution.push(
      await assertArtifactInstall(stage, name, LONGHORN_VERSION),
    );
  }
  for (const name of policy.forbidden) await assertPackageAbsent(stage, name);

  if (shape === "loophole") {
    for (const pkg of context.poodle.packages) {
      await assertArtifactInstall(stage, pkg.name, context.poodle.version);
    }
    const svelte = await installedPackage(stage, "svelte");
    if (svelte.manifest.version !== "5.38.6") {
      throw new Error("Loophole installed unexpected Svelte version");
    }
    await assertSingleSvelteRuntime(stage);
  } else {
    assertExactSet(
      `${shape} installed Poodle packages`,
      (await installedScope(stage, "@inflatable-cookie")).filter((name) =>
        name.startsWith("poodle-")
      ),
      [],
    );
  }

  const capabilityPath = join(stage, "consumers", shape, "capability.json");
  const permissions = await readPermissions(capabilityPath);
  assertExactSet(`${shape} permissions`, permissions, policy.permissions);
  assertExactSet(`${shape} imports`, await longhornImports(stage), policy.imports);
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
    poodleVersion: shape === "loophole" ? POODLE_RELEASE.version : null,
    mountedTests,
    nativeRendererParity: true,
    cleanInstall: true,
    siblingSourceAliases: false,
    trace,
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

async function installedScope(stage: string, scope: string): Promise<readonly string[]> {
  try {
    return (await readdir(join(stage, "node_modules", scope))).sort();
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return [];
    throw error;
  }
}

async function assertArtifactInstall(
  stage: string,
  name: string,
  expected: string,
) {
  const installed = await installedPackage(stage, name);
  for (const root of ["/Dev/projects/longhorn/packages/", "/Dev/projects/poodle/packages/"]) {
    if (installed.realPath.includes(root)) {
      throw new Error(`${name} resolved to sibling source: ${installed.realPath}`);
    }
  }
  // Longhorn and Poodle no longer share a version, so the expectation is the
  // caller's: LONGHORN_VERSION for a Longhorn package, the pinned Poodle
  // release for a Poodle one.
  if (installed.manifest.version !== expected) {
    throw new Error(
      `${name} installed ${installed.manifest.version}, expected ${expected}`,
    );
  }
  return { name, version: installed.manifest.version };
}

async function installedPackage(stage: string, name: string) {
  const path = join(stage, "node_modules", ...name.split("/"));
  const manifest = JSON.parse(await readFile(join(path, "package.json"), "utf8")) as {
    readonly name: string;
    readonly version: string;
  };
  if (manifest.name !== name) throw new Error(`installed package identity mismatch for ${name}`);
  return { realPath: await realpath(path), manifest };
}

async function assertPackageAbsent(stage: string, name: string): Promise<void> {
  try {
    await lstat(join(stage, "node_modules", ...name.split("/")));
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return;
    throw error;
  }
  throw new Error(`${name} unexpectedly entered the install graph`);
}

async function assertSingleSvelteRuntime(stage: string): Promise<void> {
  const manifests = (await readdir(join(stage, "node_modules"), { recursive: true })).filter(
    (path) => path === "svelte/package.json" || path.endsWith("/node_modules/svelte/package.json"),
  );
  if (manifests.length !== 1) {
    throw new Error(`expected one Svelte runtime, found ${manifests.length}`);
  }
}

async function readPermissions(path: string): Promise<readonly string[]> {
  try {
    const capability = JSON.parse(await readFile(path, "utf8")) as {
      readonly permissions: readonly string[];
    };
    return capability.permissions;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return [];
    throw error;
  }
}

async function longhornImports(stage: string): Promise<readonly string[]> {
  const files = (await readdir(stage, { recursive: true })).filter(
    (path) => /\.(ts|svelte)$/.test(path) && !path.startsWith("node_modules/"),
  );
  const imports = new Set<string>();
  for (const path of files) {
    const source = await readFile(join(stage, path), "utf8");
    for (const match of source.matchAll(/from\s+["'](@inflatable-cookie\/longhorn(?:[/-][^"']*)?)["']/g)) {
      imports.add(match[1]!);
    }
  }
  return [...imports].sort();
}
