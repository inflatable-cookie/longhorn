export type RevisionOf<Document> = (document: Document) => number;
export type OptimisticProjector<Document> = (
  authoritative: Document,
) => Document;

interface PendingProjection<Document> {
  readonly requestId: string;
  readonly project: OptimisticProjector<Document>;
}

export class OptimisticProjectionState<Document> {
  readonly #revisionOf: RevisionOf<Document>;
  #authoritative = $state.raw<Document | undefined>(undefined);
  #pending = $state.raw<readonly PendingProjection<Document>[]>([]);

  constructor(revisionOf: RevisionOf<Document>) {
    this.#revisionOf = revisionOf;
  }

  get authoritative(): Document | undefined {
    return this.#authoritative;
  }

  get projected(): Document | undefined {
    let document = this.#authoritative;
    if (document === undefined) {
      return undefined;
    }
    for (const pending of this.#pending) {
      document = pending.project(document);
    }
    return document;
  }

  get pendingRequestIds(): readonly string[] {
    return this.#pending.map(({ requestId }) => requestId);
  }

  accept(document: Document): boolean {
    const current = this.#authoritative;
    if (
      current !== undefined &&
      this.#revisionOf(document) < this.#revisionOf(current)
    ) {
      return false;
    }
    this.#authoritative = document;
    return true;
  }

  begin(
    requestId: string,
    project: OptimisticProjector<Document>,
  ): void {
    if (this.#pending.some((pending) => pending.requestId === requestId)) {
      throw new DuplicateOptimisticRequestError(requestId);
    }
    const current = this.projected;
    if (current === undefined) {
      throw new MissingAuthoritativeDocumentError();
    }
    project(current);
    this.#pending = [...this.#pending, { requestId, project }];
  }

  settle(requestId: string, document: Document): boolean {
    const accepted = this.accept(document);
    this.cancel(requestId);
    return accepted;
  }

  cancel(requestId: string): void {
    this.#pending = this.#pending.filter(
      (pending) => pending.requestId !== requestId,
    );
  }

  clear(): void {
    this.#authoritative = undefined;
    this.#pending = [];
  }
}

export class DuplicateOptimisticRequestError extends Error {
  constructor(requestId: string) {
    super(`optimistic request is already pending: ${requestId}`);
    this.name = "DuplicateOptimisticRequestError";
  }
}

export class MissingAuthoritativeDocumentError extends Error {
  constructor() {
    super("optimistic projection requires an authoritative document");
    this.name = "MissingAuthoritativeDocumentError";
  }
}
