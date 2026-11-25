`longhorn_bindings::generation::field_map(task, constant, declarations)` now
returns `String` instead of `(String, Vec<String>)`; migration: bind the
returned string directly instead of destructuring and discarding skipped
union names.
