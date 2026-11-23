// Serialized ports simulate the JSON transport, including its handling of
// undefined fields, so a structured clone would exercise a different wire.
export function wireCopy<Value>(value: Value): Value {
  // oxlint-disable-next-line unicorn/prefer-structured-clone,typescript/no-unsafe-type-assertion -- Typed port inputs retain their wire type after the JSON round trip; checked clients validate the raw output.
  return JSON.parse(JSON.stringify(value)) as Value;
}
