# Tauri config-operation capability examples

Copy the selected permission and capability files into a consumer Tauri app.
The diagnostics capability grants snapshot and transition inspection only.
The operations capability adds confirmed storage mutation and backup
publication plus destructive restore and recovery. Restore inspection remains
read-only; planning cannot publish. Injected host authorization remains
required.
