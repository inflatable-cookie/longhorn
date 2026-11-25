`execute_gpui_window_apply` takes a fifth argument, `observer: &mut impl
GpuiApplyObserver`. Pass the `GpuiWindowLifecycleHost` that will see the
window events; it implements the trait. A harness with no lifecycle host must
pass its own observer and say so.
