import { onMount } from "svelte";
import type { ForkHistorySession } from "./session.svelte.ts";
export function useForkHistorySession(session: ForkHistorySession): void { onMount(() => { void session.start().catch(() => undefined); return () => { void session.stop(); }; }); }
