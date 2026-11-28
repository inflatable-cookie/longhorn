import cloudflare from "@cloudflare/oxlint-config";
import { defineConfig } from "oxlint";

export default defineConfig({
  ...cloudflare,
  rules: {
    ...cloudflare.rules,
    // IPC decoders inspect untrusted records field by field. An unknown
    // member requires narrowing, which the type-aware unsafe-access rules enforce.
    "quality/no-unsafe-dictionary-type": "off",
  },
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
      // Command selectors, proof drivers and the shipped agent-control finder use
      // stdout/stderr as their interface.
      files: [
        "scripts/**/*.ts",
        "examples/agent-control-proof/*.ts",
        "skills/agent-control/scripts/*.ts",
      ],
      rules: { "no-console": "off" },
    },
    {
      // Raw ports deliver untrusted input to the checked client boundary.
      files: [
        "packages/**/ports.ts",
        "packages/longhorn/src/**/serialized.ts",
        "packages/longhorn/src/core/{connection,transport}.ts",
        "packages/longhorn/src/bridge/{direct,loopback,operation,router,session,supervision,stream/source}.ts",
        "packages/longhorn/src/native-content/client.ts",
        "packages/longhorn/src/agent-control/selection.ts",
        "packages/longhorn-tauri/src/{bridge,bridge-events,transport/*}.ts",
      ],
      rules: { "quality/no-unknown-returns": "off" },
    },
    {
      // These test doubles carry raw and deliberately malformed wire values.
      // Real consumers still validate the values exposed by those doubles.
      files: ["packages/*/tests/**/*.ts", "scripts/**/*.test.ts"],
      rules: {
        "quality/no-chained-type-assertions": "off",
        "quality/no-unknown-returns": "off",
        "typescript/no-unsafe-type-assertion": "off",
      },
    },
    {
      // Bun's resolves/rejects matchers run asynchronously but are typed void.
      files: [
        "packages/longhorn/tests/**/*.test.ts",
        "packages/longhorn-tauri/tests/**/*.test.ts",
        "scripts/**/*.test.ts",
      ],
      rules: { "typescript/await-thenable": "off" },
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
