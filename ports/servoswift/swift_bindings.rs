/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! C FFI bindings for Swift integration
//!
//! This module provides a C-compatible interface that can be called from Swift.
//! All Servo instances and WebViews are managed on the Swift side as opaque pointers.

use std::ffi::{CStr, c_char, c_void};
use std::rc::Rc;

use url::Url;
use log::{debug, warn, error};
use dpi::PhysicalSize;
use euclid::{Box2D, Size2D, Scale};
use servo::{Servo, ServoBuilder, WebView, WebViewBuilder};
use compositing_traits::rendering_context::{RenderingContext, SoftwareRenderingContext};
use servo::servo_url::ServoUrl;

use crate::init_crypto;

/// Actual Servo data that Swift will hold as opaque pointer
struct ServoInstance {
    servo: Servo,
    rendering_context: Rc<dyn RenderingContext>,
}

/// Actual WebView data  
struct WebViewInstance {
    webview: WebView,
}

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

    let _servo_options = if options.is_null() {
        ServoInitOptions::default()
    } else {
        unsafe { *options }
    };

    debug!("Creating Servo instance with size: {}x{}, scale: {}", width, height, scale_factor);

    // Create the rendering context
    let size = PhysicalSize::new(width, height);
    
    // Create a software rendering context for now
    let rendering_context: Rc<dyn RenderingContext> = match SoftwareRenderingContext::new(size) {
        Ok(ctx) => Rc::new(ctx),
        Err(e) => {
            error!("Failed to create rendering context: {:?}", e);
            return std::ptr::null_mut();
        }
    };

    // Create Servo builder with the rendering context
    let builder = ServoBuilder::new(rendering_context.clone());
    
    // Set up resource directory if provided
    // TODO: Use options.resources_dir_path
    
    // Create Servo instance
    let servo = builder.build();

    let instance = ServoInstance {
        servo,
        rendering_context,
    };

    let instance_ptr = Box::into_raw(Box::new(instance)) as *mut c_void;
    debug!("Created Servo instance at {:p}", instance_ptr);
    instance_ptr
}

/// Create a WebView within the Servo instance and return an opaque pointer to it
#[unsafe(no_mangle)]
pub extern "C" fn create_webview(
    servo_ptr: *mut c_void,
    url: *const c_char,
    width: u32,
    height: u32,
    scale_factor: f32,
) -> *mut c_void {
    if servo_ptr.is_null() {
        error!("Invalid Servo pointer");
        return std::ptr::null_mut();
    }

    let servo_instance = unsafe { &*(servo_ptr as *const ServoInstance) };

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

    let parsed_url = match ServoUrl::parse(url_str) {
        Ok(url) => url,
        Err(e) => {
            error!("Failed to parse URL '{}': {:?}", url_str, e);
            return std::ptr::null_mut();
        }
    };

    // Create WebView with the specified size
    let size = PhysicalSize::new(width, height);
    let rect = Box2D::from_size(Size2D::new(size.width as f32, size.height as f32));

    let webview = WebViewBuilder::new(&servo_instance.servo)
        .url(parsed_url.clone().into_url())
        .hidpi_scale_factor(Scale::new(scale_factor))
        .build();

    // Resize the WebView to the specified dimensions
    webview.move_resize(rect);

    // Navigate to the URL
    webview.load(parsed_url.into_url());    let webview_instance = WebViewInstance { webview };
    let webview_ptr = Box::into_raw(Box::new(webview_instance)) as *mut c_void;

    debug!("Created WebView at {:p} for URL: {}", webview_ptr, url_str);
    webview_ptr
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

    let parsed_url = match Url::parse(url_str) {
        Ok(url) => url,
        Err(_) => return ServoError::InvalidUrl,
    };

    let webview_instance = unsafe { &*(webview_ptr as *mut WebViewInstance) };
    webview_instance.webview.load(parsed_url);

    debug!("Loaded URL {} in WebView {:p}", url_str, webview_ptr);
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
        let _webview_instance = Box::from_raw(webview_ptr as *mut WebViewInstance);
        // WebView will be dropped automatically
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
        let _servo_instance = Box::from_raw(servo_ptr as *mut ServoInstance);
        // Servo instance will be dropped automatically
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

/// Get a render callback function that can blit the Servo framebuffer to a parent OpenGL context
/// Returns a function pointer that takes (gl_context, x, y, width, height) parameters
#[unsafe(no_mangle)]
pub extern "C" fn get_render_callback(
    servo_ptr: *mut c_void,
) -> Option<extern "C" fn(*const c_void, i32, i32, i32, i32)> {
    if servo_ptr.is_null() {
        error!("Invalid Servo pointer");
        return None;
    }

    // For now, return a simple dummy callback
    // TODO: Implement actual render_to_parent_callback integration
    debug!("Render callback requested for Servo {:p}", servo_ptr);
    Some(render_to_parent_static)
}

/// Static function that handles rendering to parent context
/// This will be called from Swift with OpenGL context and coordinates
extern "C" fn render_to_parent_static(
    _gl_context: *const c_void,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    debug!("Render to parent called: ({}, {}) {}x{}", x, y, width, height);
    // TODO: Implement actual blit operation
    // This would need access to the specific servo instance and its render callback
}
