import { readFile, writeFile } from "node:fs/promises";
import { join, relative, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");
const entryPath = join(repoRoot, "packages/longhorn/src/agent-control/inject.ts");
const outputPath = join(
  repoRoot,
  "crates/longhorn-tauri-agent-control/src/agent_control_shim.js",
);
const write = process.argv.includes("--write");

const result = await Bun.build({
  entrypoints: [entryPath],
  format: "iife",
  target: "browser",
  minify: false,
});

if (!result.success) {
  throw new Error(`could not build agent-control shim:\n${result.logs.join("\n")}`);
}
if (result.outputs.length !== 1) {
  throw new Error(`expected one agent-control shim bundle; got ${result.outputs.length}`);
}

const generated = await result.outputs[0].text();
if (generated.length === 0) {
  throw new Error("agent-control shim bundle is empty");
}

if (write) {
  await writeFile(outputPath, generated);
  console.log(`Generated ${relative(repoRoot, outputPath)}`);
} else {
  const committed = await readFile(outputPath, "utf8");
  if (committed !== generated) {
    throw new Error(
      "Tauri agent-control shim is stale; run `effigy generate:agent-control-shim`",
    );
  }
  console.log(`Verified ${relative(repoRoot, outputPath)}`);
}
