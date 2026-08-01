# Nucleus No-Surface Migration Compilation

Date: 2026-08-01
Roadmap: g01.014

## Result

Compiled Cards 094-101 into four migration batches. Card 094 is ready as a
read-only donor behavior, authority, Browser-policy, and rollback freeze. No
Nucleus or Poodle file changed.

## Current Donor Evidence

The audit used clean Nucleus `main` at
`c084d57ca15f9e4276f49a9b6b2923f5d10e7313`. Current product behavior has
removed hosted Surfaces and uses:

```text
display -> window -> region -> panel
```

Layouts are complete local-client documents keyed by project id. Native
window placement remains global. The five-region model, project isolation,
panel mutations, and child-webview Browser implementation are current donor
authority.

## Compiled Runway

| Batch | Cards | Outcome |
| --- | --- | --- |
| admission and freeze | 094-095 | checked donor baseline and private artifact/source receipt gate |
| durable state and window | 096-097 | canonical-id platform storage, `.nucleus` import, protected primary window |
| no-Surface workspace | 098-099 | project-keyed shared layout authority and checked public-Poodle renderer |
| native Browser and closeout | 100-101 | child-view coordination, duplicate removal, rollback and conformance |

The roadmap keeps project, task, runtime, resource, panel-catalogue, shell, and
Browser policy in Nucleus. Longhorn receives only product-neutral mechanism
authority.

## Storage Decision

Nucleus uses canonical id `dev.nucleus.desktop`, no stable storage-name
override, and `platform-native-v1`. The old `~/.nucleus` root is an explicit
legacy candidate. Proof roots map to `portable-v1`. The combined `ui.json`
must seed separate window and project-layout domains. SQLite uses native
snapshot/import. The fixed locator commits last and old source remains until
receipt-bound cleanup.

The CLI's current-working-directory `.nucleus/local/nucleus.sqlite` default is
outside the first desktop migration.

## Browser Policy

The first cutover preserves HTTP/HTTPS-only navigation, HTTPS normalization,
popup/download denial, no app permission prompt, normal shared engine data
store, no cookie/credential persistence in Nucleus state, trusted-toolbar
controls, remote-child capability isolation, process-lifetime reuse, hide on
unmount/project switch, and destroy on panel close.

Current overlay intersection partly depends on private Poodle DOM. That code
cannot move into Longhorn. Card 099 must preserve behavior through explicit
consumer visibility and a public Poodle seam or stop for an upstream contract.

## Gates

- Card 094 is ready and writes only Longhorn evidence.
- Private migrations use exact clean sibling sources plus separate packed
  artifact proof. Package-manager publication is deferred.
- Poodle g12.018 satisfies the public geometry and private artifact seam.
- Cross-project layout retention is operator-accepted. The new-project Agent
  Chat-only check remains open before overlapping donor writes.
- Every donor card rechecks the Nucleus worktree and stops on overlapping work.

## Post-compilation Correction

The original compilation incorrectly made registry publication a donor-write
gate. That contradicts the private development posture already used by
Nucleus and Poodle. Contract 012 and Card 095 now separate private source
linkage from clean produced-artifact evidence. Registry publication moves to
g01.016 release preparation.

## Validation

- focused read-only Nucleus source, test, contract, roadmap, capability, and
  dependency audit
- Longhorn roadmap and contract reconciliation
- `effigy qa:northstar:g01-nucleus-migration-compilation`

## Next

Record Nucleus g05's remaining new-project check, then execute Card 095's
private artifact admission without modifying Nucleus or publishing packages.
