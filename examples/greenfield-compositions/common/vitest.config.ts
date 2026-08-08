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
          "@inflatable-cookie/poodle-core",
          "@inflatable-cookie/poodle-core/icons",
          "@inflatable-cookie/poodle-core/styles",
          "@inflatable-cookie/poodle-svelte",
          "@inflatable-cookie/poodle-core/tokens",
        ],
      },
    },
  },
});
