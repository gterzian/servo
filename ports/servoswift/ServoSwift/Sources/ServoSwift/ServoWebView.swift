/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//
//  ServoWebView.swift
//  ServoSwift
//
//  WebView wrapper for Servo integration
//

import Foundation
import AppKit

/// Protocol for WebView delegate callbacks
public protocol ServoWebViewDelegate: AnyObject {
    func webView(_ webView: ServoWebView, didStartLoadingURL url: URL)
    func webView(_ webView: ServoWebView, didFinishLoadingURL url: URL)
    func webView(_ webView: ServoWebView, didFailToLoadURL url: URL, error: Error)
    func webView(_ webView: ServoWebView, didUpdateTitle title: String)
}

/// A WebView backed by Servo
public class ServoWebView: NSObject {
    let handle: UnsafeMutableRawPointer
    private weak var servoInstance: ServoInstance?
    private weak var view: NSView?
    
    public weak var delegate: ServoWebViewDelegate?
    
    /// Current URL being displayed
    public private(set) var url: URL?
    
    /// Current page title
    public private(set) var title: String?
    
    /// Whether the page is currently loading
    public private(set) var isLoading: Bool = false
    
    internal init(handle: UnsafeMutableRawPointer, servoInstance: ServoInstance, view: NSView) {
        self.handle = handle
        self.servoInstance = servoInstance
        self.view = view
        super.init()
    }
    
    /// Load a URL in this WebView
    public func load(_ url: URL) throws {
        guard let servoInstance = servoInstance else {
            throw ServoError.invalidHandle
        }
        
        let result = url.absoluteString.withCString { cString in
            return webview_load_url(servoInstance.handle, handle, cString)
        }
        
        if result != ServoError.success {
            throw result
        }
        
        self.url = url
        self.isLoading = true
        delegate?.webView(self, didStartLoadingURL: url)
    }
    
    /// Resize the WebView
    public func resize(to size: NSSize) throws {
        guard let servoInstance = servoInstance else {
            throw ServoError.invalidHandle
        }
        // Convert to physical pixels using the view/window backing scale
        let scale = Float(view?.window?.backingScaleFactor ?? 1.0)
        let pixelWidth = UInt32(size.width * CGFloat(scale))
        let pixelHeight = UInt32(size.height * CGFloat(scale))

        let result = webview_resize(
            servoInstance.handle,
            handle,
            pixelWidth,
            pixelHeight
        )
        
        if result != ServoError.success {
            throw result
        }
    }
    
    /// Paint the WebView to its rendering context
    public func paint() throws {
        print("🎨 Swift ServoWebView.paint() called")
        guard let servoInstance = servoInstance else {
            print("❌ Swift ServoWebView.paint() - no servoInstance")
            throw ServoError.invalidHandle
        }
        
        print("🔧 Swift ServoWebView.paint() - calling webview_paint with handles: servo=\(servoInstance.handle), webview=\(handle)")
        let result = webview_paint(servoInstance.handle, handle)
        print("📊 Swift ServoWebView.paint() - webview_paint returned: \(result ? "success" : "failure")")
        if !result {
            print("❌ Swift ServoWebView.paint() - error: paint failed")
            throw ServoError.renderingError
        }
        print("✅ Swift ServoWebView.paint() - completed successfully")
    }
    
    /// Handle mouse events
    public func handleMouseEvent(
        type: MouseEventType,
        at point: NSPoint,
        button: Int32 = 0
    ) {
        guard servoInstance != nil else { return }
        
        // Mouse events not implemented in current API
        // servo_handle_mouse_event would need to be added to Rust bindings
    }
    
    /// Handle keyboard events
    public func handleKeyEvent(
        type: KeyEventType,
        keyCode: Int32,
        modifiers: Int32
    ) {
        guard servoInstance != nil else { return }
        
        // Key events not implemented in current API
        // servo_handle_key_event would need to be added to Rust bindings
    }
    
    /// Handle scroll events
    public func handleScrollEvent(
        deltaX: Float,
        deltaY: Float,
        at point: NSPoint
    ) {
        guard servoInstance != nil else { return }
        
        // Scroll events not implemented in current API
        // servo_handle_scroll_event would need to be added to Rust bindings
    }
    
    /// Destroy this WebView
    internal func destroy() {
        guard let servoInstance = servoInstance else { return }
        
        _ = destroy_webview(servoInstance.handle, handle)
        servoInstance.removeWebView(self)
    }
    
    deinit {
        destroy()
    }
}

/// Mouse event types
public enum MouseEventType: Int32 {
    case mouseDown = 0
    case mouseUp = 1
    case mouseMove = 2
    case mouseDrag = 3
}

/// Keyboard event types
public enum KeyEventType: Int32 {
    case keyDown = 0
    case keyUp = 1
}
