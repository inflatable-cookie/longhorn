import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    projects: [
      {
        plugins: [svelte()],
        resolve: {
          conditions: ["browser"],
        },
        ssr: {
          noExternal: [
            "@longhorn/history",
            "@poodle/headless",
            "@poodle/icons-lucide",
            "@poodle/styles",
            "@poodle/svelte",
            "@poodle/svelte-tokens",
          ],
        },
        test: {
          name: "client",
          environment: "happy-dom",
          include: ["packages/history/tests-svelte/**/*.test.ts"],
          exclude: ["packages/history/tests-svelte/ssr.test.ts"],
          setupFiles: ["packages/history/tests-svelte/setup.ts"],
        },
      },
      {
        plugins: [svelte()],
        test: {
          name: "ssr",
          environment: "node",
          include: ["packages/history/tests-svelte/ssr.test.ts"],
        },
      },
    ],
  },
});
