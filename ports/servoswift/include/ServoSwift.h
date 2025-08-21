// Public C header for ServoSwift FFI

#ifndef ServoSwift_h
#define ServoSwift_h

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

void *create_servo(void *nsview_ptr, uint32_t width, uint32_t height, float scale_factor, void *options);
void *create_webview(void *servo_ptr, const char *url, uint32_t width, uint32_t height, float scale_factor);
int32_t webview_load_url(void *servo_ptr, void *webview_ptr, const char *url);
int32_t webview_resize(void *servo_ptr, void *webview_ptr, uint32_t width, uint32_t height);
bool webview_paint(void *servo_ptr, void *webview_ptr);
bool webview_present(void *servo_ptr);

// New: resize the underlying WindowRenderingContext surface
int32_t servo_resize_context(void *servo_ptr, uint32_t width, uint32_t height);

int32_t spin_event_loop(void *servo_ptr);
int32_t destroy_webview(void *servo_ptr, void *webview_ptr);
int32_t destroy_servo(void *servo_ptr);
const char *version(void);
int32_t servo_needs_repaint(void *servo_ptr);

#ifdef __cplusplus
}
#endif

#endif /* ServoSwift_h */
