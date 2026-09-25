//! Consumer call-site replacement for plugin-dialog `open`/`save`.
//!
//! Agent-originated calls publish a pending selection before any OS panel
//! opens. Human-originated calls pass the original options to the plugin
//! unchanged. Origin is the in-page shim, not an active-server flag.

import { SHIM_GLOBAL, type AgentControlApi } from "./shim.ts";

/** Renderer invoke name. Host-protocol pairs this with the Rust command. */
export const BEGIN_SELECTION_COMMAND = "longhorn_agent_control_begin_selection";

/** Pending-selection MCP resource. */
export const SELECTION_RESOURCE_URI = "longhorn://agent-control/selection";

export type DialogFilter = {
  name: string;
  extensions: string[];
};

export type OpenDialogOptions = {
  title?: string;
  defaultPath?: string;
  filters?: DialogFilter[];
  multiple?: boolean;
  directory?: boolean;
};

export type SaveDialogOptions = {
  title?: string;
  defaultPath?: string;
  filters?: DialogFilter[];
};

export type OpenDialogResult = string | string[] | null;
export type SaveDialogResult = string | null;

export type FileSelectionPorts = {
  invoke: (command: string, args: Record<string, unknown>) => Promise<unknown>;
  pluginOpen: (options?: OpenDialogOptions) => Promise<OpenDialogResult>;
  pluginSave: (options?: SaveDialogOptions) => Promise<SaveDialogResult>;
};

export type FileSelectionWorld = {
  readonly [SHIM_GLOBAL]?: Pick<AgentControlApi, "isAgentOriginated">;
};

export type FileSelectionApi = {
  open: (options?: OpenDialogOptions) => Promise<OpenDialogResult>;
  save: (options?: SaveDialogOptions) => Promise<SaveDialogResult>;
};

function isAgentOriginated(world: FileSelectionWorld): boolean {
  return world[SHIM_GLOBAL]?.isAgentOriginated?.() === true;
}

function beginOptions(
  kind: "open" | "save",
  options: OpenDialogOptions | SaveDialogOptions | undefined,
): Record<string, unknown> {
  const open = kind === "open" ? (options as OpenDialogOptions | undefined) : undefined;
  return {
    kind,
    directory: open?.directory ?? false,
    multiple: open?.multiple ?? false,
    filters: options?.filters ?? [],
    title: options?.title,
    defaultPath: options?.defaultPath,
  };
}

/**
 * Binds plugin-compatible `open`/`save`. Call at consumer picker sites;
 * do not monkey-patch plugin-dialog.
 */
export function bindFileSelection(
  ports: FileSelectionPorts,
  world: object = globalThis,
): FileSelectionApi {
  const originWorld = world as FileSelectionWorld;
  return {
    open: async (options?: OpenDialogOptions) => {
      if (!isAgentOriginated(originWorld)) return ports.pluginOpen(options);
      return (await ports.invoke(BEGIN_SELECTION_COMMAND, {
        options: beginOptions("open", options),
      })) as OpenDialogResult;
    },
    save: async (options?: SaveDialogOptions) => {
      if (!isAgentOriginated(originWorld)) return ports.pluginSave(options);
      return (await ports.invoke(BEGIN_SELECTION_COMMAND, {
        options: beginOptions("save", options),
      })) as SaveDialogResult;
    },
  };
}
