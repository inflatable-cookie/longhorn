import {
  CommandSession,
} from "../src/poodle.ts";
import {
  CommandController,
  type CommandExecutionIntent,
  type CommandPorts,
  type CommandUnlisten,
  type KeyboardEventLike,
} from "../src/index.ts";
import { availability, fixture } from "../tests/support.ts";

class Port<Snapshot> {
  readonly listeners = new Set<() => void>();

  constructor(readonly value: Snapshot) {}

  async load(): Promise<Snapshot> {
    return this.value;
  }

  listen(listener: () => void): CommandUnlisten {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }
}

export class FakeKeyboardTarget {
  readonly listeners = new Set<(event: KeyboardEventLike) => void>();

  addEventListener(
    _type: "keydown",
    listener: (event: KeyboardEventLike) => void,
  ): void {
    this.listeners.add(listener);
  }

  removeEventListener(
    _type: "keydown",
    listener: (event: KeyboardEventLike) => void,
  ): void {
    this.listeners.delete(listener);
  }

  dispatch(event: KeyboardEventLike): void {
    for (const listener of this.listeners) listener(event);
  }
}

export function createSession() {
  const target = new FakeKeyboardTarget();
  const catalogue = new Port(fixture.catalogue);
  const keymap = new Port(fixture.loadOutcomes[0]!);
  const available = new Port(availability());
  const executions: CommandExecutionIntent[] = [];
  let request = 0;
  const ports: CommandPorts = {
    catalogue,
    keymap,
    availability: available,
    executor: {
      async execute(intent) {
        executions.push(intent);
        return { status: "succeeded" };
      },
    },
    nextRequestId: () => `request:svelte-${++request}`,
  };
  const controller = new CommandController({
    ports,
    platform: "macOs",
  });
  const session = new CommandSession({
    controller,
    platform: "macOs",
    contextPath: () => ["global"],
    keyboardTarget: target,
  });
  return { session, target, executions };
}
