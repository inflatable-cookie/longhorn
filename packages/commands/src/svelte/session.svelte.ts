import {
  routeCommandKeyboardEvent,
  type KeyboardEventLike,
} from "../keyboard.ts";
import type {
  CommandKeyChord,
  CommandKeymapPatch,
  CommandPlatform,
} from "../generated/protocol.ts";
import type {
  CommandControllerStatus,
  CommandMutationStatus,
} from "../controller.ts";
import { CommandController } from "../controller.ts";
import type {
  CommandProjectionRecord,
  CommandSettingsRecord,
} from "../projectors.ts";

export interface CommandKeyboardTarget {
  addEventListener(
    type: "keydown",
    listener: (event: KeyboardEventLike) => void,
  ): void;
  removeEventListener(
    type: "keydown",
    listener: (event: KeyboardEventLike) => void,
  ): void;
}

export interface CommandSessionOptions {
  readonly controller: CommandController;
  readonly platform: CommandPlatform;
  readonly contextPath: () => readonly string[];
  readonly keyboardTarget?: CommandKeyboardTarget;
  readonly reserved?: (
    platform: CommandPlatform,
    chord: CommandKeyChord,
  ) => boolean;
}

export interface CapturedCommandChord {
  readonly bindingId: string;
  readonly chord: CommandKeyChord;
  readonly label: string;
}

export class CommandSession {
  readonly #controller: CommandController;
  readonly #options: CommandSessionOptions;
  #revision = $state(0);
  #open = $state(false);
  #query = $state("");
  #captureBindingId = $state<string | undefined>(undefined);
  #captured = $state.raw<CapturedCommandChord | undefined>(undefined);
  #removeObserver: (() => void) | undefined;
  #listening = false;

  readonly #keydown = (event: KeyboardEventLike): void => {
    this.handleKeyboard(event);
  };

  constructor(options: CommandSessionOptions) {
    this.#controller = options.controller;
    this.#options = options;
  }

  get status(): CommandControllerStatus {
    this.#revision;
    return this.#controller.status;
  }

  get mutation(): CommandMutationStatus {
    this.#revision;
    return this.#controller.mutation;
  }

  get projection() {
    this.#revision;
    return this.#controller.projection;
  }

  get paletteRecords(): readonly CommandProjectionRecord[] {
    const records = this.projection?.palette ?? [];
    if (this.#query.length === 0) return records;
    const hits = new Set(
      this.#controller.searchHits.map(({ record }) => record.id),
    );
    return records.filter(({ id }) => hits.has(id));
  }

  get settingsRecords(): readonly CommandSettingsRecord[] {
    return this.projection?.settings ?? [];
  }

  get open(): boolean {
    return this.#open;
  }

  set open(value: boolean) {
    this.#open = value;
  }

  get query(): string {
    return this.#query;
  }

  get captureBindingId(): string | undefined {
    return this.#captureBindingId;
  }

  get captured(): CapturedCommandChord | undefined {
    return this.#captured;
  }

  get dirty(): boolean {
    this.#revision;
    return this.#controller.dirty;
  }

  async start(): Promise<void> {
    if (this.#removeObserver === undefined) {
      this.#removeObserver = this.#controller.observe(() => {
        this.#revision += 1;
      });
    }
    if (!this.#listening && this.#options.keyboardTarget !== undefined) {
      this.#options.keyboardTarget.addEventListener("keydown", this.#keydown);
      this.#listening = true;
    }
    await this.#controller.start();
  }

  async stop(): Promise<void> {
    if (this.#listening && this.#options.keyboardTarget !== undefined) {
      this.#options.keyboardTarget.removeEventListener(
        "keydown",
        this.#keydown,
      );
      this.#listening = false;
    }
    this.#removeObserver?.();
    this.#removeObserver = undefined;
    this.#captureBindingId = undefined;
    this.#captured = undefined;
    this.#open = false;
    this.#query = "";
    await this.#controller.stop();
  }

  setOpen(open: boolean): void {
    this.#open = open;
  }

  async setQuery(query: string): Promise<void> {
    this.#query = query;
    await this.#controller.search(query);
  }

  select(commandId: string): Promise<unknown> {
    return this.#controller.dispatch({
      commandId,
      source: "palette",
    });
  }

  beginCapture(bindingId: string): void {
    this.#captureBindingId = bindingId;
    this.#captured = undefined;
  }

  cancelCapture(): void {
    this.#captureBindingId = undefined;
    this.#captured = undefined;
  }

  stagePatch(patch: CommandKeymapPatch): void {
    this.#controller.stageKeymapPatch(patch);
  }

  applyDraft(): Promise<boolean> {
    return this.#controller.applyKeymapDraft();
  }

  cancelDraft(): void {
    this.#controller.cancelKeymapDraft();
  }

  resetKeymap(): Promise<boolean> {
    return this.#controller.resetKeymap();
  }

  handleKeyboard(event: KeyboardEventLike): void {
    const model = this.#controller.model;
    if (model === undefined) return;
    routeCommandKeyboardEvent(event, {
      platform: this.#options.platform,
      contextPath: this.#options.contextPath(),
      mode: this.#captureBindingId === undefined ? "dispatch" : "capture",
      bindings: model.keymap.bindings,
      commands: model.catalogue.commands,
      reserved: this.#options.reserved,
      dispatcher: this.#controller,
      onCapture: (chord, label) => {
        const bindingId = this.#captureBindingId;
        if (bindingId === undefined) return;
        this.#captured = { bindingId, chord, label };
        this.#captureBindingId = undefined;
      },
    });
  }
}
