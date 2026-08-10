export {
  ConfigProtocolValidationError,
  type ConfigProtocolValidationCode,
} from "./validation/primitives.ts";
export * from "./validation/commands.ts";
export * from "./validation/outcomes.ts";
export { assertValidConfigOperationsSnapshot } from "./validation/projection.ts";
