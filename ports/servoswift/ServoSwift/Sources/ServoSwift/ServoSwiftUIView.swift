/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//
//  ServoSwiftUIView.swift
//  ServoSwift
//
//  SwiftUI wrapper for Servo WebView
//

import SwiftUI
import AppKit
import Foundation

/// SwiftUI view that displays a Servo WebView
public struct ServoSwiftUIView: NSViewRepresentable {
    let url: URL
    let onLoadStart: ((URL) -> Void)?
    let onLoadFinish: ((URL) -> Void)?
    
    public init(
        url: URL,
        onLoadStart: ((URL) -> Void)? = nil,
        onLoadFinish: ((URL) -> Void)? = nil
    ) {
        self.url = url
        self.onLoadStart = onLoadStart
        self.onLoadFinish = onLoadFinish
    }
    
    public func makeNSView(context: Context) -> ServoNSView {
    // makeNSView called
        let view = ServoNSView()
        view.loadURL(url)
        view.onLoadStart = onLoadStart
        view.onLoadFinish = onLoadFinish
        return view
    }
    
    public func updateNSView(_ nsView: ServoNSView, context: Context) {
        // Update if URL changed
        if nsView.currentURL != url {
            nsView.loadURL(url)
        }
    }
}

/// NSView that wraps Servo functionality for SwiftUI
public class ServoNSView: NSView {
    fileprivate var servoInstance: ServoInstance?
    private var webView: ServoWebView?
    
    // Use layer-backed view and let AppKit call updateLayer() when needed.
    // `wantsLayer` is a mutable property on NSView and must not be overridden as a
    // read-only computed property. Instead, set it in the initializers so the
    // view is layer-backed at creation time.
    override public var wantsUpdateLayer: Bool {
        return true
    }
    
    public var currentURL: URL?
    public var onLoadStart: ((URL) -> Void)?
    public var onLoadFinish: ((URL) -> Void)?
    
    // Store pending URL to load once Servo is initialized
    private var pendingURL: URL?
    
