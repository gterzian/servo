/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//
//  ServoController.swift
//  ServoSwift
//
//  High-level Swift API for managing Servo instances
//

import Foundation
import AppKit

/// Main controller class for managing the single Servo instance
public class ServoController: @unchecked Sendable {
    private var handle: ServoHandle = 0
    private var isInitialized = false
    
    /// Shared instance for global Servo management
    @MainActor
    public static let shared = ServoController()
    
    private init() {}
    
    /// Create the Servo instance for the given NSView (call once at app startup)
    @MainActor
    public func createServo(
        for view: NSView,
        options: ServoInitOptions = ServoInitOptions()
    ) throws -> ServoInstance {
        guard !isInitialized else {
            throw ServoError.unknownError // Already initialized
        }
        
        let viewPointer = Unmanaged.passUnretained(view).toOpaque()
        let frame = view.frame
        let scale = Float(view.window?.backingScaleFactor ?? 1.0)
        
        var opts = options
        let handle = create_servo(
            nsview_ptr: viewPointer,
            width: UInt32(frame.width),
            height: UInt32(frame.height),
            scale_factor: scale,
            options: &opts
        )
        
        if handle == 0 {
            throw ServoError.unknownError
        }
        
        self.handle = handle
        self.isInitialized = true
        
        return ServoInstance(handle: handle, view: view)
    }
    
    /// Get the Servo version string
    public var version: String {
        let cString = servo_version()
        return String(cString: cString)
    }
    
    deinit {
        // Cleanup is handled by individual ServoInstance objects
    }
}

/// Represents a single Servo instance tied to an NSView
public class ServoInstance: @unchecked Sendable {
    let handle: ServoHandle
    private let view: NSView
    private var webviews: [WebViewHandle: ServoWebView] = [:]
    
    internal init(handle: ServoHandle, view: NSView) {
        self.handle = handle
        self.view = view
    }
    
    /// Create a new WebView within this Servo instance
    @MainActor
    public func createWebView(url: URL? = nil) throws -> ServoWebView {
        let frame = view.frame
        let scale = Float(view.window?.backingScaleFactor ?? 1.0)
        
        let urlString = url?.absoluteString
        let webviewHandle = urlString?.withCString { cString in
            create_webview(
                servo_handle: handle,
                url: cString,
                width: UInt32(frame.width),
                height: UInt32(frame.height),
                scale_factor: scale
            )
        } ?? create_webview(
            servo_handle: handle,
            url: nil,
            width: UInt32(frame.width),
            height: UInt32(frame.height),
            scale_factor: scale
        )
        
        if webviewHandle == 0 {
            throw ServoError.unknownError
        }
        
        let webview = ServoWebView(
            handle: webviewHandle,
            servoInstance: self,
            view: view
        )
        
        webviews[webviewHandle] = webview
        return webview
    }
    
    /// Spin the Servo event loop (should be called regularly)
    public func spinEventLoop() throws {
        let result = spin_event_loop(servo_handle: handle)
        if result != ServoError.success.rawValue {
            throw ServoError(rawValue: result) ?? .unknownError
        }
    }
    
    /// Remove a WebView from this instance
    internal func removeWebView(_ webview: ServoWebView) {
        webviews.removeValue(forKey: webview.handle)
    }
    
    deinit {
        // Clean up all WebViews first
        for webview in webviews.values {
            webview.destroy()
        }
        
        // Destroy the Servo instance
        _ = destroy_servo(servo_handle: handle)
    }
}
