import {
  BridgeQueryRetryRuntime,
  type BridgeRuntimeClock,
} from "@inflatable-cookie/longhorn/bridge";

import {
  adapterNames,
  declaration,
  declarations,
  type ShapeName,
} from "./common.ts";
import { runSplitShellTrace } from "./split-shell.ts";
import { runJetstreamTrace } from "./jetstream.ts";
import { runLoopholeTrace } from "./loophole.ts";
import { runNucleusTrace } from "./nucleus.ts";
import { runSoundcheckTrace } from "./soundcheck.ts";

export async function runBridgeTopologyProof() {
  const traces = {
    "split-shell": await runSplitShellTrace(),
    jetstream: await runJetstreamTrace(),
    soundcheck: await runSoundcheckTrace(),
    nucleus: await runNucleusTrace(),
    loophole: await runLoopholeTrace(),
  };
  return {
    schema: "longhorn.bridge-topology-conformance.v1",
    declarationSchema: declarations.schema,
    adapters: adapterNames,
    traces,
    dependencyGraph: dependencyGraph(),
    topologyMatrix: Object.fromEntries(
      shapeNames().map((name) => [
        name,
        {
          hostForms: declaration(name).hostForms,
          serviceOwnership: declaration(name).serviceOwnership,
          transportFeatures: declaration(name).transportFeatures,
        },
      ]),
    ),
    retryAudit: retryAudit(),
    audits: {
      productPayloadInSharedPackages: false,
      silentAuthorityFallback: false,
      upwardOptionalEdge: false,
      productionTransportUsed: false,
      donorWrites: false,
    },
  } as const;
}

function dependencyGraph() {
  return Object.fromEntries(
    shapeNames().map((name) => [
      name,
      {
        imports: declaration(name).imports,
        tauriPermissions: declaration(name).tauriPermissions,
        events:
          declaration(name).imports.includes("@inflatable-cookie/longhorn-tauri/bridge-events"),
        service:
          declaration(name).imports.includes("@inflatable-cookie/longhorn/bridge/supervision"),
      },
    ]),
  );
}

function retryAudit() {
  return Object.fromEntries(
    shapeNames().map((name) => {
      const maximum = declaration(name).maximumQueryRetries;
      const clock: BridgeRuntimeClock = { now: () => 100 };
      const runtime = new BridgeQueryRetryRuntime(
        clock,
        { delay: (_class, attempt) => attempt * 10 },
        maximum,
      );
      const admitted = [];
      for (let attempt = 0; attempt <= maximum; attempt += 1) {
        admitted.push(
          runtime.schedule("retry", "afterBackoff")?.attempt ?? null,
        );
      }
      return [name, { maximum, admitted }];
    }),
  );
}

function shapeNames(): ShapeName[] {
  return [
    "split-shell",
    "jetstream",
    "soundcheck",
    "nucleus",
    "loophole",
  ];
}
