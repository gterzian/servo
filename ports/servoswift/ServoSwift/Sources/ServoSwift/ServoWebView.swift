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
    let handle: WebViewHandle
    private weak var servoInstance: ServoInstance?
    private weak var view: NSView?
    
    public weak var delegate: ServoWebViewDelegate?
    
    /// Current URL being displayed
    public private(set) var url: URL?
    
    /// Current page title
    public private(set) var title: String?
    
    /// Whether the page is currently loading
    public private(set) var isLoading: Bool = false
    
    internal init(handle: WebViewHandle, servoInstance: ServoInstance, view: NSView) {
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
            servo_webview_load_url(servoInstance.handle, handle, cString)
        }
        
        if result != ServoError.success.rawValue {
            throw ServoError(rawValue: result) ?? .unknownError
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
        
        let result = servo_webview_resize(
            servoInstance.handle,
            handle,
            UInt32(size.width),
            UInt32(size.height)
        )
        
        if result != ServoError.success.rawValue {
            throw ServoError(rawValue: result) ?? .unknownError
        }
    }
    
    /// Paint the WebView to its rendering context
    public func paint() throws {
        guard let servoInstance = servoInstance else {
            throw ServoError.invalidHandle
        }
        
        let result = servo_webview_paint(servoInstance.handle, handle)
        if result != ServoError.success.rawValue {
            throw ServoError(rawValue: result) ?? .unknownError
        }
    }
    
    /// Handle mouse events
    public func handleMouseEvent(
        type: MouseEventType,
        at point: NSPoint,
        button: Int32 = 0
    ) {
        guard let servoInstance = servoInstance else { return }
        
        servo_handle_mouse_event(
            servoInstance.handle,
            handle,
            type.rawValue,
            Float(point.x),
            Float(point.y),
            button
        )
    }
    
    /// Handle keyboard events
    public func handleKeyEvent(
        type: KeyEventType,
        keyCode: Int32,
        modifiers: Int32
    ) {
        guard let servoInstance = servoInstance else { return }
        
        servo_handle_key_event(
            servoInstance.handle,
            handle,
            type.rawValue,
            keyCode,
            modifiers
        )
    }
    
    /// Handle scroll events
    public func handleScrollEvent(
        deltaX: Float,
        deltaY: Float,
        at point: NSPoint
    ) {
        guard let servoInstance = servoInstance else { return }
        
        servo_handle_scroll_event(
            servoInstance.handle,
            handle,
            deltaX,
            deltaY,
            Float(point.x),
            Float(point.y)
        )
    }
    
    /// Destroy this WebView
    internal func destroy() {
        guard let servoInstance = servoInstance else { return }
        
        servo_destroy_webview(servoInstance.handle, handle)
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
