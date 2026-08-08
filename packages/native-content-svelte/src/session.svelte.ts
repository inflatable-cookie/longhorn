import {
  isNewerNativeContentSnapshot,
  type ClientRect,
  type DesiredUpdate,
  type DesiredVisibility,
  type FocusIntent,
  type InputRoutingMode,
  type NativeContentClient,
  type NativeContentConnection,
  type NativeContentProtocolRejection,
  type NativeContentSnapshot,
  type ScaleFactor,
} from "@inflatable-cookie/longhorn-native-content";

export type NativeContentSessionClient = Pick<NativeContentClient, "connect">;

export interface NativeContentResizeObserver {
  observe(target: Element): void;
  disconnect(): void;
}

export type NativeContentResizeObserverFactory = (
  callback: () => void,
) => NativeContentResizeObserver;

export type NativeContentSessionStatus =
  | { readonly kind: "idle" }
  | { readonly kind: "connecting" }
  | { readonly kind: "ready" }
  | { readonly kind: "updating" }
  | {
      readonly kind: "rejected";
      readonly rejection: NativeContentProtocolRejection;
    }
  | { readonly kind: "failed"; readonly error: unknown };

export interface NativeContentSessionOptions {
  readonly client: NativeContentSessionClient;
  readonly scale: ScaleFactor;
  readonly visibility: DesiredVisibility;
  readonly focus: FocusIntent;
  readonly inputRouting: InputRoutingMode;
  readonly resizeObserverFactory?: NativeContentResizeObserverFactory;
}

/** One checked native-content connection and viewport policy per Svelte mount. */
export class NativeContentSession {
  readonly #client: NativeContentSessionClient;
  readonly #resizeObserverFactory: NativeContentResizeObserverFactory;
  #status = $state.raw<NativeContentSessionStatus>({ kind: "idle" });
  #snapshot = $state.raw<NativeContentSnapshot | undefined>(undefined);
  #viewport = $state.raw<ClientRect | undefined>(undefined);
  #scale = $state<ScaleFactor>(1000);
  #visibility = $state.raw<DesiredVisibility>({ state: "visible" });
  #focus = $state<FocusIntent>("unchanged");
  #inputRouting = $state<InputRoutingMode>("disabled");
  #viewportElement: HTMLElement | undefined;
  #viewportBinding = 0;
  #observer: NativeContentResizeObserver | undefined;
  #connection: NativeContentConnection | undefined;
  #startTask: Promise<void> | undefined;
  #stopTask: Promise<void> | undefined;
  #pumpTask: Promise<void> | undefined;
  #pumpRequested = false;
  #lifecycle = 0;
  #active = false;

  constructor(options: NativeContentSessionOptions) {
    assertScale(options.scale);
    this.#client = options.client;
    this.#scale = options.scale;
    this.#visibility = copyVisibility(options.visibility);
    this.#focus = options.focus;
    this.#inputRouting = options.inputRouting;
    this.#resizeObserverFactory =
      options.resizeObserverFactory ?? defaultResizeObserverFactory;
  }

  get status(): NativeContentSessionStatus {
    return this.#status;
  }

  get snapshot(): NativeContentSnapshot | undefined {
    return this.#snapshot;
  }

  get viewport(): ClientRect | undefined {
    return this.#viewport;
  }

  get scale(): ScaleFactor {
    return this.#scale;
  }

