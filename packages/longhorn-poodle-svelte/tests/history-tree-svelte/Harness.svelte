<script lang="ts">import { ForkHistoryPanel } from "../../src/history-tree/poodle.ts"; import { useForkHistorySession, type ForkHistorySession } from "../../src/history-tree/svelte.ts"; let { session }: { session: ForkHistorySession } = $props(); useForkHistorySession(session);</script>
<ForkHistoryPanel {session} />
