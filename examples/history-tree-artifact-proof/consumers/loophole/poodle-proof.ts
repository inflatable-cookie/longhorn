// The Poodle-facing edge of this package is the session, not a panel:
// `ForkHistoryPanel` is gone, since Poodle's HistoryCenter covers branches and
// entries and the framework no longer ships a competing surface.
import { ForkHistorySession } from "@inflatable-cookie/longhorn-poodle-svelte/history-tree/svelte";

void ForkHistorySession;
