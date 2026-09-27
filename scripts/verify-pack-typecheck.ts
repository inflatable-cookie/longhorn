// Pack-level typecheck of `longhorn-poodle-svelte` against registry Poodle.
//
// The workspace `check:svelte` typechecks the adapter against the tree's own
// node_modules, which `bun.lock` pins to the registry -- but it proves nothing
// about what a consumer installing the *packed* package gets. This proof packs
// the published packages, installs them into a bare staging project whose only
// Poodle source is the registry, and typechecks the whole adapter surface
// there. If the published peer ranges resolve from npm and the types actually
// line up, the first publish cannot ship a peer declaration npm cannot honour.
//
// What makes it pack-level rather than a quieter copy of `check:svelte`:
//
// - Longhorn and the adapter enter the stage as `bun pm pack` tarballs, the
//   exact bytes `release.yml` publishes.
// - Poodle enters only as the pinned registry release (`poodleRelease()`),
//   never a sibling checkout or `POODLE_REPO`; the staged lockfile's integrity
//   must equal the root lockfile's, so the typecheck runs against the same
//   bytes every other proof verifies.
// - Nothing in the stage can see the repository's node_modules. A global link
//   or sibling path that made a local gate green shows up here as a resolution
//   outside the stage.
//
// The adapter's Svelte peer is a range, so one install cannot cover it: the
// workspace dev pin and every example pin Svelte `5.56.8`, while a stage that
// asks npm for the range verbatim resolves the newest release the range admits.
// This proof therefore stages the adapter twice -- once with the declared
// range, once at the range's floor -- and typechecks both against registry
// Poodle. Both resolved versions are reported in the envelope, so the claim
// names the releases it was proved on instead of leaving them implied.
//
// Temporary by design: stages live under `mkdtemp` and are deleted. Keep one
// for inspection with KEEP_PACK_TYPECHECK=1.

import { spawnSync } from "node:child_process";
import {
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  realpath,
  rm,
  stat,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, relative, resolve } from "node:path";

import { poodleRelease } from "./poodle-release.ts";

const repoRoot = resolve(import.meta.dir, "..");
const ADAPTER = "@inflatable-cookie/longhorn-poodle-svelte";
const ADAPTER_PACKAGE = "packages/longhorn-poodle-svelte";
const LONGHORN = "@inflatable-cookie/longhorn";
const LONGHORN_PACKAGE = "packages/longhorn";
const CORE = "@inflatable-cookie/poodle-core";
const SVELTE = "@inflatable-cookie/poodle-svelte";

/** Root-manifest devDependency pins, filled once. Module-level so the helpers
 * see it without relying on a try-block closure. */
const rootPins = new Map<string, string>();

const adapterManifest = JSON.parse(
  await readFile(join(repoRoot, ADAPTER_PACKAGE, "package.json"), "utf8"),
) as {
  peerDependencies: Record<string, string>;
  exports: Record<string, unknown>;
};

for (const [name, pin] of Object.entries(
  (JSON.parse(await readFile(join(repoRoot, "package.json"), "utf8")) as {
    devDependencies?: Record<string, string>;
  }).devDependencies ?? {},
)) {
  rootPins.set(name, pin);
}

const declaredSvelteRange = adapterManifest.peerDependencies.svelte;
if (!declaredSvelteRange) {
  throw new Error(`${ADAPTER} declares no svelte peer; this proof has nothing to typecheck against`);
}
const { floor, ceilingMajor } = svelteRangeShape(declaredSvelteRange);

/** The two Svelte releases this proof typechecks against. The range entry is
 * installed verbatim, so npm picks the newest release the declaration admits;
 * the floor entry demotes that to an exact pin, because a bundle of stages
 * that all resolve upward never runs the lower bound a consumer may be on. */
const svelteMatrix = [
  { label: "newest-admitted", spec: declaredSvelteRange },
  { label: "floor", spec: floor },
] as const;

const typechecked: {
  label: string;
  spec: string;
  resolved: string;
}[] = [];

for (const { label, spec } of svelteMatrix) {
  typechecked.push({
    label,
    spec,
    resolved: await typecheckStage(spec),
  });
}

