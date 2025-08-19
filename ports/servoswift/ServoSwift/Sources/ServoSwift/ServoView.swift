/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//
//  ServoView.swift
//  ServoSwift
//
//  NSView subclass for hosting Servo WebViews
//

import AppKit
import Foundation

/// An NSView that hosts a Servo WebView
public class ServoView: NSView {
    private var servoInstance: ServoInstance?
    private var webView: ServoWebView?
    private var renderTimer: Timer?
    
    /// The delegate for WebView events
    public weak var delegate: ServoWebViewDelegate? {
        didSet {
            webView?.delegate = delegate
        }
    }
    
    /// The current URL being displayed
    public var url: URL? {
        return webView?.url
    }
    
    /// Whether the page is currently loading
    public var isLoading: Bool {
        return webView?.isLoading ?? false
    }
    
    public override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        setupView()
    }
    
    public required init?(coder: NSCoder) {
        super.init(coder: coder)
        setupView()
    }
    
    private func setupView() {
        wantsLayer = true
        
        // Set up the render timer for regular rendering
        setupRenderTimer()
    }
    
    private func setupRenderTimer() {
        // Create a timer that fires 60 times per second
        renderTimer = Timer.scheduledTimer(withTimeInterval: 1.0/60.0, repeats: true) { [weak self] _ in
            Task { @MainActor in
                self?.renderFrame()
            }
        }
    }
    
    public override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        
        if window != nil {
            try? initializeServo()
            renderTimer?.fire()
        } else {
            renderTimer?.invalidate()
            cleanupServo()
        }
    }
    
    private func initializeServo() throws {
        guard servoInstance == nil else { return }
        
        // Create the single Servo instance for this view
        servoInstance = try ServoController.shared.createServo(for: self)
        
        // Create initial WebView
        webView = try servoInstance?.createWebView()
        webView?.delegate = delegate
    }
    
    @MainActor
    private func cleanupServo() {
        webView = nil
        servoInstance = nil
    }
    
    /// Load a URL in the WebView
    public func load(_ url: URL) throws {
        if webView == nil {
            try initializeServo()
        }
        try webView?.load(url)
    }
    
    /// Load a URL string in the WebView
    public func load(_ urlString: String) throws {
        guard let url = URL(string: urlString) else {
            throw ServoError.invalidUrl
        }
        try load(url)
    }
    
    public override func setFrameSize(_ newSize: NSSize) {
        super.setFrameSize(newSize)
        try? webView?.resize(to: newSize)
    }
    
    private func renderFrame() {
        // Timer callback runs on main queue, so no dispatch needed
        // Spin the Servo event loop
        try? servoInstance?.spinEventLoop()
        
        // Paint the WebView
        try? webView?.paint()
    }
    
    // MARK: - Event Handling
    
    public override func mouseDown(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)
        webView?.handleMouseEvent(type: .mouseDown, at: point, button: Int32(event.buttonNumber))
    }
    
    public override func mouseUp(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)
        webView?.handleMouseEvent(type: .mouseUp, at: point, button: Int32(event.buttonNumber))
    }
    
    public override func mouseMoved(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)
        webView?.handleMouseEvent(type: .mouseMove, at: point)
    }
    
    public override func mouseDragged(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)
        webView?.handleMouseEvent(type: .mouseDrag, at: point, button: Int32(event.buttonNumber))
    }
    
    public override func scrollWheel(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)
        webView?.handleScrollEvent(
            deltaX: Float(event.deltaX),
            deltaY: Float(event.deltaY),
            at: point
        )
    }
    
    public override func keyDown(with event: NSEvent) {
        webView?.handleKeyEvent(
            type: .keyDown,
            keyCode: Int32(event.keyCode),
            modifiers: Int32(event.modifierFlags.rawValue)
        )
    }
    
    public override func keyUp(with event: NSEvent) {
        webView?.handleKeyEvent(
            type: .keyUp,
            keyCode: Int32(event.keyCode),
            modifiers: Int32(event.modifierFlags.rawValue)
        )
    }
    
    public override var acceptsFirstResponder: Bool {
        return true
    }
    
    deinit {
        // Just let the timer be deallocated naturally
        // Don't try to access properties in deinit due to Sendable issues
    }
}
