import { assertForkBranchCommand, assertForkBranchPage, assertForkContinuationCommand, assertForkContinuationPage, assertForkDeleteCommand, assertForkPruneCommand, assertForkPruneResult, assertForkRemovalReceipt, assertForkChangedEvent, assertForkNavigationCommand, assertForkNavigationResult, assertForkPathCommand, assertForkPathPage, assertForkSnapshot } from "./validation.ts";
import type { ForkBranchPageCommand, ForkBranchPageSnapshot, ForkChangedEvent, ForkContinuationPageCommand, ForkContinuationPageSnapshot, ForkDeleteContinuationCommand, ForkPruneCommand, ForkPruneResult, ForkRemovalReceiptProjection, ForkNavigationCommand, ForkNavigationResult, ForkPathPageCommand, ForkPathPageSnapshot, ForkSnapshot } from "./generated/protocol.ts";
import type { CheckedForkHistoryPort, ForkHistoryPort, ForkHistoryUnlisten } from "./ports.ts";

export class ForkHistoryClient implements CheckedForkHistoryPort {
  constructor(readonly port: ForkHistoryPort) {}
  nextPlanId(): string { return this.port.nextPlanId(); }
  async snapshot(): Promise<ForkSnapshot> { const value = await this.port.snapshot(); assertForkSnapshot(value); return value; }
  async path(command: ForkPathPageCommand): Promise<ForkPathPageSnapshot> { assertForkPathCommand(command); const value = await this.port.path(command); assertForkPathPage(value); return value; }
  async branches(command: ForkBranchPageCommand): Promise<ForkBranchPageSnapshot> { assertForkBranchCommand(command); const value = await this.port.branches(command); assertForkBranchPage(value); return value; }
  async continuations(command: ForkContinuationPageCommand): Promise<ForkContinuationPageSnapshot> { assertForkContinuationCommand(command); const value = await this.port.continuations(command); assertForkContinuationPage(value); return value; }
  async deleteContinuation(command: ForkDeleteContinuationCommand): Promise<ForkRemovalReceiptProjection> { assertForkDeleteCommand(command); const value = await this.port.deleteContinuation(command); assertForkRemovalReceipt(value); return value; }
  async prune(command: ForkPruneCommand): Promise<ForkPruneResult> { assertForkPruneCommand(command); const value = await this.port.prune(command); assertForkPruneResult(value); return value; }
  async navigate(command: ForkNavigationCommand): Promise<ForkNavigationResult> { assertForkNavigationCommand(command); const value = await this.port.navigate(command); assertForkNavigationResult(value); return value; }
  async listen(listener: (event: ForkChangedEvent) => void): Promise<ForkHistoryUnlisten> { if (this.port.listen === undefined) return () => {}; return this.port.listen((value) => { assertForkChangedEvent(value); listener(value); }); }
}
