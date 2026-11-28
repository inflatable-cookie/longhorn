import { listenChecked, reportFailure } from "../core/connection.ts";
import type { ConnectionFailureReporter } from "../core/connection.ts";
import type {
  UpdateApplyCommand,
  UpdateCancelCommand,
  UpdateChangedEvent,
  UpdateCheckCommand,
  UpdateDeferCommand,
  UpdateOutcomeProjection,
  UpdatePrepareCommand,
  UpdateProgressEvent,
  UpdateSelectChannelCommand,
  UpdateSnapshot,
} from "./generated/protocol.ts";
import type { CheckedUpdatePort, UpdatePort, UpdateUnlisten } from "./ports.ts";
import {
  assertUpdateApplyCommand,
  assertUpdateCancelCommand,
  assertUpdateChangedEvent,
  assertUpdateCheckCommand,
  assertUpdateDeferCommand,
  assertUpdateOutcome,
  assertUpdatePrepareCommand,
  assertUpdateProgressEvent,
  assertUpdateSelectChannelCommand,
  assertUpdateSnapshot,
} from "./validation.ts";

export class UpdateClient implements CheckedUpdatePort {
  constructor(readonly port: UpdatePort) {}
  async snapshot(): Promise<UpdateSnapshot> {
    const value = await this.port.snapshot();
    assertUpdateSnapshot(value);
    return value;
  }
  async check(command: UpdateCheckCommand): Promise<UpdateOutcomeProjection> {
    assertUpdateCheckCommand(command);
    const value = await this.port.check(command);
    assertUpdateOutcome(value);
    return value;
  }
  async selectChannel(command: UpdateSelectChannelCommand): Promise<UpdateOutcomeProjection> {
    assertUpdateSelectChannelCommand(command);
    const value = await this.port.selectChannel(command);
    assertUpdateOutcome(value);
    return value;
  }
  async defer(command: UpdateDeferCommand): Promise<UpdateOutcomeProjection> {
    assertUpdateDeferCommand(command);
    const value = await this.port.defer(command);
    assertUpdateOutcome(value);
    return value;
  }
  async prepare(command: UpdatePrepareCommand): Promise<UpdateOutcomeProjection> {
    assertUpdatePrepareCommand(command);
    const value = await this.port.prepare(command);
    assertUpdateOutcome(value);
    return value;
  }
  /**
   * Prepares with live reports when the port can scope them to this invocation.
   *
   * A malformed report ends delivery for this invocation and is reported to
   * `onFailure` with phase `event`; the prepare itself still settles.
   */
  async prepareWithProgress(
    command: UpdatePrepareCommand,
    listener: (event: UpdateProgressEvent) => void,
    onFailure?: ConnectionFailureReporter,
  ): Promise<UpdateOutcomeProjection> {
    if (this.port.prepareWithProgress === undefined) {
      return this.prepare(command);
    }
    assertUpdatePrepareCommand(command);
    let malformed = false;
    const value = await this.port.prepareWithProgress(command, (event) => {
      if (malformed) {
        return;
      }
      try {
        assertUpdateProgressEvent(event);
      } catch (error) {
        malformed = true;
        reportFailure(onFailure, { phase: "event", error });
        return;
      }
      listener(event);
    });
    assertUpdateOutcome(value);
    return value;
  }
  async apply(command: UpdateApplyCommand): Promise<UpdateOutcomeProjection> {
    assertUpdateApplyCommand(command);
    const value = await this.port.apply(command);
    assertUpdateOutcome(value);
    return value;
  }
  async cancel(command: UpdateCancelCommand): Promise<UpdateOutcomeProjection> {
    assertUpdateCancelCommand(command);
    const value = await this.port.cancel(command);
    assertUpdateOutcome(value);
    return value;
  }
  listen(
    listener: (event: UpdateChangedEvent) => void,
    onFailure?: ConnectionFailureReporter,
  ): Promise<UpdateUnlisten> {
    return listenChecked({
      listen: (receive) => this.port.listen?.(receive) ?? (() => {}),
      validate: assertUpdateChangedEvent,
      listener,
      onFailure,
    });
  }
}
