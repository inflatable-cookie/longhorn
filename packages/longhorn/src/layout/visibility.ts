import type {
  RegionDefinition,
  RegionState,
  RegionVisibility,
  RegionVisibilityState,
} from "./generated/protocol.ts";

export function projectOrdinaryRegionVisibility(
  definition: RegionDefinition,
  state: RegionState,
): RegionVisibility {
  if (definition.id !== state.region_id) {
    throw new RangeError(
      `region definition ${definition.id} does not match state ${state.region_id}`,
    );
  }

  return {
    region_id: definition.id,
    state:
      state.panel_instance_ids.length > 0 ||
      definition.empty_policy === "keep_visible"
        ? "visible"
        : "hidden",
  };
}

export function isRegionPresented(state: RegionVisibilityState): boolean {
  return state !== "hidden";
}
