import { readdir, readFile } from "node:fs/promises";
import { isAbsolute, join, relative, resolve, sep } from "node:path";

import { dependencyReferences, isLonghornPathPackage, parseLock } from "./sync-prototype-locks.ts";
import { rustCommand } from "./rust-command.ts";

type CargoMetadata = {
  packages: {
    id: string;
    name: string;
    version: string;
    source: string | null;
    manifest_path: string;
    dependencies: { name: string; source: string | null; optional: boolean }[];
  }[];
  workspace_members: string[];
};

async function metadataFor(directory: string): Promise<CargoMetadata> {
  const command = rustCommand(["metadata", "--format-version", "1", "--locked", "--offline", "--no-deps"]);
  const process = Bun.spawn(command, { cwd: directory, stdout: "pipe", stderr: "pipe" });
  const [stdout, stderr, exitCode] = await Promise.all([
    new Response(process.stdout).text(),
    new Response(process.stderr).text(),
    process.exited,
  ]);
  if (exitCode !== 0) {
    throw new Error(`${command.join(" ")} failed in ${directory}\n${stdout}\n${stderr}`);
  }
  return JSON.parse(stdout) as CargoMetadata;
}

function isWithin(parent: string, child: string): boolean {
  const path = relative(parent, child);
  return path !== "" && path !== ".." && !path.startsWith(`..${sep}`) && !isAbsolute(path);
}

export async function discoverPrototypeWorkspaceDependencies(repoRoot: string): Promise<string[]> {
  const prototypesRoot = join(repoRoot, "prototypes");
  const cratesRoot = resolve(repoRoot, "crates");
  const entries = await readdir(prototypesRoot, { withFileTypes: true });
  const prototypeDirs = entries
    .filter((entry) => entry.isDirectory())
    .map((entry) => resolve(prototypesRoot, entry.name))
    .sort();
  const dependencies = new Set<string>();
  const workspaceMetadata = await metadataFor(repoRoot);
  const workspaceCrates = new Set(
    workspaceMetadata.packages
      .filter((pkg) => isWithin(cratesRoot, resolve(pkg.manifest_path)))
      .map((pkg) => pkg.name),
  );

  for (const directory of prototypeDirs) {
    const metadata = await metadataFor(directory);
    const packageById = new Map(metadata.packages.map((pkg) => [pkg.id, pkg]));
    const lockText = await readFile(join(directory, "Cargo.lock"), "utf8");
    const lockPackages = parseLock(lockText).packages;
    const pending: typeof lockPackages = [];
    const visited = new Set<string>();

    for (const memberId of metadata.workspace_members) {
      const member = packageById.get(memberId);
      if (!member) throw new Error(`prototype metadata omitted workspace member ${memberId}`);
      const lockPackage = lockPackages.find((pkg) => pkg.name === member.name && pkg.version === member.version && pkg.source === undefined);
      if (!lockPackage) throw new Error(`${relative(repoRoot, directory)}/Cargo.lock is missing workspace package ${member.name}`);
      pending.push(lockPackage);

      for (const dependency of member.dependencies) {
        if (dependency.optional || dependency.source !== null || !workspaceCrates.has(dependency.name)) continue;
        if (!lockPackages.some((pkg) => pkg.name === dependency.name && pkg.source === undefined)) {
          throw new Error(`${relative(repoRoot, directory)}/Cargo.lock is missing Longhorn path package ${dependency.name}`);
        }
      }
    }

    while (pending.length > 0) {
      const pkg = pending.pop()!;
      const key = `${pkg.name}\0${pkg.version}\0${pkg.source ?? "path"}`;
      if (visited.has(key)) continue;
      visited.add(key);

      if (isLonghornPathPackage(pkg) && workspaceCrates.has(pkg.name)) dependencies.add(pkg.name);

      for (const reference of dependencyReferences(pkg)) {
        const dependency = lockPackages.find((candidate) => candidate.name === reference.name &&
          (reference.version === undefined || candidate.version === reference.version));
        if (!dependency) {
          throw new Error(`${relative(repoRoot, directory)}/Cargo.lock is missing package ${reference.name}`);
        }
        if (dependency.source === undefined) pending.push(dependency);
      }
    }
  }

  return [...dependencies].sort();
}

export function prototypeGroupMemberCoverage(effigyToml: string, memberId: string): string[] {
  const header = /^\[qa\.groups\.prototype-dependencies\]\s*$/m.exec(effigyToml);
  if (!header || header.index === undefined) {
    throw new Error("effigy.toml has no [qa.groups.prototype-dependencies] group");
  }
  const bodyStart = header.index + header[0].length;
  const nextSection = /^\[/m.exec(effigyToml.slice(bodyStart));
  const body = effigyToml.slice(bodyStart, nextSection ? bodyStart + nextSection.index : undefined);
  const idMatch = new RegExp(`\\bid\\s*=\\s*"${memberId}"`).exec(body);
  if (!idMatch || idMatch.index === undefined) {
    throw new Error(`prototype-dependencies QA group has no ${memberId} member`);
  }
  const memberStart = body.lastIndexOf("  {", idMatch.index);
  const memberEnd = body.indexOf("\n  },", idMatch.index);
  if (memberStart < 0 || memberEnd < 0) {
    throw new Error(`could not read ${memberId} member in prototype-dependencies QA group`);
  }
  const member = body.slice(memberStart, memberEnd + 5);
  const covered = new Set<string>();
  for (const [, covers] of member.matchAll(/\bcovers\s*=\s*\[([\s\S]*?)\]/g)) {
    for (const [, packageName] of covers!.matchAll(/"cargo-package:([^"]+)"/g)) covered.add(packageName!);
  }
  return [...covered].sort();
}

export function prototypeCoverageFailures(expected: string[], actual: string[]): string[] {
  const expectedSet = new Set(expected);
  const actualSet = new Set(actual);
  return [
    ...expected.filter((name) => !actualSet.has(name)).map((name) => `missing cargo-package:${name}`),
    ...actual.filter((name) => !expectedSet.has(name)).map((name) => `unexpected cargo-package:${name}`),
  ];
}
