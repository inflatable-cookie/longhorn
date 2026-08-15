import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

// Anchored on this file, not the cwd. Note `scripts/test-packages.sh` greps
// this config for the repo-relative tests path to decide the suite is
// vitest-owned; keep the literal: packages/longhorn-poodle-svelte/tests
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
