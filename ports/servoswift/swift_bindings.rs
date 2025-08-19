/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! C FFI bindings for Swift integration
//!
//! This module provides a C-compatible interface that can be called from Swift.
//! It follows patterns similar to the Android JNI interface but adapted for
//! Swift/Objective-C interoperability.

use std::ffi::{CStr, c_char, c_void};
use std::sync::Arc;
use std::os::raw::{c_int, c_float, c_uint};
use std::ptr::NonNull;

use euclid::{Scale, Size2D};
use servo::{Servo, ServoBuilder, WebView, WebViewBuilder, WindowRenderingContext};
use raw_window_handle::{AppKitDisplayHandle, AppKitWindowHandle, RawDisplayHandle, RawWindowHandle};
use url::Url;
use log::{debug, error};
use dpi::PhysicalSize;

use crate::{SERVO_INSTANCES, next_instance_id, ServoInstance, init_crypto};

/// Opaque handle for Servo instance
pub type ServoHandle = u64;
/// Opaque handle for WebView instance  
pub type WebViewHandle = u64;

/// Error codes for Swift interop
#[repr(C)]
pub enum ServoError {
    Success = 0,
    InvalidHandle = 1,
    InvalidUrl = 2,
    RenderingContextError = 3,
    UnknownError = 4,
}

/// Servo initialization options
#[repr(C)]
pub struct ServoInitOptions {
    pub enable_webgpu: bool,
    pub enable_webxr: bool,
    pub log_level: c_int,
}

impl Default for ServoInitOptions {
    fn default() -> Self {
        Self {
            enable_webgpu: true,
            enable_webxr: false,
            log_level: 2, // Info level
        }
    }
}

/// Initialize Servo library (call once at app startup)
#[unsafe(no_mangle)]
pub extern "C" fn servo_init() -> ServoError {
    init_crypto();
    
    // Initialize logging
    env_logger::init();
    
    debug!("ServoSwift initialized");
    ServoError::Success
}

/// Create a new Servo instance
#[unsafe(no_mangle)]
pub extern "C" fn servo_create_instance(
    nsview_ptr: *mut c_void,
    width: c_uint,
    height: c_uint,
    scale_factor: c_float,
    options: *const ServoInitOptions,
) -> ServoHandle {
    if nsview_ptr.is_null() {
        error!("Invalid NSView pointer");
        return 0;
    }

    let options = if options.is_null() {
        ServoInitOptions::default()
    } else {
        unsafe { *options }
    };

    // Create display and window handles
    let display_handle = RawDisplayHandle::AppKit(AppKitDisplayHandle::new());
    let window_handle = RawWindowHandle::AppKit(AppKitWindowHandle::new(NonNull::new(nsview_ptr).unwrap()));

    // Create rendering context
    let size = dpi::PhysicalSize::new(width, height);
    let rendering_context = match WindowRenderingContext::new(
        display_handle.into(),
        window_handle.into(),
        size,
    ) {
        Ok(ctx) => Arc::new(ctx),
        Err(e) => {
            error!("Failed to create rendering context: {:?}", e);
            return 0;
        }
    };

    // Build Servo instance
    let servo = ServoBuilder::new(rendering_context)
        .build();

    servo.setup_logging();

    let instance_id = next_instance_id();
    let servo_instance = ServoInstance::new(servo);

    {
        let mut instances = SERVO_INSTANCES.lock().unwrap();
        instances.insert(instance_id, servo_instance);
    }

    debug!("Created Servo instance with ID: {}", instance_id);
    instance_id
}

/// Create a new WebView within a Servo instance
#[unsafe(no_mangle)]
pub extern "C" fn servo_create_webview(
    servo_handle: ServoHandle,
    url: *const c_char,
    width: c_uint,
    height: c_uint,
    scale_factor: c_float,
) -> WebViewHandle {
    if servo_handle == 0 {
        error!("Invalid Servo handle");
        return 0;
    }

    let url_str = if url.is_null() {
        "about:blank"
    } else {
        unsafe {
            match CStr::from_ptr(url).to_str() {
                Ok(s) => s,
                Err(_) => {
                    error!("Invalid URL string");
                    return 0;
                }
            }
        }
    };

    let parsed_url = match Url::parse(url_str) {
        Ok(url) => url,
        Err(e) => {
            error!("Failed to parse URL '{}': {:?}", url_str, e);
            return 0;
        }
    };

    let mut instances = SERVO_INSTANCES.lock().unwrap();
    let instance = match instances.get_mut(&servo_handle) {
        Some(instance) => instance,
        None => {
            error!("Servo instance not found: {}", servo_handle);
            return 0;
        }
    };

    let webview_id = instance.next_webview_id();
    
    let webview = WebViewBuilder::new(&instance.servo)
        .url(parsed_url)
        .size(Size2D::new(width as f32, height as f32))
        .hidpi_scale_factor(Scale::new(scale_factor))
        .build();

    webview.focus();
    webview.raise_to_top(true);

    instance.webviews.insert(webview_id, webview);

    debug!("Created WebView with ID: {} in Servo instance: {}", webview_id, servo_handle);
    webview_id
}

