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

import { fileDependency, run, testCount } from "./shared.ts";
import { POODLE_RELEASE } from "./artifacts.ts";
import type {
  PackageManifest,
  ProofContext,
  ShapePolicy,
} from "./types.ts";

const shapes: Record<string, ShapePolicy> = {
  "split-shell": {
    longhorn: ["@inflatable-cookie/longhorn"],
    forbiddenImports: [
      "@inflatable-cookie/longhorn/config",
      "@inflatable-cookie/longhorn/layout",
      "@inflatable-cookie/longhorn/surfaces",
      "@inflatable-cookie/longhorn/commands",
    ],
    permissions: [
      "allow-longhorn-settings-read",
      "allow-longhorn-settings-mutate",
      "core:event:allow-listen",
      "core:event:allow-unlisten",
    ],
    pages: ["Preferences"],
  },
  soundcheck: {
    longhorn: ["@inflatable-cookie/longhorn"],
    forbiddenImports: [
      "@inflatable-cookie/longhorn/layout",
      "@inflatable-cookie/longhorn/surfaces",
      "@inflatable-cookie/longhorn/commands",
    ],
    permissions: [
      "allow-longhorn-settings-read",
      "allow-longhorn-settings-mutate",
      "allow-longhorn-config-read",
      "allow-longhorn-storage-mutate",
      "allow-longhorn-backup-mutate",
      "allow-longhorn-restore-mutate",
      "core:event:allow-listen",
      "core:event:allow-unlisten",
    ],
    pages: ["Audio", "Storage", "Backups", "Restore & Recovery"],
  },
  loophole: {
    longhorn: ["@inflatable-cookie/longhorn"],
    forbiddenImports: [
      "@inflatable-cookie/longhorn/config",
      "@inflatable-cookie/longhorn/layout",
      "@inflatable-cookie/longhorn/surfaces",
      "@inflatable-cookie/longhorn/commands",
    ],
    permissions: [
      "allow-longhorn-settings-read",
      "allow-longhorn-settings-mutate",
      "core:event:allow-listen",
      "core:event:allow-unlisten",
      "loophole:allow-hardware-probe",
      "loophole:allow-keybinding-editor",
    ],
    pages: ["Application", "Appearance", "Hardware", "Keybindings"],
  },
  nucleus: {
    longhorn: ["@inflatable-cookie/longhorn"],
    forbiddenImports: [
      "@inflatable-cookie/longhorn/config",
      "@inflatable-cookie/longhorn/surfaces",
      "@inflatable-cookie/longhorn/surface-transfer",
      "@inflatable-cookie/longhorn/commands",
    ],
    permissions: [
      "allow-longhorn-settings-read",
      "allow-longhorn-settings-mutate",
      "core:event:allow-listen",
      "core:event:allow-unlisten",
    ],
    pages: ["General"],
  },
};

export async function verifyConsumers(context: ProofContext) {
  const reports = [];
  for (const [shape, policy] of Object.entries(shapes)) {
    reports.push(await verifyConsumer(context, shape, policy));
  }
  return reports;
}