    public override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
    // Make this view layer-backed.
    self.wantsLayer = true
    // Redraw layer contents when setNeedsDisplay is called.
    self.layerContentsRedrawPolicy = .onSetNeedsDisplay
    }
    
    public required init?(coder: NSCoder) {
        super.init(coder: coder)
    // Make this view layer-backed.
    self.wantsLayer = true
    // Redraw layer contents when setNeedsDisplay is called.
    self.layerContentsRedrawPolicy = .onSetNeedsDisplay
    }
    
    private var cancellables = Set<AnyCancellable>()
    
    public override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        
        if window != nil && servoInstance == nil {
            initializeServo()
        } else if window == nil {
            print("🧹 Window is nil, cleaning up...")
            cleanup()
        } else {
            print("ℹ️ Window present but Servo already initialized")
        }
    }
    
    private func initializeServo() {
        do {
            // Initialize Servo controller if needed
            try ServoController.shared.initialize()
            
            // Create Servo instance for this view
            let instance = try ServoController.shared.createInstance(for: self)
            self.servoInstance = instance
            // Servo instance created
            
                // Ensure the offscreen context matches our view size in pixels
            if let servoInstance = self.servoInstance {
                let scale = Float(self.window?.backingScaleFactor ?? 1.0)
                let pixelWidth = UInt32(self.bounds.size.width * CGFloat(scale))
                let pixelHeight = UInt32(self.bounds.size.height * CGFloat(scale))
                let _ = servo_resize_context(servoInstance.handle, pixelWidth, pixelHeight)
            }
            
            // Event loop is driven by embedder display notifications
            
            // Load pending URL if we have one
            if let pendingURL = pendingURL {
                self.pendingURL = nil
                loadURL(pendingURL)
            }
            
        } catch {
            print("❌ Failed to initialize Servo: \(error)")
            // Failed to initialize Servo
        }
    }
    
    public func loadURL(_ url: URL) {
        currentURL = url
        
        guard let servoInstance = servoInstance else {
            // Servo not ready, store pending URL
            pendingURL = url
            return
        }
    onLoadStart?(url)
        
        do {
            // Create or update WebView
            if webView == nil {
                webView = try servoInstance.createWebView(url: url)
                webView?.delegate = self
            } else {
                try webView?.load(url)
            }
        } catch {
            // Failed to load URL
        }
    }

    // When layer-backed and `wantsUpdateLayer` is true, AppKit will call
    // `updateLayer()` instead of `draw(_:)`. Update the layer with our
    // rendering work there.
    public override func updateLayer() {
    print("🖼️ ServoNSView.updateLayer() called")
    performRepaint()
    }

    // Intercept setNeedsDisplay so we can force the layer to update immediately
    // during diagnostics. This will catch calls from Rust -> objc -> Swift that
    // previously invoked `view.setNeedsDisplay(view.bounds)`.
    public override func setNeedsDisplay(_ invalidRect: NSRect) {
        super.setNeedsDisplay(invalidRect)
        print("🔔 ServoNSView.setNeedsDisplay(_:) called")
        // If this view is layer-backed, mark the layer as needing display and
        // attempt a synchronous display to exercise updateLayer().
        if let l = self.layer {
            l.setNeedsDisplay()
            l.displayIfNeeded()
        } else {
            // Fallback: force the view-level display immediately.
            self.displayIfNeeded()
        }
    }
    private func performRepaint() {
        guard let servoInstance = servoInstance else { return }

        if let webView = webView {
            do {
            
                // Paint WebRender into the offscreen FBO
                try webView.paint()
                // Present the offscreen context immediately after painting
                let _ = webview_present(servoInstance.handle)

                // Composite into parent and present
                self.paintParentSurface()
            } catch {
                // Update paint failed
            }
        }
    }
    
    public override func setFrameSize(_ newSize: NSSize) {
        super.setFrameSize(newSize)
        
        // Resize the WebView when the view size changes
        do {
            try webView?.resize(to: newSize)
            // Trigger a repaint after resize
        } catch {
            // Resize error occurred
        }
    }

    // Phase 2: composite offscreen into parent surface and present it.
    private func paintParentSurface() {
        guard let servoInstance = servoInstance else { return }

        let _ = servo_parent_prepare_for_rendering(servoInstance.handle)

        let _ = servo_render_offscreen_to_parent(servoInstance.handle)
        let _ = servo_parent_present(servoInstance.handle)
    // present completed
    }
    
    private func cleanup() {
        webView = nil
        servoInstance = nil
        cancellables.removeAll()
    }
    
    deinit {
        cleanup()
    }
}

// MARK: - ServoWebViewDelegate
extension ServoNSView: ServoWebViewDelegate {
    public func webView(_ webView: ServoWebView, didStartLoadingURL url: URL) {
        onLoadStart?(url)
    }
    
    public func webView(_ webView: ServoWebView, didFinishLoadingURL url: URL) {
        onLoadFinish?(url)
    }
    
    public func webView(_ webView: ServoWebView, didFailToLoadURL url: URL, error: Error) {
    }
    
    public func webView(_ webView: ServoWebView, didUpdateTitle title: String) {
        // Could update window title or send to parent
    }
}

// Add missing import for Combine
import Combine

// MARK: - Objective-C callbacks (called via objc_msgSend from Rust)
extension ServoNSView {
    @objc public func swift_notify_new_frame(_ ptr: UnsafeRawPointer?) {
        guard let ptr = ptr else { return }
        let view = Unmanaged<NSView>.fromOpaque(ptr).takeUnretainedValue()
        DispatchQueue.main.async {
            print("🔔 swift_notify_new_frame: scheduling setNeedsDisplay on main thread")
            view.setNeedsDisplay(view.bounds)
        }
    }

    @objc public func swift_wake_event_loop(_ ptr: UnsafeRawPointer?) {
        guard let ptr = ptr else { return }
        let view = Unmanaged<NSView>.fromOpaque(ptr).takeUnretainedValue()
        DispatchQueue.main.async {
            if let servoView = view as? ServoNSView, let instance = servoView.servoInstance {
                do {
                    try instance.spinEventLoop()
                } catch {
                    // spinEventLoop completed or threw
                }
            } else {
                // could not cast view to ServoNSView or no servoInstance
            }
        }
    }
}