/// Load a URL in an existing WebView
#[unsafe(no_mangle)]
pub extern "C" fn servo_webview_load_url(
    servo_handle: ServoHandle,
    webview_handle: WebViewHandle,
    url: *const c_char,
) -> ServoError {
    if servo_handle == 0 || webview_handle == 0 {
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

    let parsed_url = match Url::parse(url_str) {
        Ok(url) => url,
        Err(_) => return ServoError::InvalidUrl,
    };

    let instances = SERVO_INSTANCES.lock().unwrap();
    let instance = match instances.get(&servo_handle) {
        Some(instance) => instance,
        None => return ServoError::InvalidHandle,
    };

    let webview = match instance.webviews.get(&webview_handle) {
        Some(webview) => webview,
        None => return ServoError::InvalidHandle,
    };

    webview.load_url(parsed_url);
    ServoError::Success
}

/// Resize a WebView
#[unsafe(no_mangle)]
pub extern "C" fn servo_webview_resize(
    servo_handle: ServoHandle,
    webview_handle: WebViewHandle,
    width: c_uint,
    height: c_uint,
) -> ServoError {
    if servo_handle == 0 || webview_handle == 0 {
        return ServoError::InvalidHandle;
    }

    let instances = SERVO_INSTANCES.lock().unwrap();
    let instance = match instances.get(&servo_handle) {
        Some(instance) => instance,
        None => return ServoError::InvalidHandle,
    };

    let webview = match instance.webviews.get(&webview_handle) {
        Some(webview) => webview,
        None => return ServoError::InvalidHandle,
    };

    let size = dpi::PhysicalSize::new(width, height);
    webview.resize(size);
    
    ServoError::Success
}

/// Paint a WebView to its rendering context
#[unsafe(no_mangle)]
pub extern "C" fn servo_webview_paint(
    servo_handle: ServoHandle,
    webview_handle: WebViewHandle,
) -> ServoError {
    if servo_handle == 0 || webview_handle == 0 {
        return ServoError::InvalidHandle;
    }

    let instances = SERVO_INSTANCES.lock().unwrap();
    let instance = match instances.get(&servo_handle) {
        Some(instance) => instance,
        None => return ServoError::InvalidHandle,
    };

    let webview = match instance.webviews.get(&webview_handle) {
        Some(webview) => webview,
        None => return ServoError::InvalidHandle,
    };

    webview.paint();
    ServoError::Success
}

/// Spin the Servo event loop
#[unsafe(no_mangle)]
pub extern "C" fn servo_spin_event_loop(servo_handle: ServoHandle) -> ServoError {
    if servo_handle == 0 {
        return ServoError::InvalidHandle;
    }

    let instances = SERVO_INSTANCES.lock().unwrap();
    let instance = match instances.get(&servo_handle) {
        Some(instance) => instance,
        None => return ServoError::InvalidHandle,
    };

    instance.servo.spin_event_loop();
    ServoError::Success
}

/// Destroy a WebView
#[unsafe(no_mangle)]
pub extern "C" fn servo_destroy_webview(
    servo_handle: ServoHandle,
    webview_handle: WebViewHandle,
) -> ServoError {
    if servo_handle == 0 || webview_handle == 0 {
        return ServoError::InvalidHandle;
    }

    let mut instances = SERVO_INSTANCES.lock().unwrap();
    let instance = match instances.get_mut(&servo_handle) {
        Some(instance) => instance,
        None => return ServoError::InvalidHandle,
    };

    match instance.webviews.remove(&webview_handle) {
        Some(_) => {
            debug!("Destroyed WebView: {}", webview_handle);
            ServoError::Success
        }
        None => ServoError::InvalidHandle,
    }
}

/// Destroy a Servo instance and all associated WebViews
#[unsafe(no_mangle)]
pub extern "C" fn servo_destroy_instance(servo_handle: ServoHandle) -> ServoError {
    if servo_handle == 0 {
        return ServoError::InvalidHandle;
    }

    let mut instances = SERVO_INSTANCES.lock().unwrap();
    match instances.remove(&servo_handle) {
        Some(mut instance) => {
            // Clear all WebViews first
            instance.webviews.clear();
            // Servo destructor will handle cleanup
            debug!("Destroyed Servo instance: {}", servo_handle);
            ServoError::Success
        }
        None => ServoError::InvalidHandle,
    }
}

/// Get the version string
#[unsafe(no_mangle)]
pub extern "C" fn servo_version() -> *const c_char {
    static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"));
    VERSION.as_ptr() as *const c_char
}
