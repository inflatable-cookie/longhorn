import { parse } from "svelte/compiler";
import type { KnipConfig } from "knip";

export default {
  compilers: {
    svelte(source, filename) {
      const { module, instance } = parse(source, { filename, modern: true });
      // Retain type imports, which the Svelte build erases from its JS.
      return [module, instance]
        .map((script) => {
          if (!script) {
            return "";
          }
          const { content } = script;
          if (
            !("start" in content) ||
            typeof content.start !== "number" ||
            !("end" in content) ||
            typeof content.end !== "number"
          ) {
            throw new Error(`Svelte script offsets missing in ${filename}`);
          }
          return source.slice(content.start, content.end);
        })
        .join("\n");
    },
  },
  ignoreBinaries: ["effigy"],
  // The staged settings proof copies both modules beside its tests.
  ignoreUnresolved: ["./settings-proof.ts", "./config-proof.ts"],
  workspaces: {
    ".": {
      entry: [
        "svelte.config.mjs",
        "scripts/{agent-control-shim,check-*,fmt-rust,generate-*,install-agent-control-skill,proof-artifacts,release-bump,sync-prototype-locks,test-count,variant-keys-lint,verify-*}.ts",
        "scripts/**/*.test.ts",
        "examples/agent-control-proof/*.ts",
        "examples/bridge-topology-proof/*.ts",
      ],
      project: [
        "scripts/**/*.ts",
        "examples/{agent-control-proof,bridge-topology-proof}/*.ts",
        "*.{ts,mjs,svelte}",
      ],
      // These tools are invoked by Effigy. The Poodle pins are read by the
      // staged proofs, which install the packages in a separate workspace.
      ignoreDependencies: [
        "@inflatable-cookie/poodle-core",
        "@inflatable-cookie/poodle-svelte",
        "@testing-library/jest-dom",
        "oxfmt",
        "svelte-check",
      ],
    },
    "packages/longhorn": {
      entry: ["tests/**/*.test.ts", "src/agent-control/inject.ts"],
      project: ["src/**/*.{ts,svelte}", "tests/**/*.{ts,svelte}"],
      ignoreIssues: {
        // Generated domains share an output shape even when a map is empty.
        "src/bridge/generated/variant-fields.ts": ["files"],
        // Retain the types behind the agent-control shim's public results.
        "src/agent-control/shim.ts": ["types"],
      },
    },
    "packages/longhorn-tauri": {
      entry: ["tests/**/*.test.ts"],
      project: ["src/**/*.{ts,svelte}", "tests/**/*.{ts,svelte}"],
    },
    "packages/longhorn-poodle-svelte": {
      entry: ["tests/**/*.test.ts"],
      project: ["src/**/*.{ts,svelte}", "tests/**/*.{ts,svelte}"],
      ignoreIssues: {
        // Poodle is required for Poodle subpaths only, so its peers stay optional.
        "package.json": ["optionalPeerDependencies"],
      },
    },
  },
} satisfies KnipConfig;
