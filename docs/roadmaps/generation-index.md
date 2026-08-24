# Generation Index

## Mode

Dependency-led. New generations start only after a shared Longhorn gap is
characterized and promoted through the docs spine.

## Active Generation

[g02](g02/README.md) — workspace integrity remediation, now open past it.
Research memo 018 characterizes the post-g01 audit gap; Cards 138-147
delivered all six remediation milestones. Research memo 019 characterizes
the in-app update gap and compiles contract 018; g02.009 (Cards 150-154) is
the first non-remediation milestone. Research memo 020 compiles contract 019
over licensing as g02.010 (Cards 155-158).

## Rollover History

- [g01](g01/README.md) — complete, 20 milestones, Cards 001-137

## Next Task

One live pointer lives here, updated in the same commit that closes a card —
the running narrative that used to fill this section drifted into three
competing pointers, which is how Card 216 came to exist. Card closeout detail
lives on the cards themselves.

**Live:** the agent-control arc is complete in Longhorn (g02.029-033,
Cards 227-237, PRs 2-6, all 2026-08-19). Next: per-app adoption — each
consumer repo compiles one card: compose per
`docs/guides/agent-control-composition.md`, then
`bun scripts/install-agent-control-skill.ts <repo>` (from the Longhorn
checkout). Soundcheck adopted first (2026-08-20, its
`docs/logs/2026-08/20-agent-control-adoption.md`) and confirmed the
`claude mcp add` paste-line end to end — the adoption kit's last open
review item. Figmatic is in flight on the `NoCommandBridge` composition.
Figmatic's screenshot blocker cleared with g02.034 (PR 7,
`e135d725`); its PR 14 reruns on that revision. Child-webview semantic
targeting is merged:
[g02.035](g02/035-agent-control-child-webview-targeting.md) (Cards
239-240, PR 8, `592dbdb8`) — an app opts in child labels and agents
drive them directly. Figmatic's full path is now open: rerun PR 14 on
this revision, `.with_semantic_child("figmatic-preview")` at mount,
re-run the skill installer, automate preview acceptance (never opt in
`longhorn-browser` views). Three apps remain after Figmatic; operator
picks order.
Still open on the operator side: return the promoted Longhorn commit to
Bovine Card 128 adoption (from g02.028).

The memo-023 release-audit suite — milestones g02.021-027, cards 200-224 — is
complete except Card 218's remaining release machinery, noted below.

- Complete: g02.021 (200-203), g02.022 (204-206), g02.023 (207-210, 224),
  g02.024 (211-214), g02.025 (215-217), g02.027 (221-223).
- Card 224 was opened by review of the suite itself: Card 210's identity
  generation had a first-run race, and the compare-and-swap that would close
  it does not exist on any platform backend. The refusal and the narrowing
  that replaced it are recorded on the trait and in contract 004.
- Operator decisions of 2026-08-15 landed: 210 (store vocabulary moved to
  `longhorn-core`) and the Card 214 sidebar-label divergence (Svelte side
  correct; Rust aligned). Only 218 remains open, and it is no longer held.
- The `.github/workflows/` items in 219/220 landed 2026-08-15 with approval.
- g02.026 is closed except Card 218, which is part-landed. Its Poodle
  precondition is met at public 0.2.2, and g16.008 removed the linked-Poodle
  exemption (step 5), which unblocks g02.014. Step 1 — a release gate on the
  `linkedPoodleAccepted` flag — is obsolete with the flag. Steps 2-4 are
  release hardening and need recompiling before they are ready.
- No Longhorn card auto-starts from Card 225.
- Card 226 (2026-08-17) closed a mixed-scale coordinate gap raised by Figmatic
  consumer evidence: `LogicalLayoutMapper` converts each object through its own
  scale, valid on macOS and Linux and excluded on Windows. Contract 009's
  blanket ban on per-monitor division now states the per-platform truth.
  Figmatic adoption is the next task and belongs to the Figmatic thread.

Pre-suite lanes are tracked on the [g02 runway](g02/README.md). g02.015 closed
2026-08-17 and g02.018 closed 2026-08-12; g02.017 and g02.019 are the
in-progress lanes, g02.020 is ready. Open pre-suite cards are 166 (ready), 160
and 177 (in progress). Card 149 is operator-held.

## Memo 023 Release-audit Suite

Research memo 023 (full-repo release audit, 2026-08-14) compiles into
milestones [g02.021-027](g02/README.md) — cards 200-223, the suite that is
the live pointer above. Its state lives there; this section stays as the
memo-to-roadmap provenance.
