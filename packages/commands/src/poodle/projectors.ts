import type {
  CommandActionItem,
  DiscoveryState,
} from "@inflatable-cookie/poodle-svelte";

import type {
  CommandControllerStatus,
} from "../controller.ts";
import type { CommandProjectionRecord } from "../projectors.ts";

export function toPoodleCommandItems(
  records: readonly CommandProjectionRecord[],
  categoryLabel: (categoryPath: readonly string[]) => string | null = (
    _categoryPath,
  ) => null,
): CommandActionItem[] {
  return records.map((record) => ({
    id: record.id,
    title: record.label,
    description: record.description,
    group: categoryLabel(record.categoryPath),
    shortcut: record.shortcuts[0]?.label ?? null,
    keywords: [...record.keywords],
    disabled: record.availability.state !== "available",
  }));
}

export function toPoodleDiscoveryState(
  status: CommandControllerStatus,
  query: string,
  itemCount: number,
): DiscoveryState {
  if (status.kind === "idle" || status.kind === "loading") return "loading";
  if (status.kind !== "ready") return "error";
  if (itemCount === 0) return query.length === 0 ? "empty" : "no-results";
  return "ready";
}
