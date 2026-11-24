import { expect, test } from "bun:test";
import { readdir, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";

// AGENTS.md: "Real Rust stages live on external build scratch". A proof that
// builds Rust allocates build scratch with `taskScratch` (or its staged
// workspace through `prepareStagedWorkspace`), and its staging root must come
// from the same external root. `os.tmpdir()` follows `TMPDIR`, which can name
// the boot volume, so any such script that touches it breaks that claim.
const scriptsRoot = resolve(import.meta.dir);

const rustStagingCall = /\bawait (?:taskScratch|prepareStagedWorkspace)\(/;

test("a script that stages Rust builds never takes its storage from os.tmpdir()", async () => {
  const entries = await readdir(scriptsRoot, { recursive: true });
  const candidates = entries.filter(
    (path) => path.endsWith(".ts") && !path.endsWith(".test.ts") && path !== "rust-command.ts",
  );

  const staging: Array<string> = [];
  for (const path of candidates) {
    const source = await readFile(join(scriptsRoot, path), "utf8");
    if (!rustStagingCall.test(source)) {
      continue;
    }
    staging.push(path);
    expect({ path, tmpdir: /\btmpdir\b/.test(source) }).toEqual({ path, tmpdir: false });
  }

  // The scan must find the proofs it guards, or it proves nothing.
  expect(staging.length).toBeGreaterThanOrEqual(12);
});
