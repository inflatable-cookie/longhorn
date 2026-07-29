import type {
  LayoutContainer,
  LayoutDocument,
  LayoutMutationRequest,
  LayoutSchemaDefinition,
  PanelDefinition,
  PanelInstance,
  PanelInstanceId,
  RegionDefinition,
  RegionState,
  SizingSlotDefinition,
} from "@longhorn/layout";
import type { LayoutDispatchResult } from "@longhorn/svelte/layout";
import type { IconProp, PanelTabItem } from "@poodle/svelte";

export interface LayoutMutationState {
  readonly projected: LayoutDocument | undefined;
  dispatch(
    request: LayoutMutationRequest,
    project: (document: LayoutDocument) => LayoutDocument,
  ): Promise<LayoutDispatchResult>;
}

export interface PoodleLayoutDefinitions {
  readonly schemas: readonly LayoutSchemaDefinition[];
  readonly panels: readonly PanelDefinition[];
}

export interface PanelPresentation {
  readonly label: string;
  readonly icon?: IconProp | null;
}

export type PanelPresentationResolver = (
  instance: PanelInstance,
) => PanelPresentation | null;

export interface PanelRenderContext {
  readonly instance: PanelInstance;
  readonly definition: PanelDefinition;
  readonly presentation: PanelPresentation;
}

export interface RegionProjection {
  readonly container: LayoutContainer;
  readonly definition: RegionDefinition;
  readonly state: RegionState;
  readonly items: readonly PanelTabItem[];
  readonly panels: ReadonlyMap<PanelInstanceId, PanelRenderContext>;
  readonly active: PanelRenderContext | null;
}

export interface SizingSlotProjection {
  readonly definition: SizingSlotDefinition;
  readonly ratio: number;
}

export interface PoodleLayoutBindingOptions {
  readonly state: LayoutMutationState;
  readonly definitions: PoodleLayoutDefinitions;
  readonly nextRequestId: () => string;
  readonly onError: (error: unknown) => void;
  readonly onResult?: (result: LayoutDispatchResult) => void;
}

export class MissingLayoutDocumentError extends Error {
  constructor() {
    super("Poodle layout binding requires a projected layout document");
    this.name = "MissingLayoutDocumentError";
  }
}

export class MissingLayoutMemberError extends Error {
  constructor(kind: string, id: string) {
    super(`Poodle layout binding cannot resolve ${kind}: ${id}`);
    this.name = "MissingLayoutMemberError";
  }
}

export class MissingPanelPresentationError extends Error {
  constructor(panelInstanceId: PanelInstanceId) {
    super(
      `consumer supplied no presentation for panel instance ${panelInstanceId}`,
    );
    this.name = "MissingPanelPresentationError";
  }
}
