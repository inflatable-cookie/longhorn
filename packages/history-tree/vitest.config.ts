import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  test: { projects: [
    { test: { name: "core", environment: "node", include: ["packages/history-tree/tests/**/*.test.ts"] } },
    { plugins: [svelte()], resolve: { conditions: ["browser"] }, ssr: { noExternal: ["@inflatable-cookie/longhorn-history-tree", "@inflatable-cookie/poodle-headless", "@inflatable-cookie/poodle-icons-lucide", "@inflatable-cookie/poodle-styles", "@inflatable-cookie/poodle-svelte", "@inflatable-cookie/poodle-svelte-tokens"] }, test: { name: "client", environment: "happy-dom", include: ["packages/history-tree/tests-svelte/**/*.test.ts"], exclude: ["packages/history-tree/tests-svelte/ssr.test.ts"], setupFiles: ["packages/history-tree/tests-svelte/setup.ts"] } },
    { plugins: [svelte()], test: { name: "ssr", environment: "node", include: ["packages/history-tree/tests-svelte/ssr.test.ts"] } },
  ] },
});
