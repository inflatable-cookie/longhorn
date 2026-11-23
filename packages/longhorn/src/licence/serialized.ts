import { wireCopy } from "../core/wire-copy.ts";
import type {
  LicenceActivateCommand,
  LicenceDeactivateCommand,
  LicenceRefreshCommand,
  LicenceReleaseSeatCommand,
  LicenceRenameSeatCommand,
} from "./generated/protocol.ts";
import type { LicencePort, LicenceUnlisten } from "./ports.ts";
export class SerializedLicencePort implements LicencePort {
  constructor(readonly inner: LicencePort) {}
  async snapshot(): Promise<unknown> {
    return wireCopy(await this.inner.snapshot());
  }
  async activate(command: LicenceActivateCommand): Promise<unknown> {
    return wireCopy(await this.inner.activate(wireCopy(command)));
  }
  async deactivate(command: LicenceDeactivateCommand): Promise<unknown> {
    return wireCopy(await this.inner.deactivate(wireCopy(command)));
  }
  async refresh(command: LicenceRefreshCommand): Promise<unknown> {
    return wireCopy(await this.inner.refresh(wireCopy(command)));
  }
  async releaseSeat(command: LicenceReleaseSeatCommand): Promise<unknown> {
    return wireCopy(await this.inner.releaseSeat(wireCopy(command)));
  }
  async renameSeat(command: LicenceRenameSeatCommand): Promise<unknown> {
    return wireCopy(await this.inner.renameSeat(wireCopy(command)));
  }
  async listen(listener: (event: unknown) => void): Promise<LicenceUnlisten> {
    if (this.inner.listen === undefined) {
      return () => {};
    }
    return this.inner.listen((event) => listener(wireCopy(event)));
  }
}
