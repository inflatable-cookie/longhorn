import cloudflare from "@cloudflare/oxlint-config";
import { defineConfig } from "oxlint";

export default defineConfig({
  ...cloudflare,
  ignorePatterns: [
    ...(cloudflare.ignorePatterns ?? []),
    // The Rust generator and its byte-for-byte drift check own these files.
    "packages/longhorn/src/**/generated/**",
  ],
  globals: {
    ...cloudflare.globals,
    Bun: "readonly",
  },
  overrides: [
    ...(cloudflare.overrides ?? []),
    {
      // Command selectors and proof drivers use stdout/stderr as their interface.
      files: ["scripts/**/*.ts", "examples/agent-control-proof/*.ts"],
      rules: { "no-console": "off" },
    },
    {
      // Raw ports deliver untrusted input to the checked client boundary.
      files: ["packages/**/ports.ts", "packages/longhorn-tauri/src/transport/*.ts"],
      rules: { "quality/no-unknown-returns": "off" },
    },
    {
      files: ["**/*.svelte", "**/*.svelte.ts"],
      // The Svelte compiler provides these runes.
      globals: {
        $state: "readonly",
        $derived: "readonly",
        $effect: "readonly",
        $props: "readonly",
        $bindable: "readonly",
        $inspect: "readonly",
        $host: "readonly",
      },
    },
    {
      files: [
        "packages/longhorn/src/**/validation*.ts",
        "packages/longhorn/src/**/validation/*.ts",
      ],
      jsPlugins: ["./scripts/variant-keys-lint.ts"],
      rules: { "longhorn/generated-variant-keys": "error" },
    },
  ],
});
