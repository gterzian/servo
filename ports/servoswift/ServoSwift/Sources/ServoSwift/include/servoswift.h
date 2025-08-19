/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

#ifndef SERVO_SWIFT_BINDINGS_H
#define SERVO_SWIFT_BINDINGS_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Opaque handle for Servo instance
typedef uint64_t ServoHandle;

/// Opaque handle for WebView instance  
typedef uint64_t WebViewHandle;

/// Error codes for Swift interop
typedef enum {
    ServoSuccess = 0,
    ServoInvalidHandle = 1,
    ServoInvalidUrl = 2,
    ServoInitializationFailed = 3,
    ServoRenderingFailed = 4,
} ServoError;

/// Initialization options for Servo
typedef struct {
    bool enable_hardware_acceleration;
    bool enable_experimental_features;
    const char* user_agent;
    uint32_t cache_size_mb;
} ServoInitOptions;

/// Create a new Servo instance attached to an NSView
ServoHandle servo_create_instance(
    void* nsview_ptr,
    uint32_t width,
    uint32_t height,
    float scale_factor,
    const ServoInitOptions* options
);

/// Create a new WebView within a Servo instance
WebViewHandle servo_create_webview(
    ServoHandle servo_handle,
    const char* url,
    uint32_t width,
    uint32_t height,
    float scale_factor
);

/// Load a URL in an existing WebView
ServoError servo_webview_load_url(
    ServoHandle servo_handle,
    WebViewHandle webview_handle,
    const char* url
);

/// Resize a WebView
ServoError servo_webview_resize(
    ServoHandle servo_handle,
    WebViewHandle webview_handle,
    uint32_t width,
    uint32_t height
);

/// Paint a WebView to its rendering context
ServoError servo_webview_paint(
    ServoHandle servo_handle,
    WebViewHandle webview_handle
);

/// Spin the Servo event loop
ServoError servo_spin_event_loop(ServoHandle servo_handle);

/// Destroy a WebView
ServoError servo_destroy_webview(
    ServoHandle servo_handle,
    WebViewHandle webview_handle
);

/// Destroy a Servo instance and all associated WebViews
ServoError servo_destroy_instance(ServoHandle servo_handle);

#ifdef __cplusplus
}
#endif

#endif /* SERVO_SWIFT_BINDINGS_H */
