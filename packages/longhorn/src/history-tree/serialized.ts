import { wireCopy } from "../core/wire-copy.ts";
import type {
  ForkBranchPageCommand,
  ForkContinuationPageCommand,
  ForkDeleteContinuationCommand,
  ForkNavigationCommand,
  ForkPathPageCommand,
  ForkPruneCommand,
} from "./generated/protocol.ts";
import type { ForkHistoryPort, ForkHistoryUnlisten } from "./ports.ts";
export class SerializedForkHistoryPort implements ForkHistoryPort {
  constructor(readonly inner: ForkHistoryPort) {}
  async snapshot(): Promise<unknown> {
    return wireCopy(await this.inner.snapshot());
  }
  async path(command: ForkPathPageCommand): Promise<unknown> {
    return wireCopy(await this.inner.path(wireCopy(command)));
  }
  async branches(command: ForkBranchPageCommand): Promise<unknown> {
    return wireCopy(await this.inner.branches(wireCopy(command)));
  }
  async continuations(command: ForkContinuationPageCommand): Promise<unknown> {
    return wireCopy(await this.inner.continuations(wireCopy(command)));
  }
  async deleteContinuation(command: ForkDeleteContinuationCommand): Promise<unknown> {
    return wireCopy(await this.inner.deleteContinuation(wireCopy(command)));
  }
  async prune(command: ForkPruneCommand): Promise<unknown> {
    return wireCopy(await this.inner.prune(wireCopy(command)));
  }
  async navigate(command: ForkNavigationCommand): Promise<unknown> {
    return wireCopy(await this.inner.navigate(wireCopy(command)));
  }
  async listen(listener: (event: unknown) => void): Promise<ForkHistoryUnlisten> {
    if (this.inner.listen === undefined) {
      return () => {};
    }
    return this.inner.listen((event) => listener(wireCopy(event)));
  }
  nextPlanId(): string {
    return this.inner.nextPlanId();
  }
}
