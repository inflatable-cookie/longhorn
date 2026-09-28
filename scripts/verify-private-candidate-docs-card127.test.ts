import { describe, expect, test } from "bun:test";

import {
  assertCandidateFactsInProse,
  factsFromReceipt,
  type CandidateFacts,
} from "./verify-private-candidate-docs-card127.ts";

const facts: CandidateFacts = {
  version: "0.1.0",
  rustCrates: 36,
  typescriptPackages: 17,
  consumerGraphs: 7,
};

const changelogFields = ["version", "typescriptPackages", "rustCrates", "consumerGraphs"] as const;

describe("private-candidate CHANGELOG facts", () => {
  test("a reworded CHANGELOG that keeps the facts passes", () => {
    const changelog = `## [0.3.0]

- The private 0.1.0 candidate at Card 127 bound 17 TypeScript packages
  and 36 Rust crates across five Poodle artifacts and seven
  consumer graphs.
`;
    expect(() => assertCandidateFactsInProse("CHANGELOG.md", changelog, facts, changelogFields)).not.toThrow();
  });

  test("a wrong Rust crate count fails", () => {
    const changelog = `## [0.3.0]

- The private 0.1.0 candidate bound 17 TypeScript packages and 35 Rust crates
  and seven consumer graphs.
`;
    expect(() => assertCandidateFactsInProse("CHANGELOG.md", changelog, facts, changelogFields)).toThrow(
      /36 Rust crates/,
    );
  });

  test("facts are read from the receipt", () => {
    expect(
      factsFromReceipt({
        version: "0.1.0",
        artifacts: {
          longhornTypescript: { packages: Array.from({ length: 17 }) },
          longhornRust: { packages: Array.from({ length: 36 }) },
        },
        graphs: Array.from({ length: 7 }),
      }),
    ).toEqual(facts);
  });
});
