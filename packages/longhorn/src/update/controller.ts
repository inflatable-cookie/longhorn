import { ControllerLifecycle } from "../core/controller-lifecycle.ts";
import { UpdateClient } from "./client.ts";
import { UPDATE_PROTOCOL_VERSION } from "./generated/protocol.ts";
import type {
  Channel,
  DeferralCause,
  UpdateAvailabilityProjection,
  UpdateOutcomeProjection,
  UpdateProgressEvent,
  UpdateProgressProjection,
  UpdateRejectionCode,
  UpdateSnapshot,
  UpdateStagedArtifactProjection,
} from "./generated/protocol.ts";
import type { UpdatePort } from "./ports.ts";

export type UpdateControllerStatus =
  | { readonly kind: "idle" }
  | { readonly kind: "loading" }
  | { readonly kind: "ready" }
  | { readonly kind: "failed"; readonly error: unknown };

export interface UpdateControllerOptions {
  readonly port: UpdatePort;
}

export type UpdateCommandOutcome =
  | { readonly status: "committed"; readonly snapshot: UpdateSnapshot }
  | {
      readonly status: "committedWithDeferral";
      readonly snapshot: UpdateSnapshot;
      readonly deferral: NonNullable<UpdateSnapshot["deferral"]>;
    }
  | {
      readonly status: "rejected";
      readonly snapshot: UpdateSnapshot;
      readonly code: UpdateRejectionCode;
      readonly detail: string | null;
    }
  | { readonly status: "failed"; readonly error: unknown };

/**
 * What an update indicator should do.
 *
 * A predicate rather than a rendering, and Longhorn's rather than a surface's:
 * deriving it means reading five availability states against five progress
 * states and a deferral, and two surfaces deriving it separately would drift.
 *
 * - `hidden` — collapse to nothing. There is nothing the operator can do here.
 * - `quiet` — present, not insistent. Work is in flight, or they have already
 *   been asked and postponed.
 * - `attention` — something is waiting for a decision.
 */
export type UpdatePresence = "hidden" | "quiet" | "attention";

/**
 * Holds update state for a surface and keeps the three outcomes apart.
 *
 * Commands resolve with an outcome, including transport failures, and also
 * update `status` for callers that observe the controller instead:
 *
 * - **Committed.** State moved. Read the snapshot.
 * - **Committed with a deferral.** The gate postponed the install. Read
 *   `deferral` for the reason.
 * - **Rejected.** The authority refused. `lastRejection` carries the code.
 *
 * - **Failed.** The transport or local precondition failed. The error is in
 *   the outcome and in `status`.
 */
export class UpdateController {
  readonly #client: UpdateClient;
  readonly #observers = new Set<() => void>();
  #status: UpdateControllerStatus = { kind: "idle" };
  #snapshot?: UpdateSnapshot;
  #lastRejection: UpdateRejectionCode | undefined;
  #lastRejectionDetail: string | null = null;
  readonly #lifecycle = new ControllerLifecycle();
  #commandSequence = 0;
  #refreshRevision = 0;
  #pending = false;
  #legacyProgress: ((event: UpdateProgressEvent) => void) | undefined;
  /**
   * The most recent live progress report.
   *
   * Held apart from the snapshot because it is *out of band*: a transfer
   * publishes bytes while the controller's retained state still says what it
   * said before the transfer began. It is cleared when the command that
   * started the transfer settles, so a stale report cannot outlive it.
   */
  #liveProgress: UpdateProgressProjection | undefined;

  constructor(options: UpdateControllerOptions) {
    this.#client = new UpdateClient(options.port);
  }

  get status(): UpdateControllerStatus {
    return this.#lifecycle.eventFailure ?? this.#status;
  }
  get snapshot(): UpdateSnapshot | undefined {
    return this.#snapshot;
  }
  get availability(): UpdateAvailabilityProjection | undefined {
    return this.#snapshot?.availability;
  }
  /**
   * What the transfer is doing right now.
   *
   * A live report wins over the snapshot, because the snapshot is only as
   * fresh as the last commit and the transfer deliberately does not hold the
   * authority while it runs.
   */
  get progress(): UpdateProgressProjection | undefined {
    return this.#liveProgress ?? this.#snapshot?.progress;
  }
  /** The retained verified artifact, when one is staged. */
  get staged(): UpdateStagedArtifactProjection | undefined {
    return this.#snapshot?.staged ?? undefined;
  }
  get channel(): Channel | undefined {
    return this.#snapshot?.channel;
  }
  get installedVersion(): string | undefined {
    return this.#snapshot?.installedVersion;
  }
  get pending(): boolean {
    return this.#pending;
  }

  /**
   * Why the last install did not proceed, when it did not.
   *
   * The reason a committed command postponed installation.
   */
  get deferral(): { readonly version: string; readonly cause: DeferralCause } | undefined {
    return this.#snapshot?.deferral ?? undefined;
  }

