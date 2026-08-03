<script lang="ts">import { ForkHistoryPanel } from "../src/poodle.ts"; import { useForkHistorySession, type ForkHistorySession } from "../src/svelte.ts"; let { session }: { session: ForkHistorySession } = $props(); useForkHistorySession(session);</script>
<ForkHistoryPanel {session} />
