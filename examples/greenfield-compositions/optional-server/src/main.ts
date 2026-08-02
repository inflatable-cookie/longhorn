import { mount } from "svelte";

import App from "../../common/App.svelte";
import { selectedModules } from "./selected.ts";

mount(App, {
  target: document.getElementById("app")!,
  props: {
    shape: "optional-server",
    selectedModules,
    status: { kind: "ready", authority: "local-config:1" },
  },
});
