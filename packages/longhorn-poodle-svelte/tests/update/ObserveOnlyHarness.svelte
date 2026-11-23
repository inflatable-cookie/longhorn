<script lang="ts">
  import { UpdateStatus } from "@inflatable-cookie/poodle-svelte";
  import type { UpdateController } from "@inflatable-cookie/longhorn/update";

  // Deliberately NOT sampled. Every prop is read once from a plain class, so
  // the only thing that could make this reactive is Poodle's own `observe`.
  let { controller }: { controller: UpdateController } = $props();

  // A getter keeps this prop readable on each call to Poodle's observer.
  const source = {
    get availability() {
      const availability = controller.availability;
      return availability?.state === "notChecked" ? undefined : availability;
    },
  };
</script>

<UpdateStatus
  pending={controller.pending}
  status={controller.status}
  {...source.availability === undefined ? {} : { availability: source.availability }}
  {...controller.progress === undefined ? {} : { progress: controller.progress }}
  {...controller.channel === undefined ? {} : { channel: controller.channel }}
  {...controller.installedVersion === undefined ? {} : { installedVersion: controller.installedVersion }}
  observe={(o) => controller.observe(o)}
/>
