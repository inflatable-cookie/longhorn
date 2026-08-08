import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

const tests = "packages/longhorn-poodle-svelte/tests";

const noExternal = [
  "@inflatable-cookie/longhorn",
  "@inflatable-cookie/longhorn-poodle-svelte",
  "@inflatable-cookie/poodle-core",
  "@inflatable-cookie/poodle-svelte",
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
