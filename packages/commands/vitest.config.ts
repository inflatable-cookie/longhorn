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
            "@longhorn/commands",
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
          include: ["packages/commands/tests-svelte/**/*.test.ts"],
          exclude: ["packages/commands/tests-svelte/ssr.test.ts"],
          setupFiles: ["packages/commands/tests-svelte/setup.ts"],
        },
      },
      {
        plugins: [svelte()],
        test: {
          name: "ssr",
          environment: "node",
          include: ["packages/commands/tests-svelte/ssr.test.ts"],
        },
      },
    ],
  },
});
