import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

// Anchored on this file, not the cwd. `scripts/test-packages.sh` lists this
// package in VITEST_PACKAGES, so keep that list in step with this config.
const tests = fileURLToPath(new URL("./tests", import.meta.url));

const noExternal = [
  "@inflatable-cookie/longhorn",
  "@inflatable-cookie/longhorn-poodle-svelte",
  "@inflatable-cookie/poodle-core",
  "@inflatable-cookie/poodle-svelte",
  "@testing-library/svelte",
  "@testing-library/svelte-core",
];

export default defineConfig({
  test: {
    projects: [
      {
        plugins: [svelte()],
        resolve: {
          conditions: ["browser"],
        },
        ssr: {
          noExternal,
        },
        test: {
          name: "client",
          environment: "happy-dom",
          include: [`${tests}/**/*.test.ts`],
          exclude: [`${tests}/**/ssr.test.ts`],
          setupFiles: [`${tests}/setup.ts`],
        },
      },
      {
        plugins: [svelte()],
        test: {
          name: "ssr",
          environment: "node",
          include: [`${tests}/**/ssr.test.ts`],
        },
      },
    ],
  },
});
