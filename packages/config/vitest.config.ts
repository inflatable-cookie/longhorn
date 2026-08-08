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
            "@inflatable-cookie/longhorn-config",
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
          include: ["packages/config/tests-svelte/**/*.test.ts"],
          exclude: ["packages/config/tests-svelte/ssr.test.ts"],
          setupFiles: ["packages/config/tests-svelte/setup.ts"],
        },
      },
      {
        plugins: [svelte()],
        test: {
          name: "ssr",
          environment: "node",
          include: ["packages/config/tests-svelte/ssr.test.ts"],
        },
      },
    ],
  },
});
