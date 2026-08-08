import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const longhornRoot = resolve(import.meta.dir, "..");
const loopholeRoot = resolve(
  process.env.LOOPHOLE_REPO ?? resolve(longhornRoot, "../loophole"),
);
const fixture = JSON.parse(
  readFileSync(
    resolve(
      longhornRoot,
      "fixtures/migration/loophole-card111/linear-history-cutover-v1.json",
    ),
    "utf8",
  ),
) as {
  schema: string;
  outcome: string;
  payloads: { variant_count: number; codec_family: string };
  structure: { branch_package_present: boolean };
  persistence: { corrupt_canonical_fallback: boolean };
  renderer: { eight_entry_heuristic_active: boolean };
};

assertEqual(fixture.schema, "longhorn.loophole-linear-history-cutover.v1", "fixture schema");
assertEqual(fixture.payloads.variant_count, 83, "frozen payload count");
assertEqual(fixture.structure.branch_package_present, false, "branch package posture");
assertEqual(fixture.persistence.corrupt_canonical_fallback, false, "canonical fallback posture");
assertEqual(fixture.renderer.eight_entry_heuristic_active, false, "renderer heuristic posture");

const historySource = source("pulse/crates/pulse-history/src/lib.rs");
const enumBody = between(
  historySource,
  "pub enum PulseHistoryMutation",
  "impl PulseHistoryMutation",
);
const variants = [...enumBody.matchAll(/^    ([A-Z][A-Za-z0-9_]*)\s*(?:\{|,)/gm)].map(
  (match) => match[1],
);
assertEqual(variants.length, fixture.payloads.variant_count, "live payload count");
assertEqual(new Set(variants).size, variants.length, "unique payload count");

const inverseBody = between(historySource, "pub fn inverse", "pub fn coalesce_with");
const applySource = source("pulse/src/history_apply.rs");
for (const variant of variants) {
  assert(inverseBody.includes(`Self::${variant}`), `${variant} lost inverse coverage`);
  assert(
    applySource.includes(`PulseHistoryMutation::${variant}`),
    `${variant} lost product apply coverage`,
  );
}

for (const token of [
  "impl HistoryPolicy<PulseHistoryMutation> for PulseHistoryPolicy",
  fixture.payloads.codec_family,
  "LinearHistory<PulseHistoryMutation>",
  "from_persisted_for_project",
  "Longhorn history envelope disagrees with rollback projection",
  "HistoryTimedGroupRequest::new",
  "GROUPING_TIME_WINDOW",
]) {
  assert(historySource.includes(token), `Pulse history adapter lost ${JSON.stringify(token)}`);
}

const navigation = source("pulse/src/dispatch/history.rs");
for (const token of [
  "execute_navigation(plan, &mut transaction)",
  "PulseRuntimeHistoryTransaction",
  "*self.rt = self.backup.clone()",
  "journal_append_undo",
  "journal_append_redo",
]) {
  assert(navigation.includes(token), `Pulse navigation lost ${JSON.stringify(token)}`);
}

const persistence = source("pulse/crates/pulse-persistence/src/lib.rs");
assert(persistence.includes("Err(error) if canonical"), "canonical corruption can disappear silently");
const project = source("pulse/src/dispatch/project.rs");
assert(
  project.indexOf("from_persisted_for_project") < project.indexOf("rt.project_id ="),
  "project state mutates before history validation",
);

const host = source("aura/src-tauri/src/services/history_host.rs");
for (const token of [
  "impl HistoryHostAuthority for AuraHistoryHostAuthority",
  "HistoryPageSnapshot::from_page",
  "HistoryNavigationReceiptProjection::from_receipt",
  "HistoryNavigationRejectionCode::StaleRevision",
]) {
  assert(host.includes(token), `Aura history host lost ${JSON.stringify(token)}`);
}
const panel = source("aura/src/renderer/workspace/HistoryPanel.svelte");
assert(panel.includes("@inflatable-cookie/longhorn-history/poodle"), "shared Poodle history panel is not active");
assert(panel.includes("historySession.refresh()"), "external Pulse mutations do not refresh metadata");
assert(!existsSync(resolve(loopholeRoot, "aura/src/renderer/workspace/history-entries.ts")), "eight-entry renderer heuristic remains");

const manifests = [
  source("pulse/Cargo.toml"),
  source("aura/src-tauri/Cargo.toml"),
  source("aura/package.json"),
].join("\n");
assert(!manifests.includes("history-tree"), "fork-tree dependency entered Loophole");

console.log(
  JSON.stringify(
    {
      schema: "longhorn.loophole-linear-history-verification.v1",
      outcome: fixture.outcome,
      payloadVariants: variants.length,
      inverseCoverage: variants.length,
      applyCoverage: variants.length,
      canonicalRecovery: "strict",
      renderer: "authoritative-paged-poodle",
      branchPackagePresent: false,
    },
    null,
    2,
  ),
);

function source(path: string): string {
  return readFileSync(resolve(loopholeRoot, path), "utf8");
}

function between(value: string, start: string, end: string): string {
  const startAt = value.indexOf(start);
  const endAt = value.indexOf(end, startAt + start.length);
  assert(startAt >= 0 && endAt > startAt, `could not isolate ${start}..${end}`);
  return value.slice(startAt, endAt);
}

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

function assertEqual(actual: unknown, expected: unknown, label: string): void {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}