console.log(
  JSON.stringify(
    {
      schema: "longhorn.pack-typecheck.v1",
      outcome: "pass",
      poodleVersion: poodleRelease().version,
      svelteRange: declaredSvelteRange,
      typecheckedSvelte: typechecked,
      packed: [LONGHORN, ADAPTER],
      poodleIntegrityVerified: poodleRelease().packages.map((pkg) => pkg.name),
      typecheckers: ["tsc", "svelte-check"],
    },
    null,
    2,
  ),
);

/**
 * Pack both Longhorn packages, install them with registry Poodle into a bare
 * stage, and typecheck the whole adapter surface. Returns the Svelte version
 * the stage actually resolved, which must be one the declared range admits.
 */
async function typecheckStage(svelteSpec: string): Promise<string> {
  const stage = await mkdtemp(join(tmpdir(), "longhorn-pack-typecheck-"));
  /** Tarballs in the stage and each packed package's manifest version, filled
   * after packing. Function-local so one pass cannot see the other's stage. */
  const packs = new Set<string>();
  const versions = new Map<string, string>();

  try {
    const release = poodleRelease();

    // Pack the release's own tarballs into the stage: it installs the published
    // artifact, not the working tree.
    for (const [name, directory] of [
      [LONGHORN, LONGHORN_PACKAGE],
      [ADAPTER, ADAPTER_PACKAGE],
    ] as const) {
      versions.set(
        name,
        (JSON.parse(
          await readFile(join(repoRoot, directory, "package.json"), "utf8"),
        ) as { version: string }).version,
      );
      run(
        ["bun", "pm", "pack", "--ignore-scripts", "--destination", stage],
        join(repoRoot, directory),
      );
    }
    packs.clear();
    for (const entry of await readdir(stage)) {
      if (entry.endsWith(".tgz")) packs.add(entry);
    }
    if (packs.size !== 2) {
      throw new Error(`expected exactly two packed tarballs in ${stage}, got ${[...packs].join(", ")}`);
    }

    // Install the peer ranges exactly as the published manifest declares them.
    // npm is the only source of the Poodle entries here, which is the claim.
    // Svelte is the one declaration the caller overrides, to run the matrix.
    const manifest = {
      name: "longhorn-pack-typecheck-proof",
      private: true,
      type: "module",
      dependencies: {
        [LONGHORN]: `file:./${packPath(stage, LONGHORN, packs, versions)}`,
        [ADAPTER]: `file:./${packPath(stage, ADAPTER, packs, versions)}`,
        [CORE]: adapterManifest.peerDependencies[CORE]!,
        [SVELTE]: adapterManifest.peerDependencies[SVELTE]!,
        svelte: svelteSpec,
      },
      devDependencies: {
        "svelte-check": rootTool("svelte-check"),
        typescript: rootTool("typescript"),
      },
      // The adapter peer-depends on `longhorn`, which is not on npm yet: both
      // packages publish together in one release. Pin the peer to the packed
      // tarball the same way the command-system consumer proofs do, instead of
      // letting the peer resolution fetch a registry name that does not exist.
      overrides: {
        [LONGHORN]: `file:./${packPath(stage, LONGHORN, packs, versions)}`,
        [ADAPTER]: `file:./${packPath(stage, ADAPTER, packs, versions)}`,
      },
    };
    await writeFile(join(stage, "package.json"), `${JSON.stringify(manifest, null, 2)}\n`);
    run(["bun", "install", "--ignore-scripts"], stage);

    // The installed Longhorn packages must be the packed ones, inside the stage.
    // Without this, an ambiguous tarball pin could silently install something
    // other than the release identity the manifests declare.
    for (const name of [LONGHORN, ADAPTER]) {
      const installed = join(stage, "node_modules", ...name.split("/"));
      const manifestInstalled = JSON.parse(
        await readFile(join(installed, "package.json"), "utf8"),
      ) as { name: string; version: string };
      if (
        manifestInstalled.name !== name ||
        manifestInstalled.version !== versions.get(name)
      ) {
        throw new Error(
          `${name} installed as ${manifestInstalled.name}@${manifestInstalled.version}, ` +
            `expected ${name}@${versions.get(name)}`,
        );
      }
      const target = await realpath(installed);
      if (relative(await realpath(stage), target).startsWith("..")) {
        throw new Error(`${name} resolved outside the stage: ${target}`);
      }
    }

    // What npm served must be the pinned release, resolved inside the stage --
    // not a sibling checkout reached through a link -- with the same integrity
    // bytes the root lockfile records for it.
    for (const pkg of release.packages) {
      const installed = join(stage, "node_modules", ...pkg.name.split("/"));
      const manifestInstalled = JSON.parse(
        await readFile(join(installed, "package.json"), "utf8"),
      ) as { version: string };
      if (manifestInstalled.version !== release.version) {
        throw new Error(
          `${pkg.name} resolved at ${manifestInstalled.version}, ` +
            `not the pinned ${release.version}; the peer range and the repository pin have drifted`,
        );
      }
      const target = await realpath(installed);
      // macOS resolves /var under /private/var, so compare against the stage's
      // own realpath rather than the path tmpdir() handed out.
      if (relative(await realpath(stage), target).startsWith("..")) {
        throw new Error(
          `${pkg.name} resolved outside the stage: ${target}. ` +
            "The typecheck must run against a registry install, not a machine-local source.",
        );
      }
      const locked = await lockIntegrity(join(stage, "bun.lock"), pkg.name);
      if (locked !== pkg.integrity) {
        throw new Error(
          `registry install of ${pkg.name} carries ${locked}, ` +
            `the pinned release records ${pkg.integrity}`,
        );
      }
    }

    // The Svelte runtime the stage resolved is the release this pass actually
    // typechecks. Reading it off the install rather than the request keeps the
    // envelope honest when npm serves something other than the newest, and the
    // admission check keeps a hand-edited range from proving a release the
    // published declaration rejects.
    const resolvedSvelte = JSON.parse(
      await readFile(join(stage, "node_modules", "svelte", "package.json"), "utf8"),
    ).version as string;
    if (!admitsSvelte(resolvedSvelte, floor, ceilingMajor)) {
      throw new Error(
        `stage resolved svelte ${resolvedSvelte}, which the declared range ` +
          `${declaredSvelteRange} does not admit`,
      );
    }
    if (svelteSpec === floor && resolvedSvelte !== floor) {
      throw new Error(
        `the floor pass asked for exactly ${floor} but resolved ${resolvedSvelte}`,
      );
    }

    // The packed adapter must carry its published surface. files = ["src", …]:
    // if src ever drops out of the tarball, this install is not what consumers
    // get and the typecheck below would pass against the wrong bytes.
    const packed = join(stage, "node_modules", ...ADAPTER.split("/"));
    await stat(join(packed, "src", "bindings", "index.ts"));

    // svelte-check refuses to look inside node_modules, so the .svelte half of
    // the surface is typechecked from an unpacked copy of the same tarball. The
    // unpacked tree's imports of svelte and Poodle still resolve upward into the
    // stage's node_modules, so the registry install stays the only source of
    // types for them. The tarball's own `*.svelte` ambient module would collide
    // with the installed copy's, so the unpacked one is excluded.
    await mkdir(join(stage, "pack"));
    run(
      ["tar", "-xzf", packPath(stage, ADAPTER, packs, versions), "-C", "pack"],
      stage,
    );
    await stat(join(stage, "pack", "package", "src", "bindings", "index.ts"));

    // One module per exports entry, so the typecheck covers the exports map and
    // the transitive graph behind every published subpath. `./package.json` is
    // an export no module imports; it is verified separately.
    const exports = Object.keys(adapterManifest.exports).filter((key) => key !== "./package.json");
    if (!adapterManifest.exports["./package.json"]) {
      throw new Error("packed manifest does not export ./package.json");
    }
    const imports = [
      ...exports.map((key) => `import "${ADAPTER}${key === "." ? "" : key}";`),
      `import "${CORE}";`,
      `import "${SVELTE}";`,
      `import "svelte";`,
    ];
    await writeFile(join(stage, "imports.ts"), `${imports.join("\n")}\n`);

    await writeFile(
      join(stage, "tsconfig.json"),
      `${JSON.stringify(
        {
          compilerOptions: {
            allowImportingTsExtensions: true,
            module: "ESNext",
            moduleResolution: "Bundler",
            noEmit: true,
            resolveJsonModule: true,
            skipLibCheck: true,
            strict: true,
            target: "ES2022",
            verbatimModuleSyntax: true,
          },
          include: [
            "imports.ts",
            "pack/package/src/**/*.ts",
            "pack/package/src/**/*.svelte",
          ],
          exclude: ["pack/package/src/svelte.d.ts"],
        },
        null,
        2,
      )}\n`,
    );

    // tsc covers the TypeScript surface; svelte-check adds the .svelte files.
    typecheck(stage, ["bun", "x", "tsc", "-p", "tsconfig.json"]);
    typecheck(stage, ["bun", "x", "svelte-check", "--tsconfig", "tsconfig.json"]);

    return resolvedSvelte;
  } finally {
    if (process.env.KEEP_PACK_TYPECHECK === "1") {
      console.error(`retained pack-typecheck stage (${svelteSpec}): ${stage}`);
    } else {
      await rm(stage, { recursive: true, force: true });
    }
  }
}

