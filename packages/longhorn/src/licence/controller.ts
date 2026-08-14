import { LicenceClient } from "./client.ts";
import {
  LICENCE_PROTOCOL_VERSION,
  type HeldLicenceProjection,
  type LicenceCredentialProjection,
  type LicenceRejectionCode,
  type LicenceSeatProjection,
  type LicenceSnapshot,
  type LicenceTrustBasisProjection,
  type LicenceUsabilityProjection,
} from "./generated/protocol.ts";
import type { LicencePort, LicenceUnlisten } from "./ports.ts";

export type LicenceControllerStatus =
  | { readonly kind: "idle" }
  | { readonly kind: "loading" }
  | { readonly kind: "ready" }
  | { readonly kind: "failed"; readonly error: unknown };

export interface LicenceControllerOptions {
  readonly port: LicencePort;
}

/**
 * What the operator should be told, if anything.
 *
 * A predicate rather than a rendering, and Longhorn's rather than a surface's,
 * for the same reason `UpdateController.presence` is: deriving it means
 * reading five usability states against a trust basis and two windows, and two
 * surfaces deriving it separately would disagree.
 *
 * - `none` — nothing to say. Includes an in-lease renewal that has not yet
 *   succeeded, which is the one this exists to get right.
 * - `informational` — worth showing where the operator is already looking, not
 *   worth interrupting for.
 * - `actionable` — the operator must do something or lose access.
 */
export type LicenceAttention = "none" | "informational" | "actionable";

/**
 * Holds licence state for a surface.
 *
 * Reports entitlement; never enforces. What a missing entitlement *does* is
 * the application's decision, which is contract 019's line and the reason this
 * exposes state rather than permissions.
 */
export class LicenceController {
  readonly #client: LicenceClient;
  readonly #observers = new Set<() => void>();
  #status: LicenceControllerStatus = { kind: "idle" };
  #snapshot?: LicenceSnapshot;
  #lastRejection?: LicenceRejectionCode;
  #started = false;
  #lifecycle = 0;
  #pending = false;
  #unlisten: LicenceUnlisten[] = [];

  constructor(options: LicenceControllerOptions) {
    this.#client = new LicenceClient(options.port);
  }

