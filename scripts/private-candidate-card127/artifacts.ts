import { access, mkdir, readFile, readdir } from "node:fs/promises";
import { basename, dirname, join, relative, sep } from "node:path";
import type {
  ArtifactIdentity,
  CandidateArtifacts,
  PackageManifest,
  RustArtifactIdentity,
} from "./types";
import {
  artifactSet,
  assert,
  candidateVersion,
  digest,
  equal,
  hash,
  poodleRoot,
  repoRoot,
  run,
} from "./support";

type CargoPackage = {
  name: string;
  version: string;
  manifest_path: string;
  publish: string[] | null;
};

export const poodlePackages = [
  ["@inflatable-cookie/poodle-headless", "packages/core"],
  ["@inflatable-cookie/poodle-styles", "packages/styles"],
  ["@inflatable-cookie/poodle-svelte-tokens", "packages/svelte/tokens"],
  ["@inflatable-cookie/poodle-icons-lucide", "packages/svelte/icons-lucide"],
  ["@inflatable-cookie/poodle-svelte", "packages/svelte/components"],
] as const;

export async function longhornPackageManifests(): Promise<Map<string, { directory: string; manifest: PackageManifest }>> {
  const result = new Map<string, { directory: string; manifest: PackageManifest }>();
  const entries = await readdir(join(repoRoot, "packages"), { withFileTypes: true });
  for (const entry of entries.filter((candidate) => candidate.isDirectory()).sort((a, b) => a.name.localeCompare(b.name))) {
    const directory = `packages/${entry.name}`;
    const manifest = JSON.parse(await readFile(join(repoRoot, directory, "package.json"), "utf8")) as PackageManifest;
    assert(manifest.name.startsWith("@inflatable-cookie/longhorn-"), `${directory} is not a Longhorn package`);
    equal(manifest.version, candidateVersion, `${manifest.name} version`);
    equal(manifest.private, true, `${manifest.name} private posture`);
    result.set(manifest.name, { directory, manifest });
  }
  equal(result.size, 17, "TypeScript package count");
  return result;
}

export async function rustPackages(): Promise<CargoPackage[]> {
  const metadata = JSON.parse(await run(["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"])) as {
    packages: CargoPackage[];
  };
  const crateRoot = join(repoRoot, "crates") + sep;
  const packages = metadata.packages
    .filter((pkg) => pkg.manifest_path.startsWith(crateRoot) && pkg.name.startsWith("longhorn-"))
    .sort((left, right) => left.name.localeCompare(right.name));
  equal(packages.length, 36, "Rust package count");
  for (const pkg of packages) {
    equal(pkg.version, candidateVersion, `${pkg.name} version`);
    equal(pkg.publish, [], `${pkg.name} private posture`);
  }
  return packages;
}

export async function produceArtifacts(
  temporaryRoot: string,
  longhornCommit: string,
  poodleCommit: string,
): Promise<CandidateArtifacts> {
  const manifests = await longhornPackageManifests();
  const crates = await rustPackages();
  const typescript = await packReproducibleFamily(
    repoRoot,
    [...manifests].map(([name, value]) => [name, value.directory] as const),
    join(temporaryRoot, "typescript"),
  );
  const poodle = await packReproducibleFamily(poodleRoot, poodlePackages, join(temporaryRoot, "poodle"));
  const rust = await produceRustArtifacts(crates, temporaryRoot, longhornCommit);
  await assertCommitContains(poodleRoot, poodleCommit, poodlePackages.map(([, path]) => path));
  return { typescript, poodle, rust };
}

async function packReproducibleFamily(
  root: string,
  packages: readonly (readonly [string, string])[],
  destinationRoot: string,
) {
  const first = await packFamily(root, packages, join(destinationRoot, "first"));
  const second = await packFamily(root, packages, join(destinationRoot, "second"));
  equal(first.identities, second.identities, `${basename(destinationRoot)} reproducibility`);
  return {
    identities: first.identities,
    paths: first.paths,
    setSha256: artifactSet(first.identities),
  };
}

async function packFamily(root: string, packages: readonly (readonly [string, string])[], destination: string) {
  await mkdir(destination, { recursive: true });
  const identities: ArtifactIdentity[] = [];
  const paths = new Map<string, string>();
  for (const [name, directory] of packages) {
    await run(["bun", "pm", "pack", "--destination", destination, "--ignore-scripts", "--quiet"], join(root, directory));
    const path = join(destination, `${name.replace("@", "").replace("/", "-")}-${candidateVersion}.tgz`);
    await access(path);
    const listing = await run(["tar", "-tzf", path], destination);
    const packedManifest = await run(["tar", "-xOzf", path, "package/package.json"], destination);
    assert(!listing.includes("node_modules/"), `${name} artifact contains node_modules`);
    assert(!listing.includes("/tests/"), `${name} artifact contains tests`);
    assert(!/workspace:|link:/.test(packedManifest), `${name} artifact contains a source alias`);
    const manifest = JSON.parse(packedManifest) as PackageManifest;
    equal(manifest.name, name, `${name} packed name`);
    equal(manifest.version, candidateVersion, `${name} packed version`);
    identities.push({ name, version: manifest.version, filename: basename(path), sha256: await digest(path) });
    paths.set(name, path);
  }
  return { identities, paths };
}

async function produceRustArtifacts(packages: CargoPackage[], temporaryRoot: string, commit: string) {
  const firstRoot = join(temporaryRoot, "rust", "first");
  const secondRoot = join(temporaryRoot, "rust", "second");
  await mkdir(firstRoot, { recursive: true });
  await mkdir(secondRoot, { recursive: true });
  const identities: RustArtifactIdentity[] = [];
  const paths = new Map<string, string>();
  for (const pkg of packages) {
    const sourcePath = relative(repoRoot, dirname(pkg.manifest_path)).split(sep).join("/");
    const inventoryA = normalizeInventory(await run(["cargo", "package", "-p", pkg.name, "--list", "--allow-dirty"]));
    const inventoryB = normalizeInventory(await run(["cargo", "package", "-p", pkg.name, "--list", "--allow-dirty"]));
    equal(inventoryA, inventoryB, `${pkg.name} inventory reproducibility`);
    const filename = `${pkg.name}-${candidateVersion}.private.tar.gz`;
    const first = join(firstRoot, filename);
    const second = join(secondRoot, filename);
    await run(["git", "archive", "--format=tar.gz", `--output=${first}`, commit, sourcePath]);
    await run(["git", "archive", "--format=tar.gz", `--output=${second}`, commit, sourcePath]);
    const firstSha = await digest(first);
    equal(firstSha, await digest(second), `${pkg.name} source archive reproducibility`);
    identities.push({
      name: pkg.name,
      version: pkg.version,
      filename,
      sha256: firstSha,
      inventoryFiles: inventoryA.length,
      inventorySha256: hash(inventoryA.join("\n")),
      sourcePath,
    });
    paths.set(pkg.name, first);
  }
  return { identities, paths, setSha256: artifactSet(identities) };
}

function normalizeInventory(output: string): string[] {
  return output.split("\n").map((line) => line.trim()).filter(Boolean);
}

async function assertCommitContains(root: string, commit: string, paths: readonly string[]): Promise<void> {
  for (const path of paths) await run(["git", "cat-file", "-e", `${commit}:${path}`], root);
}
