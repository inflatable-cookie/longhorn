import {
  array,
  identity,
  known,
  optionalOpaque,
  record,
} from "./primitives.ts";

export function policy(
  value: unknown,
  maximum: number,
  effects: readonly string[],
): void {
  const valueRecord = record(value);
  identity(valueRecord.sourceId);
  known(valueRecord.effect, effects);
  optionalOpaque(valueRecord.constraints, maximum);
}

export function recovery(
  value: unknown,
  maximum: number,
  codes: readonly string[],
): void {
  const valueRecord = record(value);
  known(valueRecord.code, codes);
  optionalOpaque(valueRecord.diagnostic, maximum);
}

export function activation(
  value: unknown,
  states: readonly string[],
): void {
  array(value).forEach((value) => {
    const requirement = record(value);
    identity(requirement.targetId);
    known(requirement.state, states);
  });
}

export function rejection(
  value: unknown,
  maximum: number,
  codes: readonly string[],
): void {
  const valueRecord = record(value);
  known(valueRecord.code, codes);
  optionalOpaque(valueRecord.diagnostic, maximum);
}
