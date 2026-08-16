# g02.028 Consumer-scoped Credential Slots

Status: ready
Owner: Longhorn maintainers
Created: 2026-08-16
Governing refs: contract 021; contracts 001, 003, 004, and 012
Depends on: g02.023 complete

## Outcome

Longhorn's one credential-store mechanism can isolate consumer-owned secret
purposes by stable scope without learning product policy or changing persisted
built-in keyring names.

## Generation Runway

- [ ] [Card 225](batch-cards/225-consumer-scoped-credential-slot-value.md)
      replaces the closed enum, migrates built-in callers and backends, and
      proves scoped isolation.

This is one bounded shared-mechanism prerequisite characterized by Bovine's
publishing adoption. Bovine composition stays in Bovine.

## Acceptance

- Existing licence and backup credentials keep their exact storage names.
- Independently scoped consumer slots cannot alias.
- Invalid identities fail before platform access.
- No Bovine or Farmyard policy enters Longhorn.
- Longhorn QA passes.

## Next Task

Execute Card 225.

