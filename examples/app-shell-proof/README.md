# Artifact-installed App Shell Proof

Three isolated consumers prove compositional shell guidance without defining
one Longhorn app frame.

| Shape | Composition | Longhorn graph |
| --- | --- | --- |
| Split-shell | one public Poodle split, generic authority lifetime | core, Svelte |
| Nucleus | window, container, five regions, panels | core, layout, Svelte, Poodle |
| Loophole | display, window, Surface, container, eight regions, panels, transfer | full optional client and adapter graph |

Each consumer has its own manifest, capability example, shell, and mounted
tests. The verifier packs current Longhorn packages, installs each consumer in
an isolated temporary directory, pins the exact Card 038 Poodle artifact set,
runs `svelte-check` and Vitest, and rejects sibling source resolution.

Run:

```sh
bun scripts/verify-app-shell-proof.ts
```

The committed manifests are proof inputs. The verifier rewrites Longhorn and
Poodle dependencies to the produced tarballs before installation. They are not
path-based consumer migration manifests.

Canonical composition guidance:
[`docs/architecture/app-shell-composition.md`](../../docs/architecture/app-shell-composition.md).
