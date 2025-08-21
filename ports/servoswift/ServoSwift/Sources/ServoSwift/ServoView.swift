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
    private var displayLink: CVDisplayLink?
    
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
        
        // Set up the display link for regular rendering
        setupDisplayLink()
    }
    
    private func setupDisplayLink() {
        var displayLink: CVDisplayLink?
        CVDisplayLinkCreateWithActiveCGDisplays(&displayLink)
        
        if let displayLink = displayLink {
            CVDisplayLinkSetOutputCallback(displayLink, { (displayLink, inNow, inOutputTime, flagsIn, flagsOut, displayLinkContext) -> CVReturn in
                let servoView = Unmanaged<ServoView>.fromOpaque(displayLinkContext!).takeUnretainedValue()
                servoView.renderFrame()
                return kCVReturnSuccess
            }, UnsafeMutableRawPointer(Unmanaged.passUnretained(self).toOpaque()))
            
            self.displayLink = displayLink
        }
    }
    
    public override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        
        if window != nil {
            try? initializeServo()
            CVDisplayLinkStart(displayLink!)
        } else {
            CVDisplayLinkStop(displayLink!)
            cleanupServo()
        }
    }
    
    private func initializeServo() throws {
        guard servoInstance == nil else { return }
        
        // Initialize Servo if needed
        try ServoController.shared.initialize()
        
        // Create Servo instance for this view
        servoInstance = try ServoController.shared.createInstance(for: self)
        
        // Create initial WebView
        webView = try servoInstance?.createWebView()
        webView?.delegate = delegate
    }
    
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
        print("DEBUG: renderFrame() called")
        DispatchQueue.main.async {
            guard let servoInstance = self.servoInstance else {
                print("DEBUG: No servo instance")
                return
            }
            
            // Check if Servo needs repaint (this also spins the event loop)
            if servo_needs_repaint(servoInstance.handle) {
                print("DEBUG: Servo needs repaint, painting...")
                
                // Paint the WebView
                do {
                    try self.webView?.paint()
                    print("DEBUG: webView.paint() succeeded")
                } catch {
                    print("DEBUG: webView.paint() error: \(error)")
                }
            } else {
                print("DEBUG: Servo doesn't need repaint")
            }
        }
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
        if let displayLink = displayLink {
            CVDisplayLinkStop(displayLink)
        }
        cleanupServo()
    }
}
