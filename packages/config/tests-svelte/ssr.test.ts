import { expect, test } from "vitest";

test(
  "Poodle config page subpath imports without browser globals",
  async () => {
    expect("window" in globalThis).toBe(false);
    expect("document" in globalThis).toBe(false);
    const pages = await import("../src/poodle.ts");
    expect(pages.StorageSettingsPage).toBeTruthy();
    expect(pages.BackupSettingsPage).toBeTruthy();
  },
  15_000,
);
