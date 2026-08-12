// Not a validation module: `validationModules` looks for `validation*.ts`
// under `src/`, so this file is only ever read by the test that names it.
//
// It exists so the scan is proved to bite. A parameter-matching bug reads
// exactly like a clean codebase otherwise, and this milestone's whole subject
// is checks that report nothing when they should report something.

/** The allowed-keys shape: matched by parameter name and type. */
export function keys(value: Record<string, unknown>, path: string, allowed: readonly string[]) {
  return [value, path, allowed];
}

/** The discriminant shape: same type, different meaning. Must not match. */
export function member(value: unknown, values: readonly string[], path: string) {
  return [value, values, path];
}

export function planted(value: Record<string, unknown>) {
  keys(value, "$", ["kind", "entryId"]);
  member(value.kind, ["preset", "replacement"], "$.kind");
}
