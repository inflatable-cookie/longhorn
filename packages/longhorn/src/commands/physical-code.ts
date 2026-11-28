import { COMMAND_MAXIMUM_PHYSICAL_CODE_BYTES } from "./generated/protocol.ts";

/** Whether Rust's `CommandPhysicalCode` would admit `value`. */
export function isPhysicalCode(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value !== "Unidentified" &&
    value.length <= COMMAND_MAXIMUM_PHYSICAL_CODE_BYTES &&
    /^[A-Z][A-Za-z0-9]*$/u.test(value)
  );
}
