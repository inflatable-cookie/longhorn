<script lang="ts">
  import { CommandPalette } from "@inflatable-cookie/poodle-svelte";

  import type { CommandExecutionOutcome } from "@inflatable-cookie/longhorn/commands";

  import type { CommandSession } from "../svelte/session.svelte.ts";
  import { toPoodleCommandItems, toPoodleDiscoveryState } from "./projectors.ts";

  interface Props {
    session: CommandSession;
    title?: string;
    description?: string | null;
    invocationHint?: string | null;
    categoryLabel?: (categoryPath: ReadonlyArray<string>) => string | null;
    onOpenChange?: (open: boolean) => void;
  }

  let {
    session,
    title = "Command palette",
    description = null,
    invocationHint = null,
    categoryLabel = undefined,
    onOpenChange = undefined,
  }: Props = $props();

  const selectionMessages: Record<
    Exclude<CommandExecutionOutcome["status"], "succeeded">,
    string
  > = {
    unknownCommand: "This command is no longer available. Choose another command.",
    staleRegistry: "The command list changed. Reopen the palette and try again.",
    invalidArguments: "This command could not run with the supplied arguments.",
    unavailable: "This command is unavailable in the current context.",
    unauthorized: "You don’t have permission to run this command.",
    cancelled: "The command was cancelled.",
    rejected: "The app refused to run this command.",
    failed: "The command failed. Try again.",
    indeterminate: "The app could not confirm that the command finished.",
  };

  let attempt = 0;
  let failure = $state.raw<{
    session: CommandSession;
    action: "search" | "select";
    message: string;
  } | null>(null);
  let currentFailure = $derived(failure?.session === session ? failure : null);

  async function run(
    action: "search" | "select",
    perform: () => Promise<string | null>,
    failureMessage: string,
  ): Promise<void> {
    const current = session;
    const revision = ++attempt;
    failure = null;
    let message: string | null;
    try {
      message = await perform();
    } catch {
      message = failureMessage;
    }
    if (message !== null && revision === attempt && current === session && current.open) {
      failure = { session: current, action, message };
    }
  }

  let records = $derived(session.paletteRecords);
  let items = $derived(toPoodleCommandItems(records, categoryLabel));
  let discoveryState = $derived(
    currentFailure?.action === "search"
      ? "error"
      : toPoodleDiscoveryState(session.status, session.query, items.length),
  );
</script>

<CommandPalette
  open={session.open}
  query={session.query}
  {items}
  state={discoveryState}
  {title}
  description={currentFailure?.message ?? description}
  {invocationHint}
  onOpenChange={(open) => {
    ++attempt;
    failure = null;
    session.setOpen(open);
    onOpenChange?.(open);
  }}
  onQueryChange={(query) =>
    void run(
      "search",
      async () => {
        await session.setQuery(query);
        return null;
      },
      "Could not search commands. Change the search and try again.",
    )}
  onCommandSelect={(commandId) =>
    void run(
      "select",
      async () => {
        const outcome = await session.select(commandId);
        return outcome.status === "succeeded" ? null : selectionMessages[outcome.status];
      },
      selectionMessages.failed,
    )}
/>
