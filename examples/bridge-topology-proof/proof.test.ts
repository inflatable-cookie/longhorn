import { describe, expect, test } from "bun:test";

import { runBridgeTopologyProof } from "./proof.ts";

describe("five-shape bridge topology conformance", () => {
  test("preserves adapter semantics and explicit optional boundaries", async () => {
    const proof = await runBridgeTopologyProof();

    expect(proof.traces["split-shell"]).toMatchObject({
      queryOnly: true,
      adapterParity: true,
      eventsResolved: false,
      serviceResolved: false,
    });
    expect(proof.traces["ordered-streams"]).toMatchObject({
      listenerFirst: true,
      gapResync: true,
      adapterParity: true,
      queryParity: true,
    });
    expect(proof.traces["jobs-and-service-failure"]).toMatchObject({
      cancellationParity: true,
      jobParity: true,
      serviceState: "failed",
      localDomainsAvailableAfterServiceFailure: true,
    });
    expect(proof.traces["capability-authority"]).toMatchObject({
      hostConnectionSeparate: true,
      optionalHostLifecycle: "attaching",
      domainIdentityStable: true,
      capabilityDoesNotGrantWrite: "write_denied",
      capabilityDoesNotGrantExecution: "execution_denied",
      executionParity: true,
      queryParity: true,
    });
    expect(proof.traces["reconnecting-lifecycle"]).toMatchObject({
      hostForms: ["localFirst", "remote"],
      hostInstancesDiffer: true,
      sessionsDiffer: true,
      authorityStable: true,
      remoteLifecycle: "attaching",
      queryParity: true,
    });
    expect(proof.dependencyGraph["split-shell"]).toMatchObject({
      events: false,
      service: false,
    });
    expect(proof.audits).toEqual({
      productPayloadInSharedPackages: false,
      silentAuthorityFallback: false,
      upwardOptionalEdge: false,
      productionTransportUsed: false,
      donorWrites: false,
    });
  });

  test("bounds retries for every topology", async () => {
    const { retryAudit } = await runBridgeTopologyProof();
    for (const trace of Object.values(retryAudit)) {
      expect(trace.admitted.slice(0, trace.maximum)).toEqual(
        Array.from(
          { length: trace.maximum },
          (_, index) => index + 1,
        ),
      );
      expect(trace.admitted.at(-1)).toBeNull();
    }
  });
});
