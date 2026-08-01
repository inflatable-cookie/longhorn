export function clone<Value>(value: Value): Value {
  return JSON.parse(JSON.stringify(value)) as Value;
}

export function operationTrace(results: readonly { readonly snapshot: { readonly catalogueRevision: number; readonly active: readonly { readonly operationId: string; readonly state: string }[]; readonly recent: readonly { readonly operationId: string; readonly state: string }[] } }[]) {
  return results.map(({ snapshot }) => ({
    revision: snapshot.catalogueRevision,
    states: [...snapshot.active, ...snapshot.recent]
      .map(({ operationId, state }) => ({ operationId, state }))
      .sort((left, right) => left.operationId.localeCompare(right.operationId)),
  }));
}

export function notificationTrace(results: readonly { readonly snapshot: { readonly ledgerRevision: number; readonly unseenCount: number; readonly page: { readonly records: readonly { readonly notificationId: string; readonly readState: string }[] } } }[]) {
  return results.map(({ snapshot }) => ({
    records: snapshot.page.records
      .map(({ notificationId, readState }) => ({ notificationId, readState }))
      .sort((left, right) => left.notificationId.localeCompare(right.notificationId)),
    revision: snapshot.ledgerRevision,
    unseenCount: snapshot.unseenCount,
  }));
}

export function equal(left: unknown, right: unknown): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}
