/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! Event handling for macOS integration
//!
//! This module handles the translation of macOS events (NSEvent) into
//! Servo's input event system, including mouse, keyboard, and scroll events.

use std::os::raw::{c_int, c_float};

/// Mouse event types for C FFI
#[repr(C)]
pub enum MouseEventType {
    MouseDown = 0,
    MouseUp = 1,
    MouseMove = 2,
    MouseDrag = 3,
}

/// Keyboard event types for C FFI
#[repr(C)]
pub enum KeyEventType {
    KeyDown = 0,
    KeyUp = 1,
}

/// Handle mouse events from macOS
#[unsafe(no_mangle)]
pub extern "C" fn servo_handle_mouse_event(
    _servo_handle: u64,
    _webview_handle: u64,
    _event_type: MouseEventType,
    _x: c_float,
    _y: c_float,
    _button: c_int,
) {
    // TODO: Implement mouse event handling
}

/// Handle keyboard events from macOS
#[unsafe(no_mangle)]
pub extern "C" fn servo_handle_key_event(
    _servo_handle: u64,
    _webview_handle: u64,
    _event_type: KeyEventType,
    _key_code: c_int,
    _modifiers: c_int,
) {
    // TODO: Implement keyboard event handling
}

/// Handle scroll events from macOS
#[unsafe(no_mangle)]
pub extern "C" fn servo_handle_scroll_event(
    _servo_handle: u64,
    _webview_handle: u64,
    _delta_x: c_float,
    _delta_y: c_float,
    _x: c_float,
    _y: c_float,
) {
    // TODO: Implement scroll event handling
}
