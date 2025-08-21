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
    private var isInitialized = false
    
    /// Shared instance for global Servo management
    public static let shared = ServoController()
    
    private init() {}
    
    /// Initialize the Servo library (call once at app startup)
    public func initialize() throws {
        print("🚀 ServoController: initialize() called")
        guard !isInitialized else { 
            print("ℹ️ ServoController: Already initialized, skipping")
            return 
        }
        
        // No initialization function needed in our current API
        isInitialized = true
        print("✅ ServoController: Initialization completed")
    }
    
    /// Create a new Servo instance for the given NSView
    public func createInstance(
        for view: NSView,
        options: ServoInitOptions = ServoInitOptions()
    ) throws -> ServoInstance {
        print("🏗️ ServoController: createInstance() called for view: \(view)")
        guard isInitialized else {
            print("❌ ServoController: Not initialized, cannot create instance")
            throw ServoError.unknownError
        }
        
        let viewPointer = Unmanaged.passUnretained(view).toOpaque()
        let frame = view.frame
        let scale = Float(view.window?.backingScaleFactor ?? 1.0)

        // Convert to physical pixels for Servo: width_in_pixels = points * backingScaleFactor
        let pixelWidth = UInt32(frame.width * CGFloat(scale))
        let pixelHeight = UInt32(frame.height * CGFloat(scale))
        print("📊 ServoController: Creating servo with - frame: \(frame), scale: \(scale), pixels: (\(pixelWidth), \(pixelHeight))")

        var opts = options
        let handle = create_servo(
            viewPointer,
            pixelWidth,
            pixelHeight,
            scale,
            &opts
        )
        
        if handle == nil {
            print("❌ ServoController: create_servo returned null handle")
            throw ServoError.unknownError
        }

        print("✅ ServoController: Servo instance created with handle: \(handle!)")
        return ServoInstance(handle: handle!, view: view)
    }
    
    /// Get the Servo version string
    public var version: String {
        let cString = ServoSwift.version()
        return String(cString: cString)
    }
    
    deinit {
        // Cleanup is handled by individual ServoInstance objects
    }
}

/// Represents a single Servo instance tied to an NSView
public class ServoInstance {
    let handle: UnsafeMutableRawPointer
    private let view: NSView
    private var webviews: [UnsafeMutableRawPointer: ServoWebView] = [:]
    
    internal init(handle: UnsafeMutableRawPointer, view: NSView) {
        print("🏛️ ServoInstance: init() called with handle: \(handle)")
        self.handle = handle
        self.view = view
    }
    
    /// Create a new WebView within this Servo instance
    public func createWebView(url: URL? = nil) throws -> ServoWebView {
        print("🕸️ ServoInstance: createWebView() called with URL: \(url?.absoluteString ?? "nil")")
    let frame = view.frame
    let scale = Float(view.window?.backingScaleFactor ?? 1.0)

    // Send pixel dimensions to Servo (points * scale)
    let pixelWidth = UInt32(frame.width * CGFloat(scale))
    let pixelHeight = UInt32(frame.height * CGFloat(scale))
    print("📊 ServoInstance: Creating webview with - frame: \(frame), scale: \(scale), pixels: (\(pixelWidth), \(pixelHeight))")
        
        let urlString = url?.absoluteString
        let viewPointer = Unmanaged.passUnretained(view).toOpaque()
        let webviewHandle = urlString?.withCString { cString in
            print("🔗 ServoInstance: Calling create_webview with C string: \(String(cString: cString))")
            return create_webview(
                handle,
                cString,
                pixelWidth,
                pixelHeight,
                scale,
                viewPointer
            )
        } ?? create_webview(
            handle,
            nil,
            pixelWidth,
            pixelHeight,
            scale,
            viewPointer
        )
        
        if webviewHandle == nil {
            print("❌ ServoInstance: create_webview returned null handle")
            throw ServoError.unknownError
        }
        
        print("✅ ServoInstance: WebView created with handle: \(webviewHandle!)")
        
        let webview = ServoWebView(
            handle: webviewHandle!,
            servoInstance: self,
            view: view
        )
        
        webviews[webviewHandle!] = webview
        print("📋 ServoInstance: WebView registered, total webviews: \(webviews.count)")
        return webview
    }
    
    /// Spin the Servo event loop (should be called regularly)
    public func spinEventLoop() throws {
        let result = spin_event_loop(handle)
        if result != ServoError.success {
            throw result
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
        _ = destroy_servo(handle)
    }
}
