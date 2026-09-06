# g02.036 Production Contextual Agent Tools

Status: planning blocked; no execution authority
Owner: Longhorn maintainers, coordinated with Swallowtail and Desktop
Created: 2026-09-06
Governing refs: contract 023 (proposed), contracts 001, 006, 007, 010, 012,
015, 019, 020, 021, 022; spec 002; repo authority map

## Goal

Define and later prove a production-safe authenticated app-tool boundary.
Longhorn owns server semantics; Swallowtail owns reusable harness transport and
lifecycle; Desktop owns discovery, context, UX, skills, and Bovine tools.

## Execution plan

- [ ] Settle and promote contract 023, including the Swallowtail attachment
      seam, credential reference, identity, negotiation, admission, approval,
      retry, limits, and release-shape decisions.
- [ ] Implement the generic core registry/admission fixture after promotion.
- [ ] Add the separately opted-in Tauri producer mount and release-absence
      gates, with one generic read-only registered tool.
- [ ] Run cross-repo harness acceptance through Claude, Codex, Grok, and a
      disposable MCP/tool client; keep Bovine implementations in Desktop.

## Acceptance criteria

- [ ] Production capability is separate from contract 022's dev feature.
- [ ] Namespaced typed discovery, exact instance/PID identity, credential
      references, version negotiation, cleanup, and stale-instance refusal are
      proved.
- [ ] Task/session/attempt admission, deadlines, cancellation, bounded
      concurrency, typed results/errors, non-replayed mutation, least-access,
      and approval handoff are proved.
- [ ] A release-built Tauri consumer invokes one generic tool and contains no
      dev evaluate, synthetic input, shell, or arbitrary command bridge.
- [ ] Swallowtail's adapter and the real client/disposable acceptance sequence
      are recorded without moving Desktop policy into Longhorn.

## Readiness

Blocked. No card is ready while the proposed contract and cross-repo ownership
decisions remain unresolved. The next planning checkpoint is joint settlement
of the attachment and credential/admission seams.
