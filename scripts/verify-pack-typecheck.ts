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

const stage = await mkdtemp(join(tmpdir(), "longhorn-pack-typecheck-"));
/** Tarballs in the stage, filled after packing. A module-level binding so the
 * helper functions see it without relying on a try-block closure. */
const packs: string[] = [];

try {
  const release = poodleRelease();
  const root = JSON.parse(await readFile(join(repoRoot, "package.json"), "utf8")) as {
    devDependencies?: Record<string, string>;
  };
  const adapterManifest = JSON.parse(
    await readFile(join(repoRoot, ADAPTER_PACKAGE, "package.json"), "utf8"),
  ) as {
    peerDependencies: Record<string, string>;
    exports: Record<string, unknown>;
  };

  // Pack the release's own tarballs into the stage: it installs the published
  // artifact, not the working tree.
  for (const name of [LONGHORN_PACKAGE, ADAPTER_PACKAGE]) {
    run(["bun", "pm", "pack", "--ignore-scripts", "--destination", stage],
      join(repoRoot, name));
  }
  packs.push(
    ...(await readdir(stage)).filter((entry) => entry.endsWith(".tgz")),
  );

  // Install the peer ranges exactly as the published manifest declares them.
  // npm is the only source of the Poodle entries here, which is the claim.
  const manifest = {
    name: "longhorn-pack-typecheck-proof",
    private: true,
    type: "module",
    dependencies: {
      [LONGHORN]: `file:./${packPath(LONGHORN)}`,
      [ADAPTER]: `file:./${packPath(ADAPTER)}`,
      [CORE]: adapterManifest.peerDependencies[CORE]!,
      [SVELTE]: adapterManifest.peerDependencies[SVELTE]!,
      svelte: adapterManifest.peerDependencies.svelte!,
    },
    devDependencies: {
      "svelte-check": root.devDependencies?.["svelte-check"]!,
      typescript: root.devDependencies?.typescript!,
    },
    // The adapter peer-depends on `longhorn`, which is not on npm yet: both
    // packages publish together in one release. Pin the peer to the packed
    // tarball the same way the command-system consumer proofs do, instead of
    // letting the peer resolution fetch a registry name that does not exist.
    overrides: {
      [LONGHORN]: `file:./${packPath(LONGHORN)}`,
      [ADAPTER]: `file:./${packPath(ADAPTER)}`,
    },
  };
  await writeFile(join(stage, "package.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  run(["bun", "install", "--ignore-scripts"], stage);

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
    ["tar", "-xzf", packPath(ADAPTER), "-C", "pack"],
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
  typecheck(["bun", "x", "tsc", "-p", "tsconfig.json"]);
  typecheck(["bun", "x", "svelte-check", "--tsconfig", "tsconfig.json"]);

  console.log(
    JSON.stringify(
      {
        schema: "longhorn.pack-typecheck.v1",
        outcome: "pass",
        poodleVersion: release.version,
        packed: [LONGHORN, ADAPTER],
        poodleIntegrityVerified: release.packages.map((pkg) => pkg.name),
        exportsTypechecked: exports.length,
        typecheckers: ["tsc", "svelte-check"],
      },
      null,
      2,
    ),
  );
} finally {
  if (process.env.KEEP_PACK_TYPECHECK === "1") {
    console.error(`retained pack-typecheck stage: ${stage}`);
  } else {
    await rm(stage, { recursive: true, force: true });
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

function typecheck(command: readonly string[]): void {
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

/** Scoped names pack with `@` and `/` folded into `-`:
 * `@inflatable-cookie/longhorn` → `inflatable-cookie-longhorn-<version>.tgz`. */
function packPath(name: string): string {
  const prefix = `${name.replace(/^@/, "").replace("/", "-")}-`;
  const match = packs.find((entry) => entry.startsWith(prefix));
  if (!match) {
    throw new Error(`no packed tarball for ${name} in ${stage}`);
  }
  return match;
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
