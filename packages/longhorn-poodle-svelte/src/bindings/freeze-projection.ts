// Only call this on owned protocol data. Errors and other caller values stay opaque.
export function freezeProjection<Value>(value: Value): Value {
  if (typeof value === "object" && value !== null && !Object.isFrozen(value)) {
    Object.freeze(value);
    const children: Array<unknown> = Object.values(value);
    for (const child of children) {
      freezeProjection(child);
    }
  }
  return value;
}
