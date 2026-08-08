import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    conditions: ["browser"],
  },
  ssr: {
    noExternal: [
      "@inflatable-cookie/longhorn-commands",
      "@inflatable-cookie/longhorn-settings",
      "@poodle/headless",
      "@poodle/icons-lucide",
      "@poodle/styles",
      "@poodle/svelte",
      "@poodle/svelte-tokens",
    ],
  },
  test: {
    environment: "happy-dom",
    setupFiles: ["./setup.ts"],
    include: ["consumers/**/*.test.ts"],
  },
});
