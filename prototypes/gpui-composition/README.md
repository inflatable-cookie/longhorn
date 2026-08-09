# GPUI composition example

The assembly from [the GPUI composition guide](../../docs/guides/gpui-composition.md),
as something that compiles.

```sh
cd prototypes/gpui-composition && cargo run
```

Read `src/main.rs` against the guide. Its sections are the guide's composition
order, and if the two diverge one of them is wrong.

## What it shows

- **`HostServices` supplied for real.** Request ids, a date rendered in words,
  and case folding. Deliberately not `PlainHostServices` — that exists for
  tests and is named to discourage shipping it.
- **The withheld capabilities, with their reasons**, straight from
  `WITHHELD_CAPABILITIES` rather than a list an application maintains.
- **One domain end to end.** A real `NotificationLedger`, projected by
  `longhorn-poodle`, rendered by `poodle-render`, drawn by
  `poodle-gpui-node-backend`. `Critical` carries its severity in the title,
  because it shares the danger tone with `Error`.

## What it does not show

The window backend. Writing a `GpuiWindowBackend` over `gpui::PlatformWindow`
is the neighbouring [`gpui-windowing`](../gpui-windowing) prototype's subject,
and this example opens its window through gpui directly rather than restating
it. This one is about the composition.

Nor six domains. The guide is the surface; this is the proof its assembly
holds.

## Gate

Covered by `effigy check:prototypes`, which runs outside `qa` and inside the
release gates. See [Card 172](../../docs/roadmaps/g02/batch-cards/172-gpui-build-cadence.md)
for why the cadence is that and not the workspace.
