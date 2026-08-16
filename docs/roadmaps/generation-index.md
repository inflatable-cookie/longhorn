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

**Live:** Bovine publishing characterized one shared credential-store gap.
[g02.028](g02/028-consumer-scoped-credential-slots.md) and Card 225 are ready.

The memo-023 release-audit suite — milestones g02.021-027, cards 200-224 — is
complete except the separately operator-held release machinery noted below.

- Complete: g02.021 (200-203), g02.022 (204-206), g02.023 (207-210, 224),
  g02.024 (211-214), g02.025 (215-217), g02.027 (221-223).
- Card 224 was opened by review of the suite itself: Card 210's identity
  generation had a first-run race, and the compare-and-swap that would close
  it does not exist on any platform backend. The refusal and the narrowing
  that replaced it are recorded on the trait and in contract 004.
- Operator decisions of 2026-08-15 landed: 210 (store vocabulary moved to
  `longhorn-core`) and the Card 214 sidebar-label divergence (Svelte side
  correct; Rust aligned). Only 218 remains held, on Poodle v0.2.0.
- The `.github/workflows/` items in 219/220 landed 2026-08-15 with approval.
- g02.026 is closed except Card 218, which is held whole — no machinery from
  it is in the tree. Longhorn's release waits on Poodle v0.2.0 by operator
  decision, so the linked-Poodle exemption stays until then.
- Next ready: Card 225.

Pre-suite lanes are tracked on the [g02 runway](g02/README.md): g02.015's
Card 176, g02.017-019 in progress, Card 149 operator-held.

## Memo 023 Release-audit Suite

Research memo 023 (full-repo release audit, 2026-08-14) compiles into
milestones [g02.021-027](g02/README.md) — cards 200-223, the suite that is
the live pointer above. Its state lives there; this section stays as the
memo-to-roadmap provenance.
