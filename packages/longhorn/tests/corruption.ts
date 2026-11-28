import { expect } from "bun:test";

export interface CorruptionOptions {
  /** Subtrees that carry raw JSON, which no validator inspects. */
  readonly raw?: (path: string) => boolean;
  /** Records keyed by data, from which an entry may be absent. */
  readonly map?: (path: string) => boolean;
}

/**
 * Asserts that `validate` admits `valid` and refuses every single-point
 * corruption of it: an added key, a dropped key, or any nested value replaced
 * by an unrelated record. A member no validator reads survives one of these.
 */
export function expectStrict(
  validate: (value: unknown) => void,
  valid: unknown,
  options: CorruptionOptions = {},
): void {
  expect(() => validate(structuredClone(valid))).not.toThrow();
  const admitted = corruptions(valid, "$", options).filter(({ payload }) => {
    try {
      validate(payload);
      return true;
    } catch {
      return false;
    }
  });
  expect(admitted.map(({ label }) => label)).toEqual([]);
}

interface Corruption {
  readonly label: string;
  readonly payload: unknown;
}

function corruptions(value: unknown, path: string, options: CorruptionOptions): Array<Corruption> {
  if (options.raw?.(path) === true) {
    return [];
  }
  const found: Array<Corruption> =
    path === "$" ? [] : [{ label: `replaced ${path}`, payload: { corrupt: true } }];
  if (Array.isArray(value)) {
    value.forEach((entry, index) => {
      for (const { label, payload } of corruptions(entry, `${path}.${index}`, options)) {
        const copy: Array<unknown> = structuredClone(value);
        copy[index] = payload;
        found.push({ label, payload: copy });
      }
    });
  } else if (typeof value === "object" && value !== null) {
    const record = value as Record<string, unknown>;
    found.push({
      label: `added ${path}.unexpected_key`,
      payload: { ...record, unexpected_key: 1 },
    });
    for (const [key, entry] of Object.entries(record)) {
      if (options.map?.(path) !== true) {
        const { [key]: _dropped, ...rest } = record;
        found.push({ label: `dropped ${path}.${key}`, payload: rest });
      }
      for (const { label, payload } of corruptions(entry, `${path}.${key}`, options)) {
        found.push({ label, payload: { ...record, [key]: payload } });
      }
    }
  }
  return found;
}
