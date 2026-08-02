import { mount } from "svelte";

import App from "../../common/App.svelte";
import { selectedModules } from "./selected.ts";

mount(App, {
  target: document.getElementById("app")!,
  props: {
    shape: "minimal",
    selectedModules,
    status: { kind: "ready", authority: "config:1" },
  },
});
