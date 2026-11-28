import { afterEach, describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, relative, resolve } from "node:path";

import { exampleRustFiles } from "./fmt-rust.ts";

const repoRoot = resolve(import.meta.dir, "..");
const created: Array<string> = [];

afterEach(async () => {
  for (const dir of created.splice(0)) {
    await rm(dir, { recursive: true, force: true });
  }
});

describe("example Rust sources", () => {
  test("a new example group is picked up without a hand-kept list", async () => {
    const root = await mkdtemp(join(tmpdir(), "fmt-rust-examples-"));
    created.push(root);
    await mkdir(join(root, "examples", "brand-new-group", "src"), { recursive: true });
    await writeFile(join(root, "examples", "brand-new-group", "src", "lib.rs"), "fn a() {}\n");
    await mkdir(join(root, "examples", "brand-new-group", "target"), { recursive: true });
    await writeFile(join(root, "examples", "brand-new-group", "target", "skip.rs"), "");

    const files = await exampleRustFiles(root);
    expect(files.map((file) => relative(root, file))).toEqual([
      join("examples", "brand-new-group", "src", "lib.rs"),
    ]);
  });

  test("the native-content local consumer is formatted with the other examples", async () => {
    const files = (await exampleRustFiles(repoRoot)).map((file) => relative(repoRoot, file));
    expect(files).toContain(
      join("examples", "native-content-local-consumer", "consumer", "src", "main.rs"),
    );
  });
});
