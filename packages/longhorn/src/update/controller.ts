import { UpdateClient } from "./client.ts";
import {
  UPDATE_PROTOCOL_VERSION,
  type Channel,
  type DeferralCause,
  type UpdateAvailabilityProjection,
  type UpdateProgressProjection,
  type UpdateRejectionCode,
  type UpdateSnapshot,
} from "./generated/protocol.ts";
import type { UpdatePort, UpdateUnlisten } from "./ports.ts";

export type UpdateControllerStatus =
  | { readonly kind: "idle" }
  | { readonly kind: "loading" }
  | { readonly kind: "ready" }
  | { readonly kind: "failed"; readonly error: unknown };

export interface UpdateControllerOptions {
  readonly port: UpdatePort;
}

/**
 * Holds update state for a surface and keeps the three outcomes apart.
 *
 * A command can end three ways, and collapsing any two of them produces a
 * wrong message:
 *
 * - **Committed.** State moved. Read the snapshot.
 * - **Committed with a deferral.** The install did not happen and that is not
 *   a failure — Card 154 step 6. The gate refused because the user has work in
 *   flight, and `deferral` says so. A surface that reports this as an error
 *   tells a customer their update is broken when nothing is.
 * - **Rejected.** The authority refused. `lastRejection` carries the code, and
 *   this is the only one of the three that is a fault.
 *
 * Transport failures are the fourth thing, and land in `status`.
 */
export class UpdateController {
  readonly #client: UpdateClient;
  readonly #observers = new Set<() => void>();
  #status: UpdateControllerStatus = { kind: "idle" };
  #snapshot?: UpdateSnapshot;
  #lastRejection?: UpdateRejectionCode;
  #started = false;
  #lifecycle = 0;
  #pending = false;
  #unlisten: UpdateUnlisten[] = [];

  constructor(options: UpdateControllerOptions) {
    this.#client = new UpdateClient(options.port);
  }

  get status(): UpdateControllerStatus { return this.#status; }
  get snapshot(): UpdateSnapshot | undefined { return this.#snapshot; }
  get availability(): UpdateAvailabilityProjection | undefined { return this.#snapshot?.availability; }
  get progress(): UpdateProgressProjection | undefined { return this.#snapshot?.progress; }
  get channel(): Channel | undefined { return this.#snapshot?.channel; }
  get installedVersion(): string | undefined { return this.#snapshot?.installedVersion; }
  get pending(): boolean { return this.#pending; }

  /**
   * Why the last install did not proceed, when it did not.
   *
   * Present alongside a *committed* outcome. Card 154 step 6: a refused
   * restart shows its reason and is not a failure.
   */
  get deferral(): { readonly version: string; readonly cause: DeferralCause } | undefined {
    return this.#snapshot?.deferral ?? undefined;
  }

  /** The last refusal, cleared by the next command that commits. */
  get lastRejection(): UpdateRejectionCode | undefined { return this.#lastRejection; }

  /**
   * Whether this install sits ahead of the channel it follows.
   *
   * Card 154 step 5 calls this the single most likely support question the
   * feature generates: an install on `1.3.0-nightly.4` that selects production
   * receives nothing until `1.3.0` ships, which is correct and reads as a
   * broken updater unless the surface says so. Exposed as its own read so a
   * surface cannot render it as "no update available" by omission.
   */
  get aheadOfChannel(): { readonly installed: string; readonly channel: string } | undefined {
    const availability = this.#snapshot?.availability;
    if (availability?.state !== "aheadOfChannel") return undefined;
    return { installed: availability.installed, channel: availability.channel };
  }

  observe(observer: () => void): () => void {
    this.#observers.add(observer);
    return () => this.#observers.delete(observer);
  }

  async start(): Promise<void> {
    if (this.#started) return;
    this.#started = true;
    const lifecycle = ++this.#lifecycle;
    this.#setStatus({ kind: "loading" });
    try {
      const unlisten = await this.#client.listen(() => void this.refresh());
      // The listen resolved after a stop, or after a restart replaced this
      // lifetime. Contract 013: an adapter that outlives its owner keeps
      // writing into state nobody is reading.
      if (!this.#started || lifecycle !== this.#lifecycle) {
        await unlisten();
        return;
      }
      this.#unlisten.push(unlisten);
      await this.refresh();
    } catch (error) {
      if (this.#started && lifecycle === this.#lifecycle) this.#setStatus({ kind: "failed", error });
    }
  }

  async stop(): Promise<void> {
    this.#started = false;
    this.#lifecycle += 1;
    const unlisten = this.#unlisten;
    this.#unlisten = [];
    for (const stop of unlisten) await stop();
    this.#setStatus({ kind: "idle" });
  }

  async refresh(): Promise<void> {
    const lifecycle = this.#lifecycle;
    try {
      const snapshot = await this.#client.snapshot();
      if (lifecycle !== this.#lifecycle) return;
      this.#snapshot = snapshot;
      this.#setStatus({ kind: "ready" });
    } catch (error) {
      if (lifecycle === this.#lifecycle) this.#setStatus({ kind: "failed", error });
    }
  }

  /** Asks the source for the channel's current manifest. */
  async check(): Promise<void> {
    await this.#command((client, epoch) =>
      client.check({ protocolVersion: UPDATE_PROTOCOL_VERSION, authorityEpoch: epoch }),
    );
  }

  /** Follows a different channel from now on. */
  async selectChannel(channel: Channel): Promise<void> {
    await this.#command((client, epoch) =>
      client.selectChannel({ protocolVersion: UPDATE_PROTOCOL_VERSION, authorityEpoch: epoch, channel }),
    );
  }

  /** Declines a version for now. */
  async defer(version: string, cause: DeferralCause): Promise<void> {
    await this.#command((client, epoch) =>
      client.defer({ protocolVersion: UPDATE_PROTOCOL_VERSION, authorityEpoch: epoch, version, cause }),
    );
  }

  /** Fetches, verifies, gates and installs. */
  async install(version: string): Promise<void> {
    await this.#command((client, epoch) =>
      client.install({ protocolVersion: UPDATE_PROTOCOL_VERSION, authorityEpoch: epoch, version }),
    );
  }

  async #command(
    run: (client: UpdateClient, epoch: number) => Promise<import("./generated/protocol.ts").UpdateOutcomeProjection>,
  ): Promise<void> {
    const epoch = this.#snapshot?.authorityEpoch;
    if (epoch === undefined) {
      // No snapshot means no epoch to send, and inventing one would have the
      // authority refuse it as stale. Say what is actually wrong.
      this.#setStatus({ kind: "failed", error: new Error("update state has not been read yet") });
      return;
    }
    const lifecycle = this.#lifecycle;
    this.#pending = true;
    this.#notify();
    try {
      const outcome = await run(this.#client, epoch);
      if (lifecycle !== this.#lifecycle) return;
      this.#snapshot = outcome.snapshot;
      this.#lastRejection = outcome.status === "rejected" ? outcome.code : undefined;
      this.#setStatus({ kind: "ready" });
    } catch (error) {
      if (lifecycle === this.#lifecycle) this.#setStatus({ kind: "failed", error });
    } finally {
      if (lifecycle === this.#lifecycle) {
        this.#pending = false;
        this.#notify();
      }
    }
  }

  #setStatus(status: UpdateControllerStatus): void {
    this.#status = status;
    this.#notify();
  }

  #notify(): void {
    for (const observer of this.#observers) observer();
  }
}
