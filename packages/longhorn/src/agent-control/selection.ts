//! Consumer call-site replacement for plugin-dialog `open`/`save`.
//!
//! Agent-originated calls publish a pending selection before any OS panel
//! opens. Human-originated calls pass the original options to the plugin
//! unchanged. Origin is the in-page shim, not an active-server flag.

import type { BeginSelectionArgs, SelectionFilter } from "./generated/protocol.ts";
import { SHIM_GLOBAL } from "./shim.ts";
import type { AgentControlApi } from "./shim.ts";

/** Renderer invoke name. Host-protocol pairs this with the Rust command. */
export const BEGIN_SELECTION_COMMAND = "longhorn_agent_control_begin_selection";

/** Pending-selection MCP resource. */
export const SELECTION_RESOURCE_URI = "longhorn://agent-control/selection";

/** Wire token for agent origin. Matches the Rust `SELECTION_ORIGIN_AGENT`. */
export const SELECTION_ORIGIN_AGENT = "agent";

/** Wire token for human origin. Matches the Rust `SELECTION_ORIGIN_HUMAN`. */
export const SELECTION_ORIGIN_HUMAN = "human";

/** Serializable origin a consumer passes into its own command arguments. */
export type SelectionOrigin = typeof SELECTION_ORIGIN_AGENT | typeof SELECTION_ORIGIN_HUMAN;

export type DialogFilter = SelectionFilter;

export type OpenDialogOptions = {
  title?: string;
  defaultPath?: string;
  filters?: Array<DialogFilter>;
  multiple?: boolean;
  directory?: boolean;
};

export type SaveDialogOptions = {
  title?: string;
  defaultPath?: string;
  filters?: Array<DialogFilter>;
};

export type OpenDialogResult = string | Array<string> | null;
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

/**
 * Page origin as a command-argument token. Reads the installed shim;
 * with no shim, or when the shim reports human, the token is `"human"`.
 */
// oxlint-disable-next-line quality/no-object-parameters -- The world is a page global with an optional shim, including plain host test doubles.
export function currentSelectionOrigin(world: object = globalThis): SelectionOrigin {
  return isAgentOriginated(world) ? SELECTION_ORIGIN_AGENT : SELECTION_ORIGIN_HUMAN;
}

function beginOptions(
  kind: BeginSelectionArgs["kind"],
  options: OpenDialogOptions | undefined,
): BeginSelectionArgs {
  const open = kind === "open" ? options : undefined;
  const { filters = [], title, defaultPath } = options ?? {};
  return {
    kind,
    directory: open?.directory ?? false,
    multiple: open?.multiple ?? false,
    filters,
    ...(title === undefined ? {} : { title }),
    ...(defaultPath === undefined ? {} : { defaultPath }),
  };
}

/**
 * Binds plugin-compatible `open`/`save`. Call at consumer picker sites;
 * do not monkey-patch plugin-dialog.
 */
export function bindFileSelection(
  ports: FileSelectionPorts,
  // oxlint-disable-next-line quality/no-object-parameters -- The world is a page global with an optional shim.
  world: object = globalThis,
): FileSelectionApi {
  return {
    open: async (options?: OpenDialogOptions) => {
      if (!isAgentOriginated(world)) {
        return ports.pluginOpen(options);
      }
      const result = await ports.invoke(BEGIN_SELECTION_COMMAND, {
        options: beginOptions("open", options),
      });
      if (result === null || typeof result === "string") {
        return result;
      }
      if (
        Array.isArray(result) &&
        result.every((path): path is string => typeof path === "string")
      ) {
        return result;
      }
      throw new TypeError("invalid agent open-dialog result");
    },
    save: async (options?: SaveDialogOptions) => {
      if (!isAgentOriginated(world)) {
        return ports.pluginSave(options);
      }
      const result = await ports.invoke(BEGIN_SELECTION_COMMAND, {
        options: beginOptions("save", options),
      });
      if (result === null || typeof result === "string") {
        return result;
      }
      throw new TypeError("invalid agent save-dialog result");
    },
  };
}
