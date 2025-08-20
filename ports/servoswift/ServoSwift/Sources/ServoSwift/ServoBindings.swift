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
    case renderingError = 3
    case unknownError = 4
    
    var localizedDescription: String {
        switch self {
        case .success:
            return "Success"
        case .invalidHandle:
            return "Invalid handle"
        case .invalidUrl:
            return "Invalid URL"
        case .renderingError:
            return "Rendering error"
        case .unknownError:
            return "Unknown error"
        }
    }
}

/// Servo initialization options
public struct ServoInitOptions {
    public var enable_subpixel_text_antialiasing: Bool = true
    public var enable_compositing_debug_overlay: Bool = false
    public var resources_dir_path: UnsafePointer<CChar>? = nil
    
    public init(
        enable_subpixel_text_antialiasing: Bool = true,
        enable_compositing_debug_overlay: Bool = false,
        resources_dir_path: UnsafePointer<CChar>? = nil
    ) {
        self.enable_subpixel_text_antialiasing = enable_subpixel_text_antialiasing
        self.enable_compositing_debug_overlay = enable_compositing_debug_overlay
        self.resources_dir_path = resources_dir_path
    }
}

/// C function declarations
@_silgen_name("create_servo")
func create_servo(
    _ nsview_ptr: UnsafeMutableRawPointer,
    _ width: UInt32,
    _ height: UInt32,
    _ scale_factor: Float,
    _ options: UnsafeMutablePointer<ServoInitOptions>?
) -> UnsafeMutableRawPointer?

@_silgen_name("create_webview")
func create_webview(
    _ servo_ptr: UnsafeMutableRawPointer,
    _ url: UnsafePointer<CChar>?,
    _ width: UInt32,
    _ height: UInt32,
    _ scale_factor: Float
) -> UnsafeMutableRawPointer?

@_silgen_name("webview_load_url")
func webview_load_url(
    _ servo_ptr: UnsafeMutableRawPointer,
    _ webview_ptr: UnsafeMutableRawPointer,
    _ url: UnsafePointer<CChar>
) -> ServoError

@_silgen_name("webview_resize")
func webview_resize(
    _ servo_ptr: UnsafeMutableRawPointer,
    _ webview_ptr: UnsafeMutableRawPointer,
    _ width: UInt32,
    _ height: UInt32
) -> ServoError

@_silgen_name("webview_paint")
func webview_paint(
    _ servo_ptr: UnsafeMutableRawPointer,
    _ webview_ptr: UnsafeMutableRawPointer
) -> ServoError

@_silgen_name("spin_event_loop")
func spin_event_loop(_ servo_ptr: UnsafeMutableRawPointer) -> ServoError

@_silgen_name("destroy_webview")
func destroy_webview(
    _ servo_ptr: UnsafeMutableRawPointer,
    _ webview_ptr: UnsafeMutableRawPointer
) -> ServoError

@_silgen_name("destroy_servo")
func destroy_servo(_ servo_ptr: UnsafeMutableRawPointer) -> ServoError

@_silgen_name("version")
func version() -> UnsafePointer<CChar>

/// Event handling functions (not implemented yet)
/// These would need to be added to the Rust bindings
