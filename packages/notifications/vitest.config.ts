import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    projects: [
      {
        plugins: [svelte()],
        resolve: { conditions: ["browser"] },
        ssr: { noExternal: ["@inflatable-cookie/longhorn-notifications", "@inflatable-cookie/poodle-headless", "@inflatable-cookie/poodle-icons-lucide", "@inflatable-cookie/poodle-styles", "@inflatable-cookie/poodle-svelte", "@inflatable-cookie/poodle-svelte-tokens"] },
        test: { name: "client", environment: "happy-dom", include: ["packages/notifications/tests-svelte/**/*.test.ts"], exclude: ["packages/notifications/tests-svelte/ssr.test.ts"], setupFiles: ["packages/notifications/tests-svelte/setup.ts"] },
      },
      {
        plugins: [svelte()],
        test: { name: "ssr", environment: "node", include: ["packages/notifications/tests-svelte/ssr.test.ts"] },
      },
    ],
  },
});
