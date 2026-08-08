import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    conditions: ["browser"],
  },
  ssr: {
    noExternal: [
      "@inflatable-cookie/longhorn-history",
      "@inflatable-cookie/poodle-headless",
      "@inflatable-cookie/poodle-icons-lucide",
      "@inflatable-cookie/poodle-styles",
      "@inflatable-cookie/poodle-svelte",
      "@inflatable-cookie/poodle-svelte-tokens",
    ],
  },
  test: {
    environment: "happy-dom",
    setupFiles: ["./setup.ts"],
    include: ["consumers/**/*.test.ts"],
  },
});