async function verifyConsumer(
  context: ProofContext,
  shape: string,
  policy: ShapePolicy,
) {
  const source = join(context.proofRoot, shape);
  const stage = join(
    context.temporaryRoot,
    `consumer-${shape}-${randomUUID()}`,
  );
  await mkdir(join(stage, "src", "fixtures"), { recursive: true });
  for (const filename of await readdir(join(context.proofRoot, "common", "src"))) {
    if (filename === "config-proof.ts" && shape !== "soundcheck") continue;
    await cp(
      join(context.proofRoot, "common", "src", filename),
      join(stage, "src", filename),
      { recursive: true },
    );
  }
  for (const filename of await readdir(join(source, "src"))) {
    await cp(join(source, "src", filename), join(stage, "src", filename), {
      recursive: true,
    });
  }
  await cp(join(source, "capability.json"), join(stage, "capability.json"));
  for (const filename of ["tsconfig.json", "vitest.config.ts", "setup.ts"]) {
    await cp(
      join(context.proofRoot, "common", filename),
      join(stage, filename),
    );
  }
  await cp(
    join(context.repoRoot, "fixtures/settings/protocol-v1.json"),
    join(stage, "src/fixtures/settings-protocol-v1.json"),
  );
  await cp(
    join(context.repoRoot, "fixtures/config/protocol-v1.json"),
    join(stage, "src/fixtures/config-protocol-v1.json"),
  );

  const manifest = JSON.parse(
    await readFile(join(source, "package.json"), "utf8"),
  ) as PackageManifest;
  manifest.dependencies = rewriteDependencies(
    manifest.dependencies,
    context.artifacts,
  );
  // Only Longhorn's own packs are overridden onto paths. Poodle is a published
  // dependency, so the staged consumer resolves it from the registry exactly as
  // a real consumer does -- which is what the pack indirection was standing in
  // for before Poodle shipped.
  manifest.overrides = Object.fromEntries(
    [...context.artifacts].map(([name, path]) => [name, fileDependency(path)]),
  );
  await writeFile(
    join(stage, "package.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );

  await run(["bun", "install", "--ignore-scripts"], stage);
  await run(["bun", "x", "svelte-check", "--tsconfig", "./tsconfig.json"], stage);
  const testOutput = await run(
    ["bun", "x", "vitest", "run", "--config", "./vitest.config.ts"],
    stage,
  );

  const resolved = [];
  for (const name of policy.longhorn) {
    resolved.push(await assertArtifactInstall(stage, name));
  }
  for (const pkg of context.poodle.packages) {
    await assertArtifactInstall(stage, pkg.name);
  }
  // Card 164 collapsed the domains into one package, so "this consumer does
  // not install layout" is no longer expressible or true — layout ships in
  // @inflatable-cookie/longhorn whether composed or not, and tree-shaking is
  // what keeps it out of a bundle. The half that still holds is that the
  // consumer never imports it.
  await assertImportsAbsent(stage, policy.forbiddenImports);
  await assertLonghornGraphExact(stage);
  const svelte = await installedPackage(stage, "svelte");
  if (svelte.manifest.version !== "5.38.6") {
    throw new Error(`${shape} installed unexpected Svelte version`);
  }
  await assertSingleSvelteRuntime(stage);
  const capability = JSON.parse(
    await readFile(join(stage, "capability.json"), "utf8"),
  ) as { readonly permissions: readonly string[] };
  assertExactSet(`${shape} permissions`, capability.permissions, policy.permissions);
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
    pages: policy.pages,
    longhornPackages: policy.longhorn,
    forbiddenImportsAbsent: policy.forbiddenImports,
    permissions: policy.permissions,
    artifactResolution: resolved,
    poodleVersion: POODLE_RELEASE.version,
    svelte: svelte.manifest.version,
    mountedTests: testCount(testOutput),
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
  return { path, realPath: await realpath(path), manifest };
}

async function assertImportsAbsent(
  stage: string,
  specifiers: readonly string[],
): Promise<void> {
  const files = (await readdir(join(stage, "src"), { recursive: true })).filter(
    (path) => /\.(ts|svelte)$/.test(path),
  );
  const sources = await Promise.all(
    files.map((path) => readFile(join(stage, "src", path), "utf8")),
  );
  const joined = sources.join("\n");
  for (const specifier of specifiers) {
    if (joined.includes(specifier)) {
      throw new Error(`${specifier} unexpectedly imported by the consumer`);
    }
  }
}

async function assertLonghornGraphExact(stage: string): Promise<void> {
  const scope = join(stage, "node_modules", "@inflatable-cookie");
  const installed = (await readdir(scope))
    .filter((entry) => entry === "longhorn" || entry.startsWith("longhorn-"))
    .sort();
  const expected = [
    "longhorn",
    "longhorn-poodle-svelte",
  ];
  if (JSON.stringify(installed) !== JSON.stringify(expected)) {
    throw new Error(`install graph carries ${installed.join(", ")}`);
  }
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

function assertExactSet(
  label: string,
  actual: readonly string[],
  expected: readonly string[],
): void {
  const left = [...actual].sort();
  const right = [...expected].sort();
  if (JSON.stringify(left) !== JSON.stringify(right)) {
    throw new Error(`${label} mismatch: ${left.join(", ")}`);
  }
}
