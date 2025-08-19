/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//
//  main.swift
//  ServoSwiftExample
//
//  Example macOS app demonstrating ServoSwift usage
//

import AppKit
import ServoSwift

// AppDelegate for the example app
class AppDelegate: NSObject, NSApplicationDelegate {
    var window: NSWindow!
    var servoView: ServoView!
    
    func applicationDidFinishLaunching(_ notification: Notification) {
        setupWindow()
        loadInitialPage()
    }
    
    private func setupWindow() {
        window = NSWindow(
            contentRect: NSRect(x: 100, y: 100, width: 1024, height: 768),
            styleMask: [.titled, .closable, .resizable, .miniaturizable],
            backing: .buffered,
            defer: false
        )
        
        window.title = "ServoSwift Example - \(ServoController.shared.version)"
        window.center()
        window.makeKeyAndOrderFront(nil)
        
        // Create and configure ServoView
        servoView = ServoView(frame: window.contentView!.bounds)
        servoView.autoresizingMask = [.width, .height]
        servoView.delegate = self
        
        window.contentView?.addSubview(servoView)
    }
    
    private func loadInitialPage() {
        do {
            try servoView.load("https://demo.servo.org/experiments/twgl-tunnel/")
        } catch {
            print("Failed to load initial page: \(error)")
            // Fallback to a simple local page
            try? servoView.load("about:blank")
        }
    }
    
    func applicationShouldTerminateWhenLastWindowClosed(_ sender: NSApplication) -> Bool {
        return true
    }
}

// MARK: - ServoWebViewDelegate
extension AppDelegate: ServoWebViewDelegate {
    func webView(_ webView: ServoWebView, didStartLoadingURL url: URL) {
        print("Started loading: \(url)")
        window.title = "Loading... - ServoSwift Example"
    }
    
    func webView(_ webView: ServoWebView, didFinishLoadingURL url: URL) {
        print("Finished loading: \(url)")
        window.title = "ServoSwift Example - \(url.host ?? url.absoluteString)"
    }
    
    func webView(_ webView: ServoWebView, didFailToLoadURL url: URL, error: Error) {
        print("Failed to load \(url): \(error)")
        window.title = "Load Failed - ServoSwift Example"
    }
    
    func webView(_ webView: ServoWebView, didUpdateTitle title: String) {
        print("Page title updated: \(title)")
        window.title = "\(title) - ServoSwift Example"
    }
}

// Entry point
let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.run()
