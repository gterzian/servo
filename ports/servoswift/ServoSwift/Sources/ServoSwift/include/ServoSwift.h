/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

#ifndef SERVO_SWIFT_BINDINGS_H
#define SERVO_SWIFT_BINDINGS_H

#include <stdint.h>
#include <stdbool.h>

/// Error codes that can be returned from Servo operations
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

/// Create the single Servo instance and return an opaque pointer to it
void* create_servo(
    void *nsview_ptr,
    uint32_t width,
    uint32_t height,
    float scale_factor,
    ServoInitOptions *options
);

/// Create a WebView within the Servo instance and return an opaque pointer to it
void* create_webview(
    void *servo_ptr,
    const char *url,
    uint32_t width,
    uint32_t height,
    float scale_factor
);

/// Load a URL in an existing WebView
ServoError webview_load_url(
    void *servo_ptr,
    void *webview_ptr,
    const char *url
);

/// Resize a WebView
ServoError webview_resize(
    void *servo_ptr,
    void *webview_ptr,
    uint32_t width,
    uint32_t height
);

/// Paint a WebView to its surface
ServoError webview_paint(
    void *servo_ptr,
    void *webview_ptr
);

/// Spin the event loop for the Servo instance
ServoError spin_event_loop(void *servo_ptr);

/// Destroy a WebView (Swift should call this in WebView deinit)
ServoError destroy_webview(
    void *servo_ptr,
    void *webview_ptr
);

/// Destroy the Servo instance (Swift should call this in ServoInstance deinit)
ServoError destroy_servo(void *servo_ptr);

/// Get the version string
const char* version(void);

/// Get a render callback function that can blit the Servo framebuffer to a parent OpenGL context
typedef void (*RenderCallback)(const void *gl_context, int32_t x, int32_t y, int32_t width, int32_t height);

/// Get a render callback function
RenderCallback get_render_callback(void *servo_ptr);

#endif /* SERVO_SWIFT_BINDINGS_H */
