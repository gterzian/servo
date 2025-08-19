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

use servo::{Servo, WebView};
use std::sync::{Mutex, LazyLock};
use std::collections::HashMap;

/// Global state management for Servo instances and WebViews
static SERVO_INSTANCES: LazyLock<Mutex<HashMap<u64, ServoInstance>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static NEXT_INSTANCE_ID: Mutex<u64> = Mutex::new(1);

/// A wrapper around a Servo instance with associated WebViews
pub struct ServoInstance {
    pub servo: Servo,
    pub webviews: HashMap<u64, WebView>,
    pub next_webview_id: u64,
}

impl ServoInstance {
    pub fn new(servo: Servo) -> Self {
        Self {
            servo,
            webviews: HashMap::new(),
            next_webview_id: 1,
        }
    }

    pub fn next_webview_id(&mut self) -> u64 {
        let id = self.next_webview_id;
        self.next_webview_id += 1;
        id
    }
}

/// Initialize the crypto provider for Servo
pub fn init_crypto() {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Error initializing crypto provider");
}

/// Get the next available instance ID
fn next_instance_id() -> u64 {
    let mut id_counter = NEXT_INSTANCE_ID.lock().unwrap();
    let id = *id_counter;
    *id_counter += 1;
    id
}
