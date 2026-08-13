import type { LicenceActivateCommand, LicenceDeactivateCommand, LicenceRefreshCommand, LicenceReleaseSeatCommand } from "./generated/protocol.ts";
import type { LicencePort, LicenceUnlisten } from "./ports.ts";
export class SerializedLicencePort implements LicencePort {
  constructor(readonly inner: LicencePort) {}
  async snapshot(): Promise<unknown> { return clone(await this.inner.snapshot()); }
  async activate(command: LicenceActivateCommand): Promise<unknown> { return clone(await this.inner.activate(clone(command))); }
  async deactivate(command: LicenceDeactivateCommand): Promise<unknown> { return clone(await this.inner.deactivate(clone(command))); }
  async refresh(command: LicenceRefreshCommand): Promise<unknown> { return clone(await this.inner.refresh(clone(command))); }
  async releaseSeat(command: LicenceReleaseSeatCommand): Promise<unknown> { return clone(await this.inner.releaseSeat(clone(command))); }
  async listen(listener: (event: unknown) => void): Promise<LicenceUnlisten> { if (this.inner.listen === undefined) return () => {}; return this.inner.listen((event) => listener(clone(event))); }
}
function clone<T>(value: T): T { return JSON.parse(JSON.stringify(value)) as T; }
