# Canonical User-export Re-encoding

Date: 2026-08-02
Status: complete

## Change

Nucleus exposed a missing public step between selecting an inventoried
operational archive and publishing it to a user-selected export target.

`longhorn-config` now re-encodes a verified inspection through the canonical
archive encoder with only the manifest kind changed to `user-export`. Snapshot
identity, metadata, domain evidence, exclusions, payload bytes, and checksums
remain exact. The source archive is not mutated.

This keeps consumer apps out of Longhorn's ZIP vocabulary and prevents export
from recapturing newer state under an older selected digest.

## Evidence

- focused codec fixture proves operational input remains operational
- exported inspection is `user-export`
- archive id, creation time, domain evidence, and payload bytes match
- output digest differs because the manifest purpose changed
- contract 004 forbids raw copy, recapture, and consumer-owned codecs

## Consumer

Nucleus g05 card 045 may compose its asynchronous native save picker before
calling the synchronous configuration authority, then use this primitive and
Longhorn's existing verified export publication path.
