//! Raw AppKit confinement for the controlled fake native child.

#![allow(unsafe_code)]

#[cfg(target_os = "macos")]
mod implementation {
    use std::{
        ffi::{CString, c_char, c_void},
        mem,
    };

    use tauri::{Runtime, Window};

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NSRect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    }

    type MsgSendId = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
    type MsgSendIdRect = unsafe extern "C" fn(*mut c_void, *mut c_void, NSRect) -> *mut c_void;
    type MsgSendVoid = unsafe extern "C" fn(*mut c_void, *mut c_void);
    type MsgSendVoidBool = unsafe extern "C" fn(*mut c_void, *mut c_void, bool);
    type MsgSendVoidId = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void);
    type MsgSendVoidUsize = unsafe extern "C" fn(*mut c_void, *mut c_void, usize);
    type MsgSendRect = unsafe extern "C" fn(*mut c_void, *mut c_void) -> NSRect;

    #[link(name = "objc")]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> *mut c_void;
        fn sel_registerName(name: *const c_char) -> *mut c_void;
        fn objc_msgSend();
        #[cfg(target_arch = "x86_64")]
        fn objc_msgSend_stret();
    }

    pub(crate) fn install_fake_child<R: Runtime>(window: &Window<R>) -> Result<bool, String> {
        let parent = window
            .ns_view()
            .map_err(|error| format!("could not resolve Tauri NSView: {error}"))?;
        if parent.is_null() {
            return Err("Tauri returned a null NSView".to_string());
        }
        unsafe {
            let view_class = class("NSView")?;
            let bounds = rect(parent, "bounds");
            let child = id_rect(id(view_class, "alloc"), "initWithFrame:", bounds);
            if child.is_null() {
                return Err("could not allocate fake NSView child".to_string());
            }
            void_bool(child, "setWantsLayer:", true);
            void_usize(child, "setAutoresizingMask:", 2 | 16);
            void_id(parent, "addSubview:", child);
            let retained_parent = id(child, "superview");
            void(child, "release");
            Ok(retained_parent == parent)
        }
    }

    pub(crate) fn release_focus<R: Runtime>(window: &Window<R>) -> Result<(), String> {
        let ns_window = window
            .ns_window()
            .map_err(|error| format!("could not resolve Tauri NSWindow: {error}"))?;
        unsafe { void(ns_window, "resignKeyWindow") };
        Ok(())
    }

    unsafe fn class(name: &str) -> Result<*mut c_void, String> {
        let name = CString::new(name).expect("class names contain no NUL");
        let value = unsafe { objc_getClass(name.as_ptr()) };
        if value.is_null() {
            Err(format!(
                "AppKit class {} is unavailable",
                name.to_string_lossy()
            ))
        } else {
            Ok(value)
        }
    }

    unsafe fn selector(name: &str) -> *mut c_void {
        let name = CString::new(name).expect("selector names contain no NUL");
        unsafe { sel_registerName(name.as_ptr()) }
    }

    unsafe fn id(receiver: *mut c_void, name: &str) -> *mut c_void {
        let send: MsgSendId = unsafe { mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
        unsafe { send(receiver, selector(name)) }
    }

    unsafe fn id_rect(receiver: *mut c_void, name: &str, value: NSRect) -> *mut c_void {
        let send: MsgSendIdRect = unsafe { mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
        unsafe { send(receiver, selector(name), value) }
    }

    unsafe fn void(receiver: *mut c_void, name: &str) {
        let send: MsgSendVoid = unsafe { mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
        unsafe { send(receiver, selector(name)) }
    }

    unsafe fn void_bool(receiver: *mut c_void, name: &str, value: bool) {
        let send: MsgSendVoidBool =
            unsafe { mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
        unsafe { send(receiver, selector(name), value) }
    }

    unsafe fn void_id(receiver: *mut c_void, name: &str, value: *mut c_void) {
        let send: MsgSendVoidId = unsafe { mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
        unsafe { send(receiver, selector(name), value) }
    }

    unsafe fn void_usize(receiver: *mut c_void, name: &str, value: usize) {
        let send: MsgSendVoidUsize =
            unsafe { mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
        unsafe { send(receiver, selector(name), value) }
    }

    unsafe fn rect(receiver: *mut c_void, name: &str) -> NSRect {
        #[cfg(target_arch = "aarch64")]
        {
            let send: MsgSendRect =
                unsafe { mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
            unsafe { send(receiver, selector(name)) }
        }
        #[cfg(target_arch = "x86_64")]
        {
            type MsgSendRectStret = unsafe extern "C" fn(*mut NSRect, *mut c_void, *mut c_void);
            let send: MsgSendRectStret =
                unsafe { mem::transmute(objc_msgSend_stret as unsafe extern "C" fn()) };
            let mut result = NSRect {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            };
            unsafe { send(&mut result, receiver, selector(name)) };
            result
        }
    }
}

#[cfg(target_os = "macos")]
pub(crate) use implementation::{install_fake_child, release_focus};

#[cfg(not(target_os = "macos"))]
pub(crate) fn install_fake_child<R: tauri::Runtime>(
    _window: &tauri::Window<R>,
) -> Result<bool, String> {
    Err("isolated native-window proof is unsupported on this target".to_string())
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn release_focus<R: tauri::Runtime>(_window: &tauri::Window<R>) -> Result<(), String> {
    Err("isolated native-window focus release is unsupported on this target".to_string())
}
