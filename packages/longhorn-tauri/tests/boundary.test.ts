import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const packageRoot = new URL("../", import.meta.url);
const metadata = JSON.parse(
  readFileSync(new URL("package.json", packageRoot), "utf8"),
) as Record<string, unknown>;

function sourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) return sourceFiles(path);
    return entry.endsWith(".ts") ? [path] : [];
  });
}

describe("@inflatable-cookie/longhorn-tauri package boundary", () => {
  test("pins exactly the host runtime and the framework as peers", () => {
    expect(metadata.dependencies).toBeUndefined();
    expect(metadata.peerDependencies).toEqual({
      "@inflatable-cookie/longhorn": "0.1.0",
      "@tauri-apps/api": "^2.10.1",
    });
  });

  test("every export target resolves", () => {
    const exports = metadata.exports as Record<
      string,
      string | Record<string, string>
    >;
    for (const target of Object.values(exports)) {
      const paths = typeof target === "string" ? [target] : Object.values(target);
      for (const path of paths) {
        expect(existsSync(new URL(path, packageRoot))).toBeTrue();
      }
    }
  });

  // The host edge may consume the framework, but only across its published
  // surface — a relative path into the sibling package would defeat the split.
  test("reaches the framework only through its public specifiers", () => {
    const offenders = sourceFiles(new URL("src", packageRoot).pathname).filter(
      (file) => {
        const text = readFileSync(file, "utf8");
        return (
          /from "\.\.\/\.\.\/longhorn\//.test(text) ||
          text.includes("@inflatable-cookie/poodle-") ||
          text.includes('from "svelte"')
        );
      },
    );
    expect(offenders).toEqual([]);
  });
});
