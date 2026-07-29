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
        test: {
          name: "client",
          environment: "happy-dom",
          include: ["packages/svelte/tests/**/*.test.ts"],
          exclude: ["packages/svelte/tests/ssr.test.ts"],
        },
      },
      {
        plugins: [svelte()],
        test: {
          name: "ssr",
          environment: "node",
          include: ["packages/svelte/tests/ssr.test.ts"],
        },
      },
    ],
  },
});