  /** The latest refusal, cleared as soon as the next command starts. */
  get lastRejection(): UpdateRejectionCode | undefined {
    return this.#lastRejection;
  }

  /** Installer detail from the latest rejection, when supplied. */
  get lastRejectionDetail(): string | null {
    return this.#lastRejectionDetail;
  }

  /**
   * Whether this install sits ahead of the channel it follows.
   *
   * A nightly install following production waits for the channel to catch up.
   * The surface can explain this state using the installed and channel versions.
   */
  get aheadOfChannel(): { readonly installed: string; readonly channel: string } | undefined {
    const availability = this.#snapshot?.availability;
    if (availability?.state !== "aheadOfChannel") {
      return undefined;
    }
    return { installed: availability.installed, channel: availability.channel };
  }

  /**
   * Whether an update indicator should show, and how loudly.
   *
   * Three availability states are deliberately `hidden`, because none of them
   * is an install this application can perform:
   *
   * - `withheldByRollout` — a version exists and this install is not in the
   *   stage yet. Showing it would offer something that cannot be taken.
   * - `aheadOfChannel` — nothing to install. It needs explaining, and the
   *   place to explain it is the settings panel, not a notification.
   * - `managedElsewhere` — Homebrew or the App Store owns this install. There
   *   is a version and the operator can act, but not through the
   *   download-install-restart loop an indicator implies. Settings tells them
   *   where to get it.
   *
   * A postponed offer stays `quiet` rather than disappearing. They said "not
   * now", not "never", and an indicator that vanishes gives them no way back.
   */
  get presence(): UpdatePresence {
    const snapshot = this.#snapshot;
    if (snapshot === undefined) {
      return "hidden";
    }

    switch (this.progress?.state) {
      case "downloading":
      case "verifying":
      case "installing":
        return "quiet";
      case "readyToInstall":
        return "attention";
      case "idle":
      case undefined:
        break;
    }

    if (snapshot.availability.state !== "offer") {
      return "hidden";
    }
    // Postponed by the operator, not by the gate. A gate deferral is transient
    // and the offer is still the answer; `userPostponed` is a decision.
    return snapshot.deferral?.cause.cause === "userPostponed" ? "quiet" : "attention";
  }

  observe(observer: () => void): () => void {
    this.#observers.add(observer);
    return () => this.#observers.delete(observer);
  }

