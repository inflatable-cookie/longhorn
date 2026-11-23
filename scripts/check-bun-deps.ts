// Ensure TypeScript and Svelte selectors have their locked workspace dependencies.

import { access } from "node:fs/promises";
import { join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");
const markers = [
  "node_modules/typescript",
  "node_modules/svelte",
  "node_modules/@inflatable-cookie/poodle-core",
  "node_modules/.bin/oxlint",
  "node_modules/.bin/tsgolint",
  "node_modules/.bin/oxfmt",
  "node_modules/.bin/knip",
];

type CheckOptions = {
  root?: string;
  markerExists?: (path: string) => Promise<boolean>;
  install?: (root: string) => Promise<number>;
  log?: (message: string) => void;
  error?: (message: string) => void;
};

export async function checkBunDependencies(options: CheckOptions = {}): Promise<boolean> {
  const root = options.root ?? repoRoot;
  const markerExists =
    options.markerExists ??
    (async (path: string) => {
      try {
        await access(path);
        return true;
      } catch {
        return false;
      }
    });
  const log = options.log ?? console.log;
  const error = options.error ?? console.error;
  const missing = async () => {
    const absent: Array<string> = [];
    for (const marker of markers) {
      if (!(await markerExists(join(root, marker)))) {
        absent.push(marker);
      }
    }
    return absent;
  };

  let absent = await missing();
  if (absent.length > 0) {
    log("Bun dependencies missing; running bun install --frozen-lockfile.");
    const install = options.install ?? runLockedInstall;
    let exitCode: number;
    try {
      exitCode = await install(root);
    } catch (error_) {
      error(`Frozen Bun install failed: ${String(error_)}`);
      return false;
    }
    if (exitCode !== 0) {
      error(`Frozen Bun install failed with exit code ${exitCode}.`);
      return false;
    }
    absent = await missing();
    if (absent.length > 0) {
      error(
        `Bun install completed, but dependency markers are still missing: ${absent.join(", ")}.`,
      );
      return false;
    }
  }

  log(
    JSON.stringify(
      {
        schema: "longhorn.bun-deps.v1",
        outcome: "pass",
        markers,
      },
      null,
      2,
    ),
  );
  return true;
}

async function runLockedInstall(root: string): Promise<number> {
  return Bun.spawnSync(["bun", "install", "--frozen-lockfile"], {
    cwd: root,
    stdout: "inherit",
    stderr: "inherit",
  }).exitCode;
}

if (import.meta.main && !(await checkBunDependencies())) {
  process.exitCode = 1;
}
