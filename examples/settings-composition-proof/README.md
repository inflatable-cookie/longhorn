# Artifact-installed Settings Composition Proof

Four isolated consumers prove one settings registry/session contract without
defining a shared application frame.

| Shape | Host | Composition |
| --- | --- | --- |
| Split-shell | modal | one staged product preference |
| Soundcheck | window | product settings plus shared storage, backup, restore, and recovery |
| Loophole | panel | immediate, staged, policy-controlled, hardware, and keybinding pages |
| Nucleus | window | one product page; no Surface or backend navigation |

The verifier:

- packages and unpacks the Rust settings/config crates
- builds a Rust consumer from those unpacked artifacts
- packs `@inflatable-cookie/longhorn/core`, `@inflatable-cookie/longhorn/settings`, and `@inflatable-cookie/longhorn/config`
- installs every consumer in an isolated temporary root
- pins the exact Card 038 Poodle artifact set
- runs `svelte-check` and mounted Vitest proof
- rejects sibling source resolution, duplicate Svelte, unexpected optional
  packages, and capability drift

Run:

```sh
effigy proof:settings-composition
```

The manifests and capabilities are proof inputs. The verifier rewrites package
versions to produced archives only inside the temporary roots.

Canonical guidance:
[`docs/architecture/settings-composition.md`](../../docs/architecture/settings-composition.md).
