import { expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");

type PlannedMember = { task: string };
type QaGroupPlan = {
  schema: string;
  scope_assessment: "declared_match" | "needs_planner" | string;
  members?: PlannedMember[];
  unmatched_scope_inputs?: string[];
  needs_planner_reasons?: { token: string; reason: string }[];
};

const runtimeInputs = ["external:root-node-modules", "external:bun-runtime"];

const taskTableRoutes = [
  {
    group: "longhorn-pack-typecheck",
    task: "proof:pack-typecheck",
    scope: ["input:effigy-task-table-proof-pack-typecheck", ...runtimeInputs],
  },
  {
    group: "longhorn-bridge-topology-conformance",
    task: "proof:bridge-topology-conformance",
    scope: ["input:effigy-task-table-proof-bridge-topology-conformance", ...runtimeInputs],
  },
  {
    group: "longhorn-poodle-preview",
    task: "proof:poodle-preview",
    scope: ["input:effigy-task-table-proof-poodle-preview", ...runtimeInputs],
  },
  {
    group: "longhorn-documented-commands",
    task: "proof:documented-commands",
    scope: ["input:effigy-task-table-proof-documented-commands"],
  },
  {
    group: "longhorn-getting-started-docs",
    task: "proof:guides-card126",
    scope: ["input:effigy-task-table-proof-guides-card126"],
  },
] as const;

test("classified task-table entries route through a group that runs that selector", () => {
  for (const route of taskTableRoutes) {
    const plan = planGroup(route.group, route.scope);
    expect(plan.scope_assessment).toBe("declared_match");
    expect(plan.members?.some((member) => member.task === route.task)).toBe(true);
  }
});

test("mapped proof inputs route to their owning standalone selector", () => {
  const examples = planGroup("longhorn-documented-commands", [
    "path:examples/bridge-topology-proof/README.md",
  ]);
  expect(examples.scope_assessment).toBe("declared_match");
  expect(examples.members?.some((member) => member.task === "proof:documented-commands")).toBe(true);

  const packageClosure = planGroup("longhorn-pack-typecheck", [
    "bun-package:@inflatable-cookie/longhorn",
    "path:packages/longhorn/package.json",
    "bun-package:@inflatable-cookie/longhorn-poodle-svelte",
    "path:packages/longhorn-poodle-svelte/package.json",
    "path:package.json",
    "path:bun.lock",
    ...runtimeInputs,
  ]);
  expect(packageClosure.scope_assessment).toBe("declared_match");
  expect(packageClosure.members?.some((member) => member.task === "proof:pack-typecheck")).toBe(true);

  const bridgeManifest = planGroup("longhorn-bridge-topology-conformance", [
    "bun-package:@inflatable-cookie/longhorn",
    "path:packages/longhorn/src/bridge/index.ts",
    "path:crates/longhorn-bridge/Cargo.toml",
    ...runtimeInputs,
  ]);
  expect(bridgeManifest.scope_assessment).toBe("declared_match");
  expect(bridgeManifest.members?.some((member) => member.task === "proof:bridge-topology-conformance")).toBe(true);
});

test("global Effigy changes remain planner-owned, even beside a task-table token", () => {
  const globalChange = "path:effigy.toml";
  for (const scope of [
    [globalChange],
    [globalChange, "input:effigy-task-table-proof-pack-typecheck", ...runtimeInputs],
  ]) {
    const plan = planGroup("longhorn-pack-typecheck", scope);
    expect(plan.scope_assessment).toBe("needs_planner");
    expect(plan.unmatched_scope_inputs).toContain(globalChange);
    expect(plan.needs_planner_reasons?.some((entry) => entry.token === globalChange)).toBe(true);
  }
});

test("unmapped proof closures and task-table entries remain planner-owned", () => {
  const incompleteClosure = "path:scripts/verify-history-system-artifacts.ts";
  const proofPlan = planGroup("longhorn-pack-typecheck", [incompleteClosure]);
  expect(proofPlan.scope_assessment).toBe("needs_planner");
  expect(proofPlan.unmatched_scope_inputs).toContain(incompleteClosure);

  const unknownTask = "input:effigy-task-table-check-ts";
  const taskPlan = planGroup("longhorn-pack-typecheck", [unknownTask]);
  expect(taskPlan.scope_assessment).toBe("needs_planner");
  expect(taskPlan.unmatched_scope_inputs).toContain(unknownTask);
});

function planGroup(group: string, scope: readonly string[]): QaGroupPlan {
  const args = ["--json", "tasks", "qa-group", "run", group];
  for (const token of scope) args.push("--scope", token);
  args.push("--plan");

  const process = spawnSync("effigy", args, {
    cwd: repoRoot,
    encoding: "utf8",
    timeout: 20_000,
  });
  if (process.error) throw process.error;
  if (process.stdout.trim() === "") {
    throw new Error(`Effigy returned no plan for ${group}: ${process.stderr}`);
  }

  const envelope = JSON.parse(process.stdout) as {
    result?: QaGroupPlan;
    error?: { details?: QaGroupPlan };
  };
  const plan = envelope.result ?? envelope.error?.details;
  if (!plan || plan.schema !== "effigy.qa-group-plan.v1") {
    throw new Error(`Effigy returned no QA-group plan for ${group}: ${process.stdout}`);
  }
  return plan;
}
