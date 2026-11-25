// A stop invalidates both active callbacks and a start waiting for teardown.
export class StartStop {
  #generation = 0;
  #startTask: Promise<void> | undefined;
  #stopTask: Promise<void> | undefined;

  get generation(): number {
    return this.#generation;
  }

  start(begin: (generation: number) => Promise<void>, retryOnFailure = false): Promise<void> {
    if (this.#startTask !== undefined) {
      return this.#startTask;
    }

    const generation = ++this.#generation;
    const afterStop = this.#stopTask;
    const { task, invoke } = reserve(async () => {
      try {
        if (afterStop !== undefined) {
          await afterStop;
        }
        if (generation === this.#generation) {
          await begin(generation);
        }
      } catch (error) {
        if (retryOnFailure && generation === this.#generation) {
          this.#startTask = undefined;
        }
        throw error;
      }
    });
    this.#startTask = task;
    invoke();
    return task;
  }

  stop(end: () => Promise<void>): Promise<void> {
    this.#startTask = undefined;
    ++this.#generation;
    if (this.#stopTask === undefined) {
      const { task, invoke } = reserve(async () => {
        try {
          await end();
        } finally {
          this.#stopTask = undefined;
        }
      });
      this.#stopTask = task;
      invoke();
      return task;
    }
    return this.#stopTask;
  }
}

// StartStop needs to publish the promise before invoking synchronous hooks.
function reserve(run: () => Promise<void>) {
  let invoke!: () => void;
  const task = new Promise<void>((resolve, reject) => {
    invoke = () => void run().then(resolve, reject);
  });
  return { task, invoke };
}
