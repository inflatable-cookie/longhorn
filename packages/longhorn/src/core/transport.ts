export type Unlisten = () => void | Promise<void>;

export interface InvokeTransport {
  invoke(
    command: string,
    arguments_: Record<string, unknown>,
  ): Promise<unknown>;
}

export interface EventTransport extends InvokeTransport {
  listen(
    event: string,
    listener: (payload: unknown) => void,
  ): Promise<Unlisten>;
}

export function isEventTransport(
  transport: InvokeTransport,
): transport is EventTransport {
  return (
    "listen" in transport &&
    typeof (transport as { listen?: unknown }).listen === "function"
  );
}