function run(command: readonly string[], cwd: string): void {
  const result = spawnSync(command[0]!, command.slice(1), {
    cwd,
    stdio: ["ignore", "ignore", "inherit"],
  });
  if (result.status !== 0) {
    throw new Error(`${command.join(" ")} failed in ${cwd}`);
  }
}

function typecheck(stage: string, command: readonly string[]): void {
  const result = spawnSync(command[0]!, command.slice(1), {
    cwd: stage,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (result.status !== 0) {
    process.stderr.write(result.stdout ?? "");
    process.stderr.write(result.stderr ?? "");
    throw new Error(`${command.join(" ")} failed against the packed adapter`);
  }
}

/** The exact tarball name bun packs for this package:
 * `@inflatable-cookie/longhorn` → `inflatable-cookie-longhorn-<version>.tgz`.
 * Prefix matching is not enough here: `inflatable-cookie-longhorn-` is also a
 * prefix of the adapter's tarball, so the version read from the packed
 * manifest makes the match exact instead of order-dependent. */
function packPath(
  stage: string,
  name: string,
  packs: ReadonlySet<string>,
  versions: ReadonlyMap<string, string>,
): string {
  const expected = `${name.replace(/^@/, "").replace("/", "-")}-${versions.get(name)}.tgz`;
  if (!packs.has(expected)) {
    throw new Error(`no packed tarball for ${name}: expected ${expected} in ${stage}`);
  }
  return expected;
}

/** Tool pins come from the root manifest; a missing one must fail the proof
 * rather than let `bun x` fetch an unpinned version at run time. */
function rootTool(name: string): string {
  const pin = rootPins.get(name);
  if (!pin) {
    throw new Error(`the root manifest no longer pins ${name}; the proof needs the pin`);
  }
  return pin;
}

/** bun.lock is JSONC; the registry entry shape is the same one
 * poodle-release.ts matches. Kept local because the root there is fixed and
 * here it is the staged lockfile. */
async function lockIntegrity(lockPath: string, name: string): Promise<string> {
  const lock = await readFile(lockPath, "utf8");
  const escaped = name.replace(/[/@-]/g, (character) => `\\${character}`);
  const entry = new RegExp(
    `"${escaped}":\\s*\\["${escaped}@([^"]+)"[\\s\\S]*?"(sha512-[^"]+)"\\s*\\]`,
  ).exec(lock);
  if (!entry) {
    throw new Error(`staged bun.lock records no registry entry for ${name}`);
  }
  return entry[2]!;
}

/**
 * The declared `svelte` peer range, narrowed to the one shape this repository
 * writes: `>=<x.y.z> <<N>`. A different shape is a stop rather than a silent
 * skip: the matrix below is derived from the floor and the ceiling, and a range
 * this parser only half understands would prove the wrong releases.
 */
function svelteRangeShape(range: string): { floor: string; ceilingMajor: number } {
  const match = /^>=(\d+\.\d+\.\d+) <(\d+)$/.exec(range);
  if (!match) {
    throw new Error(
      `svelte peer range ${range} is not the \`>=x.y.z <N\` shape this proof ` +
        "can derive its matrix from",
    );
  }
  return { floor: match[1]!, ceilingMajor: Number(match[2]!) };
}

/** Membership in `>=floor <ceilingMajor`, for release versions only. A
 * prerelease is not admitted: the range carries no prerelease tag, and npm
 * does not offer one for it either. */
function admitsSvelte(version: string, floor: string, ceilingMajor: number): boolean {
  if (version.includes("-")) return false;
  const parsed = version.split(".").map(Number);
  const low = floor.split(".").map(Number);
  if (parsed.length !== 3 || parsed.some(Number.isNaN)) return false;
  if (parsed[0]! >= ceilingMajor) return false;
  for (let index = 0; index < 3; index += 1) {
    if (parsed[index]! !== low[index]!) return parsed[index]! > low[index]!;
  }
  return true;
}
