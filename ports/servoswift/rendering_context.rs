/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! macOS-specific rendering context utilities
//!
//! This module provides utilities for working with macOS NSView integration
//! for Servo's WindowRenderingContext. The actual rendering context creation
//! is handled by Servo's surfman-based WindowRenderingContext in swift_bindings.rs.

use std::ffi::c_void;
use dpi::PhysicalSize;
use raw_window_handle::{AppKitWindowHandle, AppKitDisplayHandle, RawWindowHandle, RawDisplayHandle};
use std::ptr::NonNull;

/// Create raw window and display handles from an NSView pointer
/// This is used internally by the FFI layer to convert NSView to raw-window-handle types
pub fn create_raw_handles_from_nsview(
    nsview_ptr: *mut c_void,
) -> Result<(RawDisplayHandle, RawWindowHandle), Box<dyn std::error::Error>> {
    if nsview_ptr.is_null() {
        return Err("NSView pointer is null".into());
    }

    let display_handle = RawDisplayHandle::AppKit(AppKitDisplayHandle::new());
    let window_handle = RawWindowHandle::AppKit(
        AppKitWindowHandle::new(NonNull::new(nsview_ptr).unwrap())
    );

    Ok((display_handle, window_handle))
}

/// Validate that the NSView size matches the requested rendering size
/// This helps catch size mismatches early in the rendering pipeline
pub fn validate_nsview_size(
    _nsview_ptr: *mut c_void,
    expected_size: PhysicalSize<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Implement actual NSView size validation using Objective-C interop
    // For now, we trust that the Swift layer is passing correct sizes
    log::debug!(
        "Validating NSView size: {}x{}",
        expected_size.width,
        expected_size.height
    );
    Ok(())
}

/// Check if the NSView supports the required rendering capabilities
/// This can be extended to check for Metal, OpenGL support, etc.
pub fn check_nsview_capabilities(
    _nsview_ptr: *mut c_void,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Implement capability checking:
    // - Metal support detection
    // - OpenGL context compatibility
    // - High DPI support
    // - Color space support
    log::debug!("Checking NSView rendering capabilities");
    Ok(())
}
