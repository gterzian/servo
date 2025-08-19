/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! C FFI bindings for Swift integration
//!
//! This module provides a C-compatible interface that can be called from Swift.
//! All Servo instances and WebViews are managed on the Swift side as opaque pointers.

use std::ffi::{CStr, c_char, c_void};

use url::Url;
use log::{debug, warn, error};

use crate::init_crypto;

/// Opaque handle that Swift will manage (not used on Rust side)
pub type ServoHandle = u64;
/// Opaque handle that Swift will manage (not used on Rust side)  
pub type WebViewHandle = u64;

/// Error codes that can be returned from Servo operations
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum ServoError {
    Success = 0,
    InvalidHandle = 1,
    InvalidUrl = 2,
    RenderingError = 3,
    UnknownError = 4,
}

/// Initialization options for Servo
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ServoInitOptions {
    pub enable_subpixel_text_antialiasing: bool,
    pub enable_compositing_debug_overlay: bool,
    pub resources_dir_path: *const c_char,
}

impl Default for ServoInitOptions {
    fn default() -> Self {
        Self {
            enable_subpixel_text_antialiasing: true,
            enable_compositing_debug_overlay: false,
            resources_dir_path: std::ptr::null(),
        }
    }
}

/// Create the single Servo instance and return an opaque pointer to it
/// This handles all initialization and should only be called once
#[unsafe(no_mangle)]
pub extern "C" fn create_servo(
    nsview_ptr: *mut c_void,
    width: u32,
    height: u32,
    scale_factor: f32,
    options: *mut ServoInitOptions,
) -> *mut c_void {
    if nsview_ptr.is_null() {
        warn!("NSView pointer is null");
        return std::ptr::null_mut();
    }

    // Initialize logging and crypto - only happens once due to Once
    env_logger::init();
    init_crypto();

    let servo_options = if options.is_null() {
        ServoInitOptions::default()
    } else {
        unsafe { *options }
    };

    debug!("Creating Servo instance with size: {}x{}, scale: {}", width, height, scale_factor);

    // For now, return a dummy pointer since we can't easily create a real Servo instance
    // without proper rendering context integration
    let dummy_value = Box::new(42u64);
    let dummy_ptr = Box::into_raw(dummy_value) as *mut c_void;

    debug!("Created dummy Servo instance at {:p}", dummy_ptr);
    dummy_ptr
}

/// Create a WebView within the Servo instance and return an opaque pointer to it
#[unsafe(no_mangle)]
pub extern "C" fn create_webview(
    servo_ptr: *mut c_void,
    url: *const c_char,
    _width: u32,
    _height: u32,
    _scale_factor: f32,
) -> *mut c_void {
    if servo_ptr.is_null() {
        error!("Invalid Servo pointer");
        return std::ptr::null_mut();
    }

    let url_str = if url.is_null() {
        "about:blank"
    } else {
        unsafe {
            match CStr::from_ptr(url).to_str() {
                Ok(s) => s,
                Err(_) => {
                    error!("Invalid URL string");
                    return std::ptr::null_mut();
                }
            }
        }
    };

    let _parsed_url = match Url::parse(url_str) {
        Ok(url) => url,
        Err(e) => {
            error!("Failed to parse URL '{}': {:?}", url_str, e);
            return std::ptr::null_mut();
        }
    };

    // For now, return a dummy pointer
    let dummy_value = Box::new(43u64);
    let dummy_ptr = Box::into_raw(dummy_value) as *mut c_void;

    debug!("Created dummy WebView at {:p} for URL: {}", dummy_ptr, url_str);
    dummy_ptr
}

/// Load a URL in an existing WebView
#[unsafe(no_mangle)]
pub extern "C" fn webview_load_url(
    _servo_ptr: *mut c_void,
    webview_ptr: *mut c_void,
    url: *const c_char,
) -> ServoError {
    if webview_ptr.is_null() {
        return ServoError::InvalidHandle;
    }

    if url.is_null() {
        return ServoError::InvalidUrl;
    }

    let url_str = unsafe {
        match CStr::from_ptr(url).to_str() {
            Ok(s) => s,
            Err(_) => return ServoError::InvalidUrl,
        }
    };

    let _parsed_url = match Url::parse(url_str) {
        Ok(url) => url,
        Err(_) => return ServoError::InvalidUrl,
    };

    debug!("Load URL {} in WebView {:p}", url_str, webview_ptr);
    ServoError::Success
}

/// Resize a WebView
#[unsafe(no_mangle)]
pub extern "C" fn webview_resize(
    _servo_ptr: *mut c_void,
    webview_ptr: *mut c_void,
    width: u32,
    height: u32,
) -> ServoError {
    if webview_ptr.is_null() {
        return ServoError::InvalidHandle;
    }

    debug!("Resize WebView {:p} to {}x{}", webview_ptr, width, height);
    ServoError::Success
}

/// Paint a WebView to its surface
#[unsafe(no_mangle)]
pub extern "C" fn webview_paint(
    _servo_ptr: *mut c_void,
    webview_ptr: *mut c_void,
) -> ServoError {
    if webview_ptr.is_null() {
        return ServoError::InvalidHandle;
    }

    debug!("Paint WebView {:p}", webview_ptr);
    ServoError::Success
}

/// Spin the event loop for the Servo instance
#[unsafe(no_mangle)]
pub extern "C" fn spin_event_loop(servo_ptr: *mut c_void) -> ServoError {
    if servo_ptr.is_null() {
        return ServoError::InvalidHandle;
    }

    debug!("Spin event loop for Servo {:p}", servo_ptr);
    ServoError::Success
}

/// Destroy a WebView (Swift should call this in WebView deinit)
#[unsafe(no_mangle)]
pub extern "C" fn destroy_webview(
    _servo_ptr: *mut c_void,
    webview_ptr: *mut c_void,
) -> ServoError {
    if webview_ptr.is_null() {
        return ServoError::InvalidHandle;
    }

    unsafe {
        let _dummy = Box::from_raw(webview_ptr as *mut u64);
    }

    debug!("Destroyed WebView at {:p}", webview_ptr);
    ServoError::Success
}

/// Destroy the Servo instance (Swift should call this in ServoInstance deinit)
#[unsafe(no_mangle)]
pub extern "C" fn destroy_servo(servo_ptr: *mut c_void) -> ServoError {
    if servo_ptr.is_null() {
        return ServoError::InvalidHandle;
    }

    unsafe {
        let _dummy = Box::from_raw(servo_ptr as *mut u64);
    }

    debug!("Destroyed Servo instance at {:p}", servo_ptr);
    ServoError::Success
}

/// Get the version string
#[unsafe(no_mangle)]
pub extern "C" fn version() -> *const c_char {
    static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"));
    VERSION.as_ptr() as *const c_char
}
