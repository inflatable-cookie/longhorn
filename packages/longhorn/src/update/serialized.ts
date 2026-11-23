import { wireCopy } from "../core/wire-copy.ts";
import type {
  UpdateApplyCommand,
  UpdateCancelCommand,
  UpdateCheckCommand,
  UpdateDeferCommand,
  UpdatePrepareCommand,
  UpdateSelectChannelCommand,
} from "./generated/protocol.ts";
import type { UpdatePort, UpdateUnlisten } from "./ports.ts";
export class SerializedUpdatePort implements UpdatePort {
  constructor(readonly inner: UpdatePort) {}
  async snapshot(): Promise<unknown> {
    return wireCopy(await this.inner.snapshot());
  }
  async check(command: UpdateCheckCommand): Promise<unknown> {
    return wireCopy(await this.inner.check(wireCopy(command)));
  }
  async selectChannel(command: UpdateSelectChannelCommand): Promise<unknown> {
    return wireCopy(await this.inner.selectChannel(wireCopy(command)));
  }
  async defer(command: UpdateDeferCommand): Promise<unknown> {
    return wireCopy(await this.inner.defer(wireCopy(command)));
  }
  async prepare(command: UpdatePrepareCommand): Promise<unknown> {
    return wireCopy(await this.inner.prepare(wireCopy(command)));
  }
  async apply(command: UpdateApplyCommand): Promise<unknown> {
    return wireCopy(await this.inner.apply(wireCopy(command)));
  }
  async cancel(command: UpdateCancelCommand): Promise<unknown> {
    return wireCopy(await this.inner.cancel(wireCopy(command)));
  }
  async listen(listener: (event: unknown) => void): Promise<UpdateUnlisten> {
    if (this.inner.listen === undefined) {
      return () => {};
    }
    return this.inner.listen((event) => listener(wireCopy(event)));
  }
  async listenProgress(listener: (event: unknown) => void): Promise<UpdateUnlisten> {
    if (this.inner.listenProgress === undefined) {
      return () => {};
    }
    return this.inner.listenProgress((event) => listener(wireCopy(event)));
  }
}
