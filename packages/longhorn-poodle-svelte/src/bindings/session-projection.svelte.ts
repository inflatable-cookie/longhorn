import { StartStop } from "./start-stop.ts";

interface ObservableController {
  observe(listener: () => void): () => void;
  start(): Promise<void>;
  stop(): Promise<void>;
}

export class SessionProjection<Projection> {
  readonly #controller: ObservableController;
  readonly #unobserve: () => void;
  readonly #lifetime = new StartStop();
  #current: Projection;
  #retryFailure = $state.raw<{ readonly kind: "failed"; readonly error: unknown }>();

  constructor(controller: ObservableController, project: () => Projection) {
    this.#controller = controller;
    this.#current = $state.raw(project());
    this.#unobserve = controller.observe(() => {
      this.#current = project();
    });
  }

  get current(): Projection {
    return this.#current;
  }

  get retryFailure() {
    return this.#retryFailure;
  }

  start(): Promise<void> {
    return this.#lifetime.start(() => this.#controller.start(), true);
  }

  stop(): Promise<void> {
    this.#retryFailure = undefined;
    return this.#lifetime.stop(() => this.#controller.stop());
  }

  async retry(previousError: unknown): Promise<void> {
    this.#retryFailure = undefined;
    const teardown = this.#lifetime.stop(async () => {
      try {
        await this.#controller.stop();
      } catch (error) {
        // A checked subscription may repeat its startup failure at teardown.
        if (!Object.is(error, previousError)) {
          throw error;
        }
      }
    });
    const start = this.start();
    const generation = this.#lifetime.generation;
    try {
      await Promise.all([teardown, start]);
    } catch (error) {
      if (generation === this.#lifetime.generation) {
        this.#retryFailure = { kind: "failed", error };
      }
      throw error;
    }
  }

  async dispose(): Promise<void> {
    this.#unobserve();
    await this.stop();
  }
}
