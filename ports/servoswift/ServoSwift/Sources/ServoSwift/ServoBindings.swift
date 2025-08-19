/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//
//  ServoBindings.swift
//  ServoSwift
//
//  Low-level C bindings for Servo integration
//

import Foundation

/// Error types that can be returned from Servo operations
public enum ServoError: Int32, Error {
    case success = 0
    case invalidHandle = 1
    case invalidUrl = 2
    case renderingContextError = 3
    case unknownError = 4
    
    var localizedDescription: String {
        switch self {
        case .success:
            return "Success"
        case .invalidHandle:
            return "Invalid handle"
        case .invalidUrl:
            return "Invalid URL"
        case .renderingContextError:
            return "Rendering context error"
        case .unknownError:
            return "Unknown error"
        }
    }
}

/// Servo initialization options
public struct ServoInitOptions {
    public var enableWebGPU: Bool = true
    public var enableWebXR: Bool = false
    public var logLevel: Int32 = 2
    
    public init(enableWebGPU: Bool = true, enableWebXR: Bool = false, logLevel: Int32 = 2) {
        self.enableWebGPU = enableWebGPU
        self.enableWebXR = enableWebXR
        self.logLevel = logLevel
    }
}

/// Opaque handle types for C interop
public typealias ServoHandle = UInt64
public typealias WebViewHandle = UInt64

/// C function declarations
@_silgen_name("create_servo")
func create_servo(
    nsview_ptr: UnsafeMutableRawPointer,
    width: UInt32,
    height: UInt32,
    scale_factor: Float,
    options: UnsafePointer<ServoInitOptions>?
) -> ServoHandle

@_silgen_name("create_webview")
func create_webview(
    servo_handle: ServoHandle,
    url: UnsafePointer<CChar>?,
    width: UInt32,
    height: UInt32,
    scale_factor: Float
) -> WebViewHandle

@_silgen_name("webview_load_url")
func webview_load_url(
    servo_handle: ServoHandle,
    webview_handle: WebViewHandle,
    url: UnsafePointer<CChar>
) -> Int32

@_silgen_name("webview_resize")
func webview_resize(
    servo_handle: ServoHandle,
    webview_handle: WebViewHandle,
    width: UInt32,
    height: UInt32
) -> Int32

@_silgen_name("webview_paint")
func webview_paint(
    servo_handle: ServoHandle,
    webview_handle: WebViewHandle
) -> Int32

@_silgen_name("spin_event_loop")
func spin_event_loop(servo_handle: ServoHandle) -> Int32

@_silgen_name("destroy_webview")
func destroy_webview(
    servo_handle: ServoHandle,
    webview_handle: WebViewHandle
) -> Int32

@_silgen_name("destroy_servo")
func destroy_servo(servo_handle: ServoHandle) -> Int32

@_silgen_name("version")
func servo_version() -> UnsafePointer<CChar>

/// Event handling functions
@_silgen_name("servo_handle_mouse_event")
func servo_handle_mouse_event(
    servo_handle: ServoHandle,
    webview_handle: WebViewHandle,
    event_type: Int32,
    x: Float,
    y: Float,
    button: Int32
)

@_silgen_name("servo_handle_key_event")
func servo_handle_key_event(
    servo_handle: ServoHandle,
    webview_handle: WebViewHandle,
    event_type: Int32,
    key_code: Int32,
    modifiers: Int32
)

@_silgen_name("servo_handle_scroll_event")
func servo_handle_scroll_event(
    servo_handle: ServoHandle,
    webview_handle: WebViewHandle,
    delta_x: Float,
    delta_y: Float,
    x: Float,
    y: Float
)
