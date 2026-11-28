import { OPAQUE_ID_RULE } from "./generated/identifiers.ts";

/** Whether Rust would admit `value` as an opaque Longhorn identifier. */
export function isOpaqueId(value: unknown): value is string {
  // Every admitted character is one byte, so the length is also the byte count.
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= OPAQUE_ID_RULE.maximumBytes &&
    Array.from(value).every((character) => OPAQUE_ID_RULE.characters.includes(character))
  );
}

/** The shape of a generated `DomainId` grammar. */
export interface DomainIdRule {
  readonly maximumBytes: number;
  readonly separator: string;
  readonly segmentInitial: string;
  readonly segmentAdditional: string;
}

/** Whether Rust would admit `value` as a `DomainId` under the generated rule. */
export function matchesDomainIdRule(value: unknown, rule: DomainIdRule): value is string {
  const { maximumBytes, separator, segmentInitial, segmentAdditional } = rule;
  return (
    typeof value === "string" &&
    new TextEncoder().encode(value).length <= maximumBytes &&
    value.split(separator).every((segment) => {
      const [first, ...rest] = segment;
      return (
        first !== undefined &&
        segmentInitial.includes(first) &&
        rest.every(
          (character) =>
            segmentInitial.includes(character) || segmentAdditional.includes(character),
        )
      );
    })
  );
}
