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
            "@longhorn/native-content",
            "@longhorn/native-content-svelte",
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
          include: ["packages/native-content-svelte/tests/**/*.test.ts"],
          exclude: ["packages/native-content-svelte/tests/ssr.test.ts"],
        },
      },
      {
        plugins: [svelte()],
        test: {
          name: "ssr",
          environment: "node",
          include: ["packages/native-content-svelte/tests/ssr.test.ts"],
        },
      },
    ],
  },
});
