**Breaking:** `publish_progress` and `publish_terminal` in
`longhorn-tauri-bridge` now fail with `readDenied` for a session without read
authority for the domain, as domain events already did. Grant read authority
(`projection` or `authoritative`) to sessions that must receive job events.