  get visibility(): DesiredVisibility {
    return copyVisibility(this.#visibility);
  }

  get focus(): FocusIntent {
    return this.#focus;
  }

  get inputRouting(): InputRoutingMode {
    return this.#inputRouting;
  }

  start(): Promise<void> {
    if (this.#startTask !== undefined) return this.#startTask;
    if (this.#active) return Promise.resolve();

    const task =
      this.#stopTask === undefined
        ? this.#begin()
        : this.#stopTask.then(() => this.#begin());
    this.#startTask = task;
    void task.then(
      () => {
        if (this.#startTask === task) this.#startTask = undefined;
      },
      () => {
        if (this.#startTask === task) this.#startTask = undefined;
      },
    );
    return task;
  }

  stop(): Promise<void> {
    if (this.#stopTask !== undefined) return this.#stopTask;
    if (!this.#active && this.#connection === undefined) {
      this.#status = { kind: "idle" };
      this.#snapshot = undefined;
      return Promise.resolve();
    }

    ++this.#lifecycle;
    this.#active = false;
    this.#pumpRequested = false;
    this.#pumpTask = undefined;
    this.#disconnectObserver();
    const connection = this.#connection;
    this.#connection = undefined;
    this.#snapshot = undefined;
    this.#status = { kind: "idle" };

    const task = (async () => {
      try {
        await connection?.dispose();
      } catch (error) {
        this.#status = { kind: "failed", error };
        throw error;
      }
    })();
    this.#stopTask = task;
    void task.then(
      () => {
        if (this.#stopTask === task) this.#stopTask = undefined;
      },
      () => {
        if (this.#stopTask === task) this.#stopTask = undefined;
      },
    );
    return task;
  }

  bindViewport(element: HTMLElement): () => void {
    const binding = ++this.#viewportBinding;
    this.#disconnectObserver();
    this.#viewportElement = element;
    this.refreshViewport();
    if (this.#active) this.#installObserver();

    return () => {
      if (binding !== this.#viewportBinding) return;
      ++this.#viewportBinding;
      this.#disconnectObserver();
      this.#viewportElement = undefined;
      this.#viewport = undefined;
      this.#pumpRequested = false;
    };
  }

  refreshViewport(): ClientRect {
    const element = this.#viewportElement;
    if (element === undefined) {
      throw new NativeContentViewportUnboundError();
    }
    const viewport = measureViewport(element);
    if (!equalRect(viewport, this.#viewport)) {
      this.#viewport = viewport;
      this.#requestPump();
    }
    return viewport;
  }

  setScale(scale: ScaleFactor): void {
    assertScale(scale);
    if (scale === this.#scale) return;
    this.#scale = scale;
    this.#requestPump();
  }

  setVisibilityPolicy(visibility: DesiredVisibility): void {
    if (equalVisibility(visibility, this.#visibility)) return;
    this.#visibility = copyVisibility(visibility);
    this.#requestPump();
  }

  setFocusIntent(focus: FocusIntent): void {
    if (focus === this.#focus) return;
    this.#focus = focus;
    this.#requestPump();
  }

  setInputRouting(inputRouting: InputRoutingMode): void {
    if (inputRouting === this.#inputRouting) return;
    this.#inputRouting = inputRouting;
    this.#requestPump();
  }

  async whenSettled(): Promise<void> {
    while (this.#pumpTask !== undefined) {
      await this.#pumpTask;
    }
  }

  async #begin(): Promise<void> {
    const lifecycle = ++this.#lifecycle;
    this.#active = true;
    this.#status = { kind: "connecting" };

    try {
      this.#installObserver();
      let connection: NativeContentConnection | undefined;
      let earlySnapshot: NativeContentSnapshot | undefined;
      let earlyFailure: unknown;
      connection = this.#client.connect(
        (snapshot) => {
          if (connection === undefined) {
            earlySnapshot = snapshot;
          } else if (this.#isCurrent(lifecycle, connection)) {
            this.#acceptSnapshot(snapshot);
          }
        },
        (failure) => {
          if (connection === undefined) {
            earlyFailure = failure.error;
          } else if (this.#isCurrent(lifecycle, connection)) {
            this.#status = { kind: "failed", error: failure.error };
          }
        },
      );
      this.#connection = connection;
      if (earlyFailure !== undefined) {
        this.#status = { kind: "failed", error: earlyFailure };
      }
      if (earlySnapshot !== undefined) this.#acceptSnapshot(earlySnapshot);
      const snapshot = await connection.ready;
      if (!this.#isCurrent(lifecycle, connection)) return;
      this.#acceptSnapshot(connection.current() ?? snapshot);
    } catch (error) {
      if (lifecycle === this.#lifecycle) {
        this.#active = false;
        this.#disconnectObserver();
        this.#status = { kind: "failed", error };
      }
      const connection = this.#connection;
      this.#connection = undefined;
      await connection?.dispose().catch(() => undefined);
      throw error;
    }
  }

  #acceptSnapshot(snapshot: NativeContentSnapshot): void {
    const current = this.#snapshot;
    const sameCursor =
      current?.cursor.authority_epoch === snapshot.cursor.authority_epoch &&
      current.cursor.client_epoch === snapshot.cursor.client_epoch &&
      current.cursor.desired_revision === snapshot.cursor.desired_revision &&
      current.cursor.observed_revision === snapshot.cursor.observed_revision;
    if (
      current !== undefined &&
      !sameCursor &&
      !isNewerNativeContentSnapshot(snapshot, current)
    ) {
      return;
    }
    this.#snapshot = snapshot;
    if (this.#status.kind !== "updating") this.#status = { kind: "ready" };
    this.#requestPump();
  }

  #requestPump(): void {
    if (!this.#active || this.#connection === undefined) return;
    this.#pumpRequested = true;
    if (this.#pumpTask !== undefined) return;

    const lifecycle = this.#lifecycle;
    const connection = this.#connection;
    const task = this.#drain(lifecycle, connection).catch((error) => {
      if (this.#isCurrent(lifecycle, connection)) {
        this.#pumpRequested = false;
        this.#status = { kind: "failed", error };
      }
    });
    this.#pumpTask = task;
    void task.then(() => {
      if (this.#pumpTask === task) this.#pumpTask = undefined;
      if (this.#pumpRequested && this.#isCurrent(lifecycle, connection)) {
        this.#requestPump();
      }
    });
  }

  async #drain(
    lifecycle: number,
    connection: NativeContentConnection,
  ): Promise<void> {
    while (this.#pumpRequested && this.#isCurrent(lifecycle, connection)) {
      this.#pumpRequested = false;
      await this.#updateOnce(lifecycle, connection);
    }
  }

  async #updateOnce(
    lifecycle: number,
    connection: NativeContentConnection,
  ): Promise<void> {
    const snapshot = connection.current() ?? this.#snapshot;
    const viewport = this.#viewport;
    if (snapshot === undefined || viewport === undefined) return;

    const update = this.#desiredUpdate(snapshot, viewport);
    if (equalDesired(update, snapshot.desired)) {
      this.#status = { kind: "ready" };
      return;
    }

    const generation = snapshot.cursor.attach_generation;
    this.#status = { kind: "updating" };
    const result = await connection.updateDesired(update);
    if (!this.#isCurrent(lifecycle, connection)) return;

    const current = connection.current();
    if (
      current !== undefined &&
      current.cursor.attach_generation !== generation
    ) {
      this.#acceptSnapshot(current);
      return;
    }
    if (result.status === "rejected") {
      this.#status = { kind: "rejected", rejection: result.rejection };
      return;
    }

    const admitted = current ?? result.snapshot;
    if (admitted.cursor.attach_generation === generation) {
      this.#acceptSnapshot(admitted);
    }
  }

  #desiredUpdate(
    snapshot: NativeContentSnapshot,
    viewport: ClientRect,
  ): DesiredUpdate {
    return {
      generation: snapshot.cursor.attach_generation,
      host_window_id: snapshot.desired.host_window_id,
      viewport,
      scale: this.#scale,
      rounding: snapshot.desired.rounding,
      presence: snapshot.desired.presence,
      visibility: copyVisibility(this.#visibility),
      focus: this.#focus,
      input_routing: this.#inputRouting,
    };
  }

  #installObserver(): void {
    if (
      !this.#active ||
      this.#viewportElement === undefined ||
      this.#observer !== undefined
    ) {
      return;
    }
    const observer = this.#resizeObserverFactory(() => {
      try {
        this.refreshViewport();
      } catch (error) {
        if (this.#active) this.#status = { kind: "failed", error };
      }
    });
    observer.observe(this.#viewportElement);
    this.#observer = observer;
  }

  #disconnectObserver(): void {
    this.#observer?.disconnect();
    this.#observer = undefined;
  }

  #isCurrent(
    lifecycle: number,
    connection: NativeContentConnection,
  ): boolean {
    return (
      this.#active &&
      lifecycle === this.#lifecycle &&
      connection === this.#connection
    );
  }
}

function defaultResizeObserverFactory(
  callback: () => void,
): NativeContentResizeObserver {
  if (typeof globalThis.ResizeObserver !== "function") {
    throw new NativeContentResizeObserverUnavailableError();
  }
  return new globalThis.ResizeObserver(() => callback());
}

function measureViewport(element: HTMLElement): ClientRect {
  const rect = element.getBoundingClientRect();
  for (const [name, value] of [
    ["left", rect.left],
    ["top", rect.top],
    ["width", rect.width],
    ["height", rect.height],
  ] as const) {
    if (!Number.isFinite(value)) {
      throw new NativeContentViewportMeasurementError(name, value);
    }
  }
  if (rect.width < 0 || rect.height < 0) {
    throw new NativeContentViewportMeasurementError(
      "extent",
      Math.min(rect.width, rect.height),
    );
  }
  return {
    origin: { x: rect.left, y: rect.top },
    size: { width: rect.width, height: rect.height },
  };
}

function assertScale(scale: ScaleFactor): void {
  if (
    !Number.isSafeInteger(scale) ||
    scale < 1 ||
    scale > 0xffff_ffff
  ) {
    throw new RangeError(
      "native-content scale must be integer thousandths in the u32 range",
    );
  }
}

function copyVisibility(visibility: DesiredVisibility): DesiredVisibility {
  return visibility.state === "visible"
    ? { state: "visible" }
    : { state: "hidden", reason: visibility.reason };
}

function equalVisibility(
  left: DesiredVisibility,
  right: DesiredVisibility,
): boolean {
  return (
    left.state === right.state &&
    (left.state === "visible" ||
      (right.state === "hidden" && left.reason === right.reason))
  );
}

function equalRect(left: ClientRect, right: ClientRect | undefined): boolean {
  return (
    right !== undefined &&
    left.origin.x === right.origin.x &&
    left.origin.y === right.origin.y &&
    left.size.width === right.size.width &&
    left.size.height === right.size.height
  );
}

function equalDesired(
  update: DesiredUpdate,
  desired: NativeContentSnapshot["desired"],
): boolean {
  return (
    update.generation === desired.generation &&
    update.host_window_id === desired.host_window_id &&
    equalRect(update.viewport, desired.viewport) &&
    update.scale === desired.scale &&
    update.rounding === desired.rounding &&
    update.presence === desired.presence &&
    equalVisibility(update.visibility, desired.visibility) &&
    update.focus === desired.focus &&
    update.input_routing === desired.input_routing
  );
}

export class NativeContentViewportUnboundError extends Error {
  constructor() {
    super("native-content viewport element is not bound");
    this.name = "NativeContentViewportUnboundError";
  }
}

export class NativeContentViewportMeasurementError extends Error {
  constructor(
    readonly field: string,
    readonly value: number,
  ) {
    super(`native-content viewport ${field} is invalid: ${value}`);
    this.name = "NativeContentViewportMeasurementError";
  }
}

export class NativeContentResizeObserverUnavailableError extends Error {
  constructor() {
    super("ResizeObserver is required for native-content viewport binding");
    this.name = "NativeContentResizeObserverUnavailableError";
  }
}
