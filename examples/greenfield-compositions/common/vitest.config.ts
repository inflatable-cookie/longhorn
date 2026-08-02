import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    conditions: ["browser"],
  },
  test: {
    environment: "happy-dom",
    setupFiles: ["common/setup.ts"],
    include: ["common/App.test.ts"],
    server: {
      deps: {
        inline: [
          "@poodle/headless",
          "@poodle/icons-lucide",
          "@poodle/styles",
          "@poodle/svelte",
          "@poodle/svelte-tokens",
        ],
      },
    },
  },
});
