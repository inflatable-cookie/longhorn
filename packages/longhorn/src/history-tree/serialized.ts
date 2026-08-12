import type { ForkBranchPageCommand, ForkContinuationPageCommand, ForkDeleteContinuationCommand, ForkNavigationCommand, ForkPathPageCommand } from "./generated/protocol.ts";
import type { ForkHistoryPort, ForkHistoryUnlisten } from "./ports.ts";
export class SerializedForkHistoryPort implements ForkHistoryPort {
  constructor(readonly inner: ForkHistoryPort) {}
  async snapshot(): Promise<unknown> { return clone(await this.inner.snapshot()); }
  async path(command: ForkPathPageCommand): Promise<unknown> { return clone(await this.inner.path(clone(command))); }
  async branches(command: ForkBranchPageCommand): Promise<unknown> { return clone(await this.inner.branches(clone(command))); }
  async continuations(command: ForkContinuationPageCommand): Promise<unknown> { return clone(await this.inner.continuations(clone(command))); }
  async deleteContinuation(command: ForkDeleteContinuationCommand): Promise<unknown> { return clone(await this.inner.deleteContinuation(clone(command))); }
  async navigate(command: ForkNavigationCommand): Promise<unknown> { return clone(await this.inner.navigate(clone(command))); }
  async listen(listener: (event: unknown) => void): Promise<ForkHistoryUnlisten> { if (this.inner.listen === undefined) return () => {}; return this.inner.listen((event) => listener(clone(event))); }
  nextPlanId(): string { return this.inner.nextPlanId(); }
}
function clone<T>(value: T): T { return JSON.parse(JSON.stringify(value)) as T; }
