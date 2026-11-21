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

// Cross-check at import: the workspace's own rust-version must agree.
const workspace = await readFile(resolve(import.meta.dir, "../Cargo.toml"), "utf8");
if (!workspace.includes(`rust-version = "${MSRV}"`)) {
  throw new Error(`Cargo.toml rust-version disagrees with the ${MSRV} floor`);
}
