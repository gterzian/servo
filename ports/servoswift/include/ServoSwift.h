/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

#ifndef SERVO_SWIFT_BINDINGS_H
#define SERVO_SWIFT_BINDINGS_H

#include <stdint.h>
#include <stdbool.h>

/// Opaque handle for Servo instance
typedef uint64_t ServoHandle;

/// Opaque handle for WebView instance  
typedef uint64_t WebViewHandle;

/// Error codes for Swift interop
typedef enum {
    ServoError_Success = 0,
    ServoError_InvalidHandle = 1,
    ServoError_InvalidUrl = 2,
    ServoError_RenderingError = 3,
    ServoError_UnknownError = 4,
} ServoError;

/// Initialization options for Servo
typedef struct {
    bool enable_subpixel_text_antialiasing;
    bool enable_compositing_debug_overlay;
    const char *resources_dir_path;
} ServoInitOptions;

/// Create the Servo instance attached to an NSView
ServoHandle create_servo(
    void *nsview_ptr,
    uint32_t width,
    uint32_t height,
    float scale_factor,
    const ServoInitOptions *options
);

/// Create a new WebView within the Servo instance
WebViewHandle create_webview(
    ServoHandle servo_handle,
    const char *url,
    uint32_t width,
    uint32_t height,
    float scale_factor
);

/// Load a URL in an existing WebView
int32_t webview_load_url(
    ServoHandle servo_handle,
    WebViewHandle webview_handle,
    const char *url
);

/// Resize a WebView
int32_t webview_resize(
    ServoHandle servo_handle,
    WebViewHandle webview_handle,
    uint32_t width,
    uint32_t height
);

/// Paint a WebView to its rendering context
int32_t webview_paint(
    ServoHandle servo_handle,
    WebViewHandle webview_handle
);

/// Spin the Servo event loop
int32_t spin_event_loop(ServoHandle servo_handle);

/// Destroy a WebView
int32_t destroy_webview(
    ServoHandle servo_handle,
    WebViewHandle webview_handle
);

/// Destroy the Servo instance and all associated WebViews
int32_t destroy_servo(ServoHandle servo_handle);

/// Get the Servo version string
const char* version(void);

#endif /* SERVO_SWIFT_BINDINGS_H */
