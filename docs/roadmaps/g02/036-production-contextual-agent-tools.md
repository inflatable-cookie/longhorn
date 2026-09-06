# g02.036 Production Contextual Agent Tools

Status: planning blocked; no execution authority
Owner: Longhorn maintainers, coordinated with Swallowtail and Desktop
Created: 2026-09-06
Governing refs: contract 023 (proposed), contracts 001, 006, 007, 010, 012,
015, 019, 020, 021, 022; spec 002; repo authority map

## Goal

Define and later prove a production-safe authenticated app-tool boundary.
Swallowtail owns the registration snapshot and one operation bridge kernel;
Longhorn owns typed host dispatch/validation only; Desktop owns tool/domain
policy, context disclosure, admission issuance, and packaging.

## Execution plan

- [ ] Reconcile Swallowtail PR254/spec014 and Contract060 with this split;
      preserve WatcherBridge as a closed compatible profile.
- [ ] Settle and promote Longhorn's typed dispatch/validation contract,
      credential-reference, protocol/schema, bounded result/error, and release
      absence/API evidence surfaces.
- [ ] Implement the generic core registry/admission fixture after promotion.
- [ ] Desktop packages the linked Longhorn host library and supplies one
      read-only tool plus bounded context/admission; no standalone daemon.
- [ ] Swallowtail owns the route matrix and cross-repo harness acceptance;
      Longhorn consumes that evidence without duplicating provider gates.

## Acceptance criteria

- [ ] Production capability is separate from contract 022's dev feature.
- [ ] Longhorn validates the admitted namespaced snapshot, trusted process
      incarnation/operation generation, credential references, versions, and
      bounded results without owning discovery or identity issuance.
- [ ] Task/session/attempt admission, deadlines, cancellation, bounded
      concurrency, typed results/errors, non-replayed mutation, least-access,
      and approval handoff are proved.
- [ ] A release-built Tauri consumer invokes one generic tool and contains no
      dev evaluate, synthetic input, shell, or arbitrary command bridge.
- [ ] Swallowtail's adapter and route matrix, plus Desktop's context disclosure
      and durable task/attempt binding, are recorded by their owning plans.

## Readiness

Blocked. No card is ready while Swallowtail has not revised PR254/Contract060
and Desktop's disclosure/admission record is not published through its
canonical spec. Longhorn-owned planning items are not additional operator
gates; route support remains Swallowtail-owned.