  start(): Promise<void> {
    return this.#lifecycle.start({
      loading: () => this.#setStatus({ kind: "loading" }),
      connect: async ({ isCurrent, own, reportFailure }) => {
        const unlisten = await this.#client.listen(() => {
          if (isCurrent()) {
            void this.refresh();
          }
        }, reportFailure);
        await own(unlisten);
        if (!isCurrent()) {
          return;
        }
        const progress = await this.#client.listenProgress((event) => {
          if (isCurrent()) {
            this.#legacyProgress?.(event);
          }
        }, reportFailure);
        await own(progress);
        if (isCurrent()) {
          await this.refresh();
        }
      },
      failed: (error) => this.#setStatus({ kind: "failed", error }),
    });
  }

  stop(): Promise<void> {
    return this.#lifecycle.stop(() => {
      this.#pending = false;
      this.#legacyProgress = undefined;
      this.#liveProgress = undefined;
      this.#setStatus({ kind: "idle" });
    }, "reject");
  }

  async refresh(): Promise<void> {
    if (!this.#lifecycle.active) {
      return;
    }
    const lifecycle = this.#lifecycle.revision;
    const revision = ++this.#refreshRevision;
    const isCurrent = () =>
      this.#lifecycle.isCurrent(lifecycle) && revision === this.#refreshRevision;
    try {
      const snapshot = await this.#client.snapshot();
      if (!isCurrent()) {
        return;
      }
      if (snapshot.authorityEpoch !== this.#snapshot?.authorityEpoch) {
        this.#liveProgress = undefined;
      }
      this.#snapshot = snapshot;
      this.#setStatus({ kind: "ready" });
    } catch (error) {
      if (isCurrent()) {
        this.#setStatus({ kind: "failed", error });
      }
    }
  }

  /** Asks the source for the channel's current manifest. */
  async check(): Promise<UpdateCommandOutcome> {
    return this.#command((client, { authorityEpoch }) =>
      client.check({ protocolVersion: UPDATE_PROTOCOL_VERSION, authorityEpoch }),
    );
  }

  /** Follows a different channel from now on. */
  async selectChannel(channel: Channel): Promise<UpdateCommandOutcome> {
    return this.#command((client, { authorityEpoch }) =>
      client.selectChannel({
        protocolVersion: UPDATE_PROTOCOL_VERSION,
        authorityEpoch,
        channel,
      }),
    );
  }

  /** Declines a version for now. */
  async defer(version: string, cause: DeferralCause): Promise<UpdateCommandOutcome> {
    return this.#command(
      (client, { authorityEpoch }) =>
        client.defer({
          protocolVersion: UPDATE_PROTOCOL_VERSION,
          authorityEpoch,
          version,
          cause,
        }),
      { committedMayDefer: true },
    );
  }

  /** Fetches, verifies and retains an update for a later apply. */
  async prepare(version: string): Promise<UpdateCommandOutcome> {
    return this.#command(
      (client, snapshot, reportProgress) =>
        client.prepareWithProgress(
          {
            protocolVersion: UPDATE_PROTOCOL_VERSION,
            authorityEpoch: snapshot.authorityEpoch,
            version,
          },
          reportProgress,
        ),
      { prepare: true },
    );
  }

  /**
   * Applies the retained staged artifact, replacing the application.
   *
   * A gate refusal is a deferral: the artifact stays staged and the surface
   * can offer the restart again.
   */
  async apply(version: string): Promise<UpdateCommandOutcome> {
    return this.#command(
      async (client, snapshot) => {
        const staged = snapshot.staged;
        if (staged?.version !== version) {
          return { status: "rejected", code: "noOffer", detail: null, snapshot };
        }
        return client.apply({
          protocolVersion: UPDATE_PROTOCOL_VERSION,
          authorityEpoch: snapshot.authorityEpoch,
          version,
          digest: staged.digest,
        });
      },
      { committedMayDefer: true },
    );
  }

  /** Discards the retained staged artifact. */
  async cancel(): Promise<UpdateCommandOutcome> {
    return this.#command((client, { authorityEpoch }) =>
      client.cancel({ protocolVersion: UPDATE_PROTOCOL_VERSION, authorityEpoch }),
    );
  }

  async #command(
    run: (
      client: UpdateClient,
      snapshot: UpdateSnapshot,
      reportProgress: (event: UpdateProgressEvent) => void,
    ) => Promise<UpdateOutcomeProjection>,
    options: { readonly committedMayDefer?: boolean; readonly prepare?: boolean } = {},
  ): Promise<UpdateCommandOutcome> {
    const commandSequence = ++this.#commandSequence;
    this.#lastRejection = undefined;
    this.#lastRejectionDetail = null;
    const snapshot = this.#snapshot;
    if (snapshot === undefined) {
      // No snapshot means no epoch to send, and inventing one would have the
      // authority refuse it as stale. Say what is actually wrong.
      const error = new Error("update state has not been read yet");
      this.#setStatus({ kind: "failed", error });
      return { status: "failed", error };
    }
    const lifecycle = this.#lifecycle.revision;
    const isCurrent = () =>
      lifecycle === this.#lifecycle.revision && commandSequence === this.#commandSequence;
    const reportProgress = (event: UpdateProgressEvent) => {
      if (
        this.#lifecycle.active &&
        isCurrent() &&
        this.#pending &&
        event.authorityEpoch === snapshot.authorityEpoch &&
        event.authorityEpoch === this.#snapshot?.authorityEpoch
      ) {
        this.#liveProgress = event.progress;
        this.#notify();
      }
    };
    this.#pending = true;
    this.#legacyProgress =
      options.prepare && this.#client.port.prepareWithProgress === undefined
        ? reportProgress
        : undefined;
    this.#liveProgress = undefined;
    this.#notify();
    try {
      const outcome = await run(this.#client, snapshot, reportProgress);
      const result: UpdateCommandOutcome =
        outcome.status === "rejected"
          ? {
              status: "rejected",
              snapshot: outcome.snapshot,
              code: outcome.code,
              detail: outcome.detail,
            }
          : options.committedMayDefer && outcome.snapshot.deferral !== null
            ? {
                status: "committedWithDeferral",
                snapshot: outcome.snapshot,
                deferral: outcome.snapshot.deferral,
              }
            : { status: "committed", snapshot: outcome.snapshot };
      if (isCurrent()) {
        this.#pending = false;
        this.#legacyProgress = undefined;
        this.#refreshRevision += 1;
        this.#snapshot = outcome.snapshot;
        this.#liveProgress = undefined;
        this.#lastRejection = result.status === "rejected" ? result.code : undefined;
        this.#lastRejectionDetail = result.status === "rejected" ? result.detail : null;
        this.#setStatus({ kind: "ready" });
      }
      return result;
    } catch (error) {
      if (isCurrent()) {
        this.#refreshRevision += 1;
        this.#pending = false;
        this.#legacyProgress = undefined;
        this.#liveProgress = undefined;
        this.#setStatus({ kind: "failed", error });
      }
      return { status: "failed", error };
    }
  }

  #setStatus(status: UpdateControllerStatus): void {
    this.#status = status;
    this.#notify();
  }

  #notify(): void {
    for (const observer of this.#observers) {
      observer();
    }
  }
}
