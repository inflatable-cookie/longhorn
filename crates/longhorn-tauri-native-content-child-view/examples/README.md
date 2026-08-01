# Capability example

`controller-only.json` grants no commands and matches only the trusted local
controller webview. It has no `remote` selector and does not name a child-view
label. Remote child content therefore receives no Tauri capability by default.

Consumers must make any broader capability grant explicit in their own Tauri
application. A capability grant remains command access, not authorization to
navigate, download, open popups, or mutate product state.
