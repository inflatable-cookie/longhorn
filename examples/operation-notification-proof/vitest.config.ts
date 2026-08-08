import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [svelte()],
  resolve: { conditions: ["browser"] },
  ssr: { noExternal: ["@inflatable-cookie/longhorn/operation", "@inflatable-cookie/longhorn/notifications", "@inflatable-cookie/poodle-core", "@inflatable-cookie/poodle-core/icons", "@inflatable-cookie/poodle-core/styles", "@inflatable-cookie/poodle-svelte", "@inflatable-cookie/poodle-core/tokens"] },
  test: {
    environment: "happy-dom",
    include: ["consumers/loophole/*.test.ts"],
    setupFiles: ["./setup.ts"],
  },
});
