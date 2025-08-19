/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! ServoSwift - A Swift/macOS port of the Servo web engine
//!
//! This library provides a C FFI interface for embedding Servo in Swift applications
//! on macOS. It handles the creation and management of Servo instances, WebViews,
//! and provides the necessary bridge between Swift/Objective-C and Servo's Rust API.

mod swift_bindings;
mod servo_view;
mod rendering_context;
mod event_handling;

pub use swift_bindings::*;
pub use servo_view::*;

use std::sync::Once;

/// Initialize the crypto provider for Servo
pub fn init_crypto() {
    static INIT: Once = Once::new();
    
    INIT.call_once(|| {
        match rustls::crypto::aws_lc_rs::default_provider().install_default() {
            Ok(_) => {
                log::debug!("Crypto provider initialized successfully");
            }
            Err(e) => {
                log::error!("Failed to initialize crypto provider: {:?}", e);
                // Don't panic here, just log the error
            }
        }
    });
}