  get status(): LicenceControllerStatus { return this.#status; }
  get snapshot(): LicenceSnapshot | undefined { return this.#snapshot; }
  get licence(): HeldLicenceProjection | undefined { return this.#snapshot?.licence ?? undefined; }
  get usability(): LicenceUsabilityProjection | undefined { return this.licence?.usability; }
  get trustBasis(): LicenceTrustBasisProjection | undefined { return this.licence?.trustBasis; }
  get pending(): boolean { return this.#pending; }
  get lastRejection(): LicenceRejectionCode | undefined { return this.#lastRejection; }

  /** Whether a licence is held at all. Absence is not a usability state. */
  get activated(): boolean { return this.licence !== undefined; }

  /** Whether the software may be used. Grace counts, which is its point. */
  get usable(): boolean {
    const state = this.usability?.state;
    return state === "active" || state === "inGrace";
  }

  /**
   * When use stops being permitted, and when updates stop being covered.
   *
   * Two reads rather than one, because they are two different sentences. "Your
   * subscription lapsed" and "your updates lapsed but the app keeps working"
   * are not the same message, and conflating them on a perpetual licence reads
   * as the application breaking.
   */
  get useUntil(): number | undefined { return this.licence?.useUntil ?? undefined; }
  get updateUntil(): number | undefined { return this.licence?.updateUntil ?? undefined; }

  /** Entitlement ids held, opaque by design. Longhorn enumerates no features. */
  get entitlements(): readonly string[] {
    return this.licence?.entitlements.map((entitlement) => entitlement.id) ?? [];
  }

  /** Whether an entitlement is held. The application decides what that means. */
  holds(id: string): boolean {
    return this.licence?.entitlements.some((entitlement) => entitlement.id === id) ?? false;
  }

  /** The bound on an entitlement, or `undefined` for unlimited or unheld. */
  limit(id: string): number | undefined {
    return this.licence?.entitlements.find((entitlement) => entitlement.id === id)?.atMost ?? undefined;
  }

  /**
   * What to tell the operator.
   *
   * `inGrace` is deliberately `none`. A renewal that has not yet succeeded,
   * inside its lease, is a backend outage rather than the customer's problem —
   * and a surface that raises it turns an outage into a support ticket from
   * someone who has already paid.
   *
   * `clockRefused` is `actionable` and is not an expiry. The remedy is to fix
   * the clock, and a surface that showed it as expired would send the operator
   * to buy something they already own.
   */
  get attention(): LicenceAttention {
    if (!this.activated) return "actionable";
    switch (this.usability?.state) {
      case "active":
      case "inGrace":
        break;
      case "useWindowExpired":
      case "leaseLapsed":
      case "clockRefused":
        return "actionable";
      default:
        return "none";
    }
    // Usable, but updates are no longer covered. Worth saying where the
    // operator is already looking; not worth interrupting for, because
    // nothing stops working.
    return this.updateUntil !== undefined ? "informational" : "none";
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

  /**
   * Every seat held under this licence, this machine included.
   *
   * Empty when the authority does not account for seats, which is a different
   * state from a licence with one seat.
   */
  get seats(): readonly LicenceSeatProjection[] { return this.licence?.seats ?? []; }

  /** The seats that are not this machine — the ones a release would free. */
  get otherSeats(): readonly LicenceSeatProjection[] {
    return this.seats.filter((seat) => !seat.thisMachine);
  }

  /**
   * Presents a credential. One command for all three routes, as Card 193 set.
   *
   * `label` is what the customer calls this machine in the seat list, asked
   * for at the only moment it can be without inventing a settings screen.
   */
  async activate(
    credential: LicenceCredentialProjection,
    label: string | null = null,
  ): Promise<void> {
    await this.#command((client, epoch) =>
      client.activate({
        protocolVersion: LICENCE_PROTOCOL_VERSION,
        authorityEpoch: epoch,
        credential,
        label,
      }),
    );
  }

  /**
   * Releases another machine's seat.
   *
   * The answer to "I got a new laptop": free the seats that are not this one
   * without a support conversation. Separate from `deactivate`, which leaves
   * the machine you are sitting at.
   */
  async releaseSeat(machineId: string): Promise<void> {
    await this.#command((client, epoch) =>
      client.releaseSeat({
        protocolVersion: LICENCE_PROTOCOL_VERSION,
        authorityEpoch: epoch,
        machineId,
      }),
    );
  }

  /** Releases this machine's seat. */
  async deactivate(): Promise<void> {
    await this.#command((client, epoch) =>
      client.deactivate({ protocolVersion: LICENCE_PROTOCOL_VERSION, authorityEpoch: epoch }),
    );
  }

  /** Re-checks the lease now. */
  async refreshLease(): Promise<void> {
    await this.#command((client, epoch) =>
      client.refresh({ protocolVersion: LICENCE_PROTOCOL_VERSION, authorityEpoch: epoch }),
    );
  }

  /**
   * Renames a seat — this machine's or another's.
   *
   * `null` clears the label back to unnamed. The label stays the customer's
   * word: this is the one write that can change it after activation.
   */
  async renameSeat(machineId: string, label: string | null): Promise<void> {
    await this.#command((client, epoch) =>
      client.renameSeat({
        protocolVersion: LICENCE_PROTOCOL_VERSION,
        authorityEpoch: epoch,
        machineId,
        label,
      }),
    );
  }

  async #command(
    run: (
      client: LicenceClient,
      epoch: number,
    ) => Promise<import("./generated/protocol.ts").LicenceOutcomeProjection>,
  ): Promise<void> {
    const epoch = this.#snapshot?.authorityEpoch;
    if (epoch === undefined) {
      // Inventing one would have the authority refuse it as stale, which reads
      // as a protocol fault rather than as "nothing has been read yet".
      this.#setStatus({ kind: "failed", error: new Error("licence state has not been read yet") });
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

  #setStatus(status: LicenceControllerStatus): void {
    this.#status = status;
    this.#notify();
  }

  #notify(): void {
    for (const observer of this.#observers) observer();
  }
}
