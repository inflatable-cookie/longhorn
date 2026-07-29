import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    conditions: ["browser"],
  },
  ssr: {
    noExternal: [
      "@longhorn/poodle",
      "@longhorn/svelte",
      "@poodle/headless",
      "@poodle/icons-lucide",
      "@poodle/styles",
      "@poodle/svelte",
      "@poodle/svelte-tokens",
    ],
  },
  test: {
    environment: "happy-dom",
    include: ["src/**/*.test.ts"],
    setupFiles: ["./setup.ts"],
  },
});
