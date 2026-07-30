export {
  ConfigProtocolIncompatibilityError,
  type ConfigProtocolIncompatibilityCode,
} from "./compatibility/primitives.ts";
export * from "./compatibility/commands.ts";
export * from "./compatibility/outcomes.ts";
export { assertCompatibleConfigOperationsSnapshot } from "./compatibility/projection.ts";
