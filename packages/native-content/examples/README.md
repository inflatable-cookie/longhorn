# Native-content capability composition

Grant `read-native-content` to a trusted renderer that only observes island
coordination. Add `mutate-native-content` only when that renderer may submit
desired viewport/lifecycle state or decide a content-size proposal.

These permissions admit protocol access only. Content construction,
navigation, plugin loading, rendering, semantic input, and product mutation
require separate consumer-owned authorization. Tauri labels select transport
targets and never become `NativeContentIslandId`.
