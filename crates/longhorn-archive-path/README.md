# Longhorn Archive Path

The archive entry-name and link-target rule shared by Longhorn's bounded
extractions: `longhorn-update-install` (contract 018) and
`longhorn-tool-provision` (contract 027).

`bounded` drops `.` components and refuses `..`, a root or a drive prefix,
and an empty path. It does not touch the disk. Callers that need the
filesystem's view add their own check on top.
