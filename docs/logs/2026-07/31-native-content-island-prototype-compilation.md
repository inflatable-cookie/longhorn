# Native Content Island Prototype Compilation

Date: 2026-07-31
State: complete planning batch

## Outcome

- compiled g01.013 into Cards 082-086
- made Card 082 the sole ready card
- kept all prototype code non-publishable and outside the root workspace
- separated child-webview, isolated-window, and backing-surface proof cards
- required packaged macOS behavior for every mechanism
- required explicit Windows and Linux support or unsupported status
- required unused mechanisms to stay absent from isolated graphs
- reserved production package topology and public clients for Card 086
- kept donor migrations blocked until production artifacts exist

## Compiled Runway

1. Card 082 — private pure coordination model and three-shape traces
2. Card 083 — child-webview packaged mechanism proof
3. Card 084 — isolated native-window packaged fake-child proof
4. Card 085 — backing-surface packaged mechanism proof
5. Card 086 — promotion decision and g01.013 closeout

Card 082 is ready. Cards 083-086 remain planned.

## Package Gate

The provisional `longhorn-native-content` and `@inflatable-cookie/longhorn-native-content`
names remain documentation only. Cards 082-085 cannot join the root workspace,
publish artifacts, create compatibility promises, or trigger donor migration.

Card 086 chooses promote, narrow, retain, or reject. A promote or narrow
outcome must compile a separate production runway before implementation.

## Mechanism Gates

- child webview: bounds, reuse, visibility, focus, close, scale, and remote
  capability confinement
- isolated window: fake-child resize negotiation, helper loss, focus, and
  bounded teardown without a plugin dependency
- backing surface: full-host native storage, viewport clipping, forwarded
  input gating, scale, destroy, and declared detach policy
- every mechanism: packaged macOS evidence, per-target support truth, stale
  generation rejection, and graph isolation

## Limits

- no code or prototype implemented
- no donor repository changed
- no production package or public API created
- no cross-platform support claimed
- no browser, plugin, GPU, input, or Poodle authority moved into Longhorn

## Validation

- focused Northstar structure and link checks
- ready/planned and live-pointer drift scans
- `git diff --check`

## Posture

`strict-ready`

## Next

Execute Card 082. Stop if one pure model cannot represent all three traces
without product payloads or host implementation.
