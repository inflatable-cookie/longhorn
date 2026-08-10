import {
  BridgeProtocolValidationError,
  parseBridgeHelloRequest,
} from "@inflatable-cookie/longhorn/bridge";

export function protocolArtifactTrace() {
  const accepted = parseBridgeHelloRequest({
    protocolVersion: 1,
    bridgeId: "bridge:artifact-proof",
    requestedDomains: ["fixture.catalog"],
  });
  let futureVersion: string | undefined;
  try {
    parseBridgeHelloRequest({
      protocolVersion: 2,
      bridgeId: "bridge:artifact-proof",
      requestedDomains: ["fixture.catalog"],
    });
  } catch (error) {
    if (!(error instanceof BridgeProtocolValidationError)) {
      throw error;
    }
    futureVersion = error.code;
  }
  if (futureVersion === undefined) {
    throw new Error("future bridge protocol version was accepted");
  }
  return {
    acceptedVersion: accepted.protocolVersion,
    futureVersion,
  } as const;
}
