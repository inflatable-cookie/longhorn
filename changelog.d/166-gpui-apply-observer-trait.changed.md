`GpuiApplyObserver` is new. Its `register_apply` and `retag` methods are what
the apply calls before each native mutation and each identity change. `retag`
returns `GpuiRetagError`, which is new: `Refused` leaves the registry
unchanged, and `Rescheduling` means the window moved but a pending deadline
could not be re-issued, so the registry moves too and the attempt reports it.
