/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! Simple exports for cbindgen to generate C headers
//! This module only contains the types and function signatures that cbindgen can understand

use std::ffi::c_void;
use std::os::raw::{c_char, c_float, c_int, c_uint};

/// Opaque handle for Servo instance
pub type ServoHandle = u64;

/// Opaque handle for WebView instance  
pub type WebViewHandle = u64;

/// Error codes for Swift interop
#[derive(Clone, Copy)]
#[repr(C)]
pub enum ServoError {
    Success = 0,
    InvalidHandle = 1,
    InvalidUrl = 2,
    InitializationFailed = 3,
    RenderingFailed = 4,
}

/// Initialization options for Servo
#[derive(Clone, Copy)]
#[repr(C)]
pub struct ServoInitOptions {
    pub enable_hardware_acceleration: bool,
    pub enable_experimental_features: bool,
    pub user_agent: *const c_char,
    pub cache_size_mb: c_uint,
}

impl Default for ServoInitOptions {
    fn default() -> Self {
        ServoInitOptions {
            enable_hardware_acceleration: true,
            enable_experimental_features: false,
            user_agent: std::ptr::null(),
            cache_size_mb: 100,
        }
    }
}

// Function declarations for cbindgen
unsafe extern "C" {
    /// Create the Servo instance attached to an NSView
    pub fn create_servo(
        nsview_ptr: *mut c_void,
        width: c_uint,
        height: c_uint,
        scale_factor: c_float,
        options: *const ServoInitOptions,
    ) -> ServoHandle;

    /// Create a new WebView within the Servo instance
    pub fn create_webview(
        servo_handle: ServoHandle,
        url: *const c_char,
        width: c_uint,
        height: c_uint,
        scale_factor: c_float,
    ) -> WebViewHandle;

    /// Load a URL in an existing WebView
    pub fn webview_load_url(
        servo_handle: ServoHandle,
        webview_handle: WebViewHandle,
        url: *const c_char,
    ) -> ServoError;

    /// Resize a WebView
    pub fn webview_resize(
        servo_handle: ServoHandle,
        webview_handle: WebViewHandle,
        width: c_uint,
        height: c_uint,
    ) -> ServoError;

    /// Paint a WebView to its rendering context
    pub fn webview_paint(
        servo_handle: ServoHandle,
        webview_handle: WebViewHandle,
    ) -> ServoError;

    /// Spin the Servo event loop
    pub fn spin_event_loop(servo_handle: ServoHandle) -> ServoError;

    /// Destroy a WebView
    pub fn destroy_webview(
        servo_handle: ServoHandle,
        webview_handle: WebViewHandle,
    ) -> ServoError;

    /// Destroy the Servo instance and all associated WebViews
    pub fn destroy_servo(servo_handle: ServoHandle) -> ServoError;

    /// Get the Servo version string
    pub fn version() -> *const c_char;
}
