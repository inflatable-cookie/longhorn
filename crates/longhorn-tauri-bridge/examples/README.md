# Tauri Bridge Capability Examples

Copy the selected permission and capability files into a consumer Tauri app.
The query-only shape admits hello, authority refresh, query, and resync
without event permissions. The subscription shape adds authoritative command,
cancellation, and the Tauri event listener lifetime.

These files grant platform reachability only. They do not create a bridge
session, advertise a domain capability, or grant read, write, or execution
authority. The Rust assembly checks those facts before dispatch.
