import { readCargoInventory } from "./cargo-metadata.ts";
import { dirname, resolve } from "node:path";
import { runSync } from "./proof-support.ts";
import { rustCommand } from "./rust-command.ts";

const root = resolve(import.meta.dir, "..");
const metadata = readCargoInventory(
  runSync(rustCommand(["metadata", "--locked", "--no-deps", "--format-version", "1"]), root),
);
const members = metadata.packages.map((member) => dirname(member.manifest_path));
if (members.length === 0) {
  throw new Error("Cargo metadata did not return any workspace members");
}
const check = Bun.spawnSync(rustCommand(["machete", ...members]), {
  cwd: root,
  stdout: "inherit",
  stderr: "inherit",
});
process.exitCode = check.exitCode;
