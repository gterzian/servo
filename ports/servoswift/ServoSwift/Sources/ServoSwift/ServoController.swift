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

/// Main controller class for managing Servo instances
public class ServoController {
    private var handle: ServoHandle = 0
    private var isInitialized = false
    
    /// Shared instance for global Servo management
    public static let shared = ServoController()
    
    private init() {}
    
    /// Initialize the Servo library (call once at app startup)
    public func initialize() throws {
        guard !isInitialized else { return }
        
        let result = servo_init()
        if result != ServoError.success.rawValue {
            throw ServoError(rawValue: result) ?? .unknownError
        }
        
        isInitialized = true
    }
    
    /// Create a new Servo instance for the given NSView
    public func createInstance(
        for view: NSView,
        options: ServoInitOptions = ServoInitOptions()
    ) throws -> ServoInstance {
        guard isInitialized else {
            throw ServoError.unknownError
        }
        
        let viewPointer = Unmanaged.passUnretained(view).toOpaque()
        let frame = view.frame
        let scale = Float(view.window?.backingScaleFactor ?? 1.0)
        
        var opts = options
        let handle = servo_create_instance(
            viewPointer,
            UInt32(frame.width),
            UInt32(frame.height),
            scale,
            &opts
        )
        
        if handle == 0 {
            throw ServoError.unknownError
        }
        
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
public class ServoInstance {
    let handle: ServoHandle
    private let view: NSView
    private var webviews: [WebViewHandle: ServoWebView] = [:]
    
    internal init(handle: ServoHandle, view: NSView) {
        self.handle = handle
        self.view = view
    }
    
    /// Create a new WebView within this Servo instance
    public func createWebView(url: URL? = nil) throws -> ServoWebView {
        let frame = view.frame
        let scale = Float(view.window?.backingScaleFactor ?? 1.0)
        
        let urlString = url?.absoluteString
        let webviewHandle = urlString?.withCString { cString in
            servo_create_webview(
                handle,
                cString,
                UInt32(frame.width),
                UInt32(frame.height),
                scale
            )
        } ?? servo_create_webview(
            handle,
            nil,
            UInt32(frame.width),
            UInt32(frame.height),
            scale
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
        let result = servo_spin_event_loop(handle)
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
        servo_destroy_instance(handle)
    }
}
