export {
  ConfigProtocolIncompatibilityError,
  type ConfigProtocolIncompatibilityCode,
} from "./validation/primitives.ts";
export * from "./validation/commands.ts";
export * from "./validation/outcomes.ts";
export { assertCompatibleConfigOperationsSnapshot } from "./validation/projection.ts";
