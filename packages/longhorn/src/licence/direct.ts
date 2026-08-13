import type { LicencePort } from "./ports.ts";
export type DirectLicenceHandlers = LicencePort;
export function createDirectLicencePort(handlers: DirectLicenceHandlers): LicencePort { return handlers; }
