import { describe, expect, test } from "bun:test";

import {
  BEGIN_SELECTION_COMMAND,
  SELECTION_RESOURCE_URI,
  bindFileSelection,
  type FileSelectionPorts,
  type OpenDialogOptions,
  type OpenDialogResult,
  type SaveDialogOptions,
  type SaveDialogResult,
} from "../../src/agent-control/index.ts";
import { findByName, install, openPage } from "./support.ts";

type InvokeCall = { command: string; args: Record<string, unknown> };

function ports(): FileSelectionPorts & {
  opens: Array<OpenDialogOptions | undefined>;
  saves: Array<SaveDialogOptions | undefined>;
  invokes: InvokeCall[];
} {
  const opens: Array<OpenDialogOptions | undefined> = [];
  const saves: Array<SaveDialogOptions | undefined> = [];
  const invokes: InvokeCall[] = [];
  return {
    opens,
    saves,
    invokes,
    invoke: async (command, args) => {
      invokes.push({ command, args });
      return "/agent/path";
    },
    pluginOpen: async (options) => {
      opens.push(options);
      return "/human/open";
    },
    pluginSave: async (options) => {
      saves.push(options);
      return "/human/save";
    },
  };
}

describe("bindFileSelection", () => {
  test("host-protocol names the begin-selection command and resource", () => {
    expect(BEGIN_SELECTION_COMMAND).toBe("longhorn_agent_control_begin_selection");
    expect(SELECTION_RESOURCE_URI).toBe("longhorn://agent-control/selection");
  });

  test("human open passes the original options to plugin-dialog", async () => {
    const window = openPage(`<button>Import</button>`);
    install(window);
    const recorded = ports();
    const api = bindFileSelection(recorded, window);
    const options: OpenDialogOptions = { directory: true, title: "Choose folder" };
    const result = await api.open(options);
    expect(result).toBe("/human/open");
    expect(recorded.opens).toEqual([options]);
    expect(recorded.invokes).toEqual([]);
  });

  test("human save preserves plugin results while the shim is present", async () => {
    const window = openPage(`<button>Export</button>`);
    install(window);
    const recorded = ports();
    const api = bindFileSelection(recorded, window);
    const options: SaveDialogOptions = { defaultPath: "/tmp/out.txt" };
    const result: SaveDialogResult = await api.save(options);
    expect(result).toBe("/human/save");
    expect(recorded.saves).toEqual([options]);
    expect(recorded.invokes).toEqual([]);
  });

  test("omitted options stay omitted on the human path", async () => {
    const window = openPage(`<p>Idle</p>`);
    install(window);
    const recorded = ports();
    const api = bindFileSelection(recorded, window);
    await api.open();
    await api.save();
    expect(recorded.opens).toEqual([undefined]);
    expect(recorded.saves).toEqual([undefined]);
  });

  test("agent-originated open invokes the begin command and skips the plugin", async () => {
    const window = openPage(`<button>Import</button>`);
    const shim = install(window);
    const snapshot = shim.snapshot();
    expect(snapshot.ok).toBe(true);
    if (!snapshot.ok) return;
    expect(shim.click(findByName(snapshot.root, "Import")!.elementRef).ok).toBe(true);
    const recorded = ports();
    recorded.invoke = async (command, args) => {
      recorded.invokes.push({ command, args });
      return "/agent/folder";
    };
    const api = bindFileSelection(recorded, window);
    const options: OpenDialogOptions = { directory: true, title: "Choose folder" };
    const result: OpenDialogResult = await api.open(options);
    expect(result).toBe("/agent/folder");
    expect(recorded.opens).toEqual([]);
    expect(recorded.invokes).toEqual([
      {
        command: BEGIN_SELECTION_COMMAND,
        args: {
          options: {
            kind: "open",
            directory: true,
            multiple: false,
            filters: [],
            title: "Choose folder",
            defaultPath: undefined,
          },
        },
      },
    ]);
  });

  test("agent-originated save is target selection only", async () => {
    const window = openPage(`<button>Export</button>`);
    const shim = install(window);
    shim.markAgentOrigin();
    const recorded = ports();
    const api = bindFileSelection(recorded, window);
    const result = await api.save({ defaultPath: "/tmp/out.txt" });
    expect(result).toBe("/agent/path");
    expect(recorded.saves).toEqual([]);
    expect(recorded.invokes[0]?.args).toEqual({
      options: {
        kind: "save",
        directory: false,
        multiple: false,
        filters: [],
        title: undefined,
        defaultPath: "/tmp/out.txt",
      },
    });
  });
});
