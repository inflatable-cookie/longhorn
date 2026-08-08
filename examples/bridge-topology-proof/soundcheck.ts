import { BridgeJobTracker } from "@inflatable-cookie/longhorn/bridge";
import { BridgeServiceRuntime } from "@inflatable-cookie/longhorn/bridge/supervision";
import {
  BRIDGE_PROGRESS_EVENT,
  BRIDGE_TERMINAL_EVENT,
  listenTauriBridgeJob,
} from "@inflatable-cookie/longhorn-tauri/bridge-events";

import {
  cancellationParity,
  declaration,
  jsonRoundTrip,
  queryParity,
  sameValues,
  unknownCodec,
  type AdapterName,
} from "./common.ts";

const requestId = "request:fixture-soundcheck-job";
const jobId = "job:fixture-soundcheck";

export async function runSoundcheckTrace() {
  const fixture = declaration("soundcheck");
  const jobDomain = fixture.domains.at(-1)!;
  const cancellation = await cancellationParity("soundcheck", jobDomain);
  const jobs = {
    direct: directJobTrace(false),
    tauri: await tauriJobTrace(),
    loopback: directJobTrace(true),
  } satisfies Record<AdapterName, readonly string[]>;

  const service = new BridgeServiceRuntime("externalLocal");
  const failedService = await service.execute(
    { perform: () => ({ failed: "attachFailed" }) },
    "attach",
  );
  const localDomains = fixture.domains.filter(
    ({ authority }) => authority.availability === "available",
  );
  const localQueries: Record<
    string,
    Awaited<ReturnType<typeof queryParity>>["traces"]
  > = {};
  for (const domain of localDomains) {
    localQueries[domain.domainId] = (
      await queryParity("soundcheck", domain)
    ).traces;
  }

  return {
    cancellationParity: sameValues(cancellation),
    jobParity: sameValues(jobs),
    jobs,
    serviceState: failedService.current,
    localDomainsAvailableAfterServiceFailure:
      localDomains.length === 3 &&
      Object.values(localQueries).every((traces) => sameValues(traces)),
    localQueries,
  } as const;
}

function directJobTrace(serialized: boolean): readonly string[] {
  const tracker = new BridgeJobTracker(requestId, jobId);
  const transfer = <T>(value: T) =>
    serialized ? jsonRoundTrip(value) : value;
  const progress = transfer({
    requestId,
    jobId,
    progress: { completed: 1, total: 2 },
  });
  const foreign = transfer({
    requestId: "request:fixture-other",
    jobId,
    progress: { completed: 2, total: 2 },
  });
  const terminal = transfer({
    requestId,
    jobId,
    outcome: { succeeded: { value: 2 } },
  });
  return [
    tracker.classifyProgress(progress),
    tracker.classifyProgress(foreign),
    tracker.classifyTerminal(terminal),
    tracker.classifyProgress(progress),
    tracker.classifyTerminal(terminal),
  ];
}

async function tauriJobTrace(): Promise<readonly string[]> {
  const listeners = new Map<string, (value: unknown) => void>();
  const accepted: string[] = [];
  const transport = {
    invoke: () => Promise.reject(new Error("invoke is not used")),
    listen(event: string, listener: (value: unknown) => void) {
      listeners.set(event, listener);
      return Promise.resolve(() => {
        listeners.delete(event);
      });
    },
  };
  const dispose = await listenTauriBridgeJob(
    requestId,
    jobId,
    {
      progressCodec: unknownCodec,
      successCodec: unknownCodec,
      detailCodec: unknownCodec,
      progress: () => accepted.push("accept"),
      terminal: () => accepted.push("terminal"),
    },
    transport,
  );
  const emit = (event: string, value: unknown) =>
    listeners.get(event)?.(value);
  emit(BRIDGE_PROGRESS_EVENT, {
    requestId,
    jobId,
    progress: { completed: 1, total: 2 },
  });
  emit(BRIDGE_PROGRESS_EVENT, {
    requestId: "request:fixture-other",
    jobId,
    progress: { completed: 2, total: 2 },
  });
  const terminal = {
    requestId,
    jobId,
    outcome: { succeeded: { value: 2 } },
  };
  emit(BRIDGE_TERMINAL_EVENT, terminal);
  emit(BRIDGE_PROGRESS_EVENT, {
    requestId,
    jobId,
    progress: { completed: 2, total: 2 },
  });
  emit(BRIDGE_TERMINAL_EVENT, terminal);
  await dispose();
  if (JSON.stringify(accepted) !== '["accept","terminal"]') {
    throw new Error(
      `unexpected Tauri job callbacks: ${JSON.stringify(accepted)}`,
    );
  }
  return [
    "accept",
    "ignoreWrongCorrelation",
    "accept",
    "ignoreAfterTerminal",
    "ignoreAlreadyTerminal",
  ];
}
