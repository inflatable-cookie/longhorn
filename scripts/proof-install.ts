import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
type Lock = { packages: Record<string, [string, string, unknown, string?]> };
const rootLock = Bun.JSONC.parse(
  await readFile(join(root, "bun.lock"), "utf8"),
) as Lock;

function registryVersions(lock: Lock): Map<string, Set<string>> {
  const versions = new Map<string, Set<string>>();
  for (const entry of Object.values(lock.packages)) {
    if (!entry[3]?.startsWith("sha512-")) continue;
    const at = entry[0].lastIndexOf("@");
    const name = entry[0].slice(0, at);
    const version = entry[0].slice(at + 1);
    if (!versions.has(name)) versions.set(name, new Set());
    versions.get(name)!.add(version);
  }
  return versions;
}

const lockedVersions = registryVersions(rootLock);

/** Install packed artifacts with the root lock's third-party resolutions. */
export async function installProofStage(
  stage: string,
  floating: readonly string[] = [],
): Promise<void> {
  const manifestPath = join(stage, "package.json");
  const manifest = JSON.parse(await readFile(manifestPath, "utf8")) as {
    dependencies?: Record<string, string>;
    devDependencies?: Record<string, string>;
    overrides?: Record<string, string>;
  };
  const excluded = new Set(floating);
  const direct = { ...manifest.dependencies, ...manifest.devDependencies };
  const pins: Record<string, string> = {};
  for (const [name, versions] of lockedVersions) {
    // An exact consumer pin is itself deterministic, even if it predates the root lock.
    if (
      !excluded.has(name) &&
      !/^\d+\.\d+\.\d+(?:-[\w.-]+)?$/.test(direct[name] ?? "")
    ) {
      // Prefer the root's top-level resolution where it has nested variants.
      const topLevel = rootLock.packages[name]?.[0];
      pins[name] = topLevel?.slice(topLevel.lastIndexOf("@") + 1) ?? [...versions][0]!;
    }
  }
  manifest.overrides = { ...pins, ...manifest.overrides };
  await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);

  // Some proofs scan every file under the stage; keep Bun's cache outside it.
  const cacheDir = await mkdtemp(join(tmpdir(), "longhorn-proof-bun-cache-"));
  try {
    const command = ["bun", "install", "--ignore-scripts"];
    const child = Bun.spawn(command, {
      cwd: stage,
      env: { ...process.env, BUN_INSTALL_CACHE_DIR: cacheDir },
      stdout: "pipe",
      stderr: "pipe",
    });
    const [code, stdout, stderr] = await Promise.all([
      child.exited,
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
    ]);
    if (code !== 0) {
      throw new Error(`${command.join(" ")} failed in ${stage}\n${stdout}\n${stderr}`);
    }
  } finally {
    await rm(cacheDir, { recursive: true, force: true });
  }

  const staged = Bun.JSONC.parse(
    await readFile(join(stage, "bun.lock"), "utf8"),
  ) as Lock;
  for (const [name, versions] of registryVersions(staged)) {
    if (excluded.has(name)) continue;
    const rootVersions = lockedVersions.get(name);
    if (!rootVersions) {
      throw new Error(`${stage} resolved ${name} absent from the root lock`);
    }
    for (const version of versions) {
      if (
        ![...rootVersions].some((rootVersion) =>
          Bun.semver.satisfies(version, `<=${rootVersion}`),
        )
      ) {
        throw new Error(`${stage} resolved ${name}@${version} newer than the root lock`);
      }
    }
  }
}
