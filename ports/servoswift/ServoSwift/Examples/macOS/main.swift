/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//
//  main.swift
//  ServoSwiftExample
//
//  Example macOS app demonstrating ServoSwift usage
//

import SwiftUI
import ServoSwift
import AppKit
import Dispatch

// Use a DispatchSource-based SIGINT handler. This is signal-safe and
// integrates with GCD; it will call NSApplication.shared.terminate(nil)
// on the main queue when Ctrl-C is received in the terminal.
func installSigintHandler() {
    let sigintSource = DispatchSource.makeSignalSource(signal: SIGINT, queue: .main)
    sigintSource.setEventHandler {
    print("SIGINT received: terminating app")
    NSApplication.shared.terminate(nil)
    }
    // Must call signal() to ensure the signal is not ignored by the system
    sigintSource.resume()
}

// Main SwiftUI app
@main
struct ServoSwiftExampleApp: App {
    init() {
        installSigintHandler()
    }
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}

struct ContentView: View {
    @State private var url = URL(string: "https://servo.org")!
    @State private var isLoading = false
    
    var body: some View {
        VStack {
            HStack {
                TextField("URL", text: Binding(
                    get: { url.absoluteString },
                    set: { if let newURL = URL(string: $0) { url = newURL } }
                ))
                .textFieldStyle(RoundedBorderTextFieldStyle())
                
                Button("Go") {
                    // URL will update automatically via the binding
                }
                .disabled(isLoading)
            }
            .padding()
            
            ServoSwiftUIView(
                url: url,
                onLoadStart: { url in
                    print("Started loading: \(url)")
                    isLoading = true
                },
                onLoadFinish: { url in
                    print("Finished loading: \(url)")
                    isLoading = false
                }
            )
            .frame(minWidth: 800, minHeight: 600)
        }
        .navigationTitle("ServoSwift Example - \(url)")
    }
}
