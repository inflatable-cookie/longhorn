// Record members stay unknown until a domain validator checks them.
export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function readWireRecord(
  value: unknown,
  invalidObject: () => never,
): Record<string, unknown> {
  return isRecord(value) ? value : invalidObject();
}

// A required wire field must be an own, enumerable property to appear in JSON.
export function checkWireFields(
  value: Record<string, unknown>,
  required: ReadonlyArray<string> | undefined,
  invalidField: (code: "unknown_field" | "missing_field", key: string) => never,
  optional: ReadonlyArray<string> = [],
): void {
  if (required === undefined) {
    return;
  }
  const present = new Set(Object.keys(value));
  const allowed = new Set([...required, ...optional]);
  for (const key of present) {
    if (!allowed.has(key)) {
      invalidField("unknown_field", key);
    }
  }
  for (const key of required) {
    if (!present.has(key)) {
      invalidField("missing_field", key);
    }
  }
}

export function wireVariantFields(
  fields: Readonly<Record<string, Readonly<Record<string, ReadonlyArray<string>>>>>,
  discriminants: Readonly<Record<string, string>>,
  type: string,
  value: Record<string, unknown>,
  invalidVariant: (discriminant: unknown) => never,
): ReadonlyArray<string> {
  const tag = discriminants[type];
  const discriminant = tag === undefined ? undefined : value[tag];
  const variants = fields[type];
  if (
    typeof discriminant !== "string" ||
    variants === undefined ||
    !Object.hasOwn(variants, discriminant)
  ) {
    return invalidVariant(discriminant);
  }
  return variants[discriminant] ?? invalidVariant(discriminant);
}

export function unsignedInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

export function unsigned32(value: unknown): value is number {
  const maximum = 0xff_ff_ff_ff;
  return unsignedInteger(value) && value <= maximum;
}
