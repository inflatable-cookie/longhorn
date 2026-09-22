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
  async snapshot(): Promise<unknown> { return clone(await this.inner.snapshot()); }
  async check(command: UpdateCheckCommand): Promise<unknown> { return clone(await this.inner.check(clone(command))); }
  async selectChannel(command: UpdateSelectChannelCommand): Promise<unknown> { return clone(await this.inner.selectChannel(clone(command))); }
  async defer(command: UpdateDeferCommand): Promise<unknown> { return clone(await this.inner.defer(clone(command))); }
  async prepare(command: UpdatePrepareCommand): Promise<unknown> { return clone(await this.inner.prepare(clone(command))); }
  async apply(command: UpdateApplyCommand): Promise<unknown> { return clone(await this.inner.apply(clone(command))); }
  async cancel(command: UpdateCancelCommand): Promise<unknown> { return clone(await this.inner.cancel(clone(command))); }
  async listen(listener: (event: unknown) => void): Promise<UpdateUnlisten> { if (this.inner.listen === undefined) return () => {}; return this.inner.listen((event) => listener(clone(event))); }
  async listenProgress(listener: (event: unknown) => void): Promise<UpdateUnlisten> { if (this.inner.listenProgress === undefined) return () => {}; return this.inner.listenProgress((event) => listener(clone(event))); }
}
function clone<T>(value: T): T { return JSON.parse(JSON.stringify(value)) as T; }
