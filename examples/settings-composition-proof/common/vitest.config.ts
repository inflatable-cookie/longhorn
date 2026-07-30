import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    conditions: ["browser"],
  },
  ssr: {
    noExternal: [
      "@longhorn/config",
      "@longhorn/settings",
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
    include: ["src/**/*.test.ts"],
  },
});
