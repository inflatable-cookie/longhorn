// The MSRV floor is declared once, in release-baselines/rust-toolchains.env,
// and everywhere else reads it from here. A floor bump is one edit plus the
// recorded `effigy release:floor` run, not fifteen scattered strings.

import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

const baseline = await readFile(
  resolve(import.meta.dir, "../release-baselines/rust-toolchains.env"),
  "utf8",
);
const declared = /^LONGHORN_GENERAL_MSRV=(\d+\.\d+\.\d+)$/m.exec(baseline)?.[1];
if (!declared) {
  throw new Error("LONGHORN_GENERAL_MSRV not declared in release-baselines/rust-toolchains.env");
}

/** The toolchain selector, e.g. "1.97.1" — for `cargo +<toolchain>`. */
export const MSRV_TOOLCHAIN = declared;
/** The exact Cargo `rust-version` for generated manifests. */
export const MSRV = declared;

/** Parse `[workspace.package].rust-version` from a Cargo.toml manifest. */
export function parseWorkspaceRustVersion(manifest: string): string | undefined {
  const parsed = Bun.TOML.parse(manifest) as {
    workspace?: { package?: { "rust-version"?: unknown } };
  };
  const value = parsed.workspace?.package?.["rust-version"];
  return typeof value === "string" ? value : undefined;
}

// Cross-check at import: the workspace's own rust-version must agree.
const workspace = await readFile(resolve(import.meta.dir, "../Cargo.toml"), "utf8");
const workspaceRustVersion = parseWorkspaceRustVersion(workspace);
if (workspaceRustVersion !== MSRV) {
  throw new Error(
    `Cargo.toml [workspace.package].rust-version (${String(workspaceRustVersion)}) disagrees with the ${MSRV} floor`,
  );
}
