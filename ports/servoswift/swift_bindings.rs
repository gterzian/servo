/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! C FFI bindings for Swift integration
//!
//! This module provides a C-compatible interface that can be called from Swift.
//! All Servo instances and WebViews are managed on the Swift side as opaque pointers.

use std::ffi::{CStr, c_char, c_void};
use std::rc::Rc;

use url::Url;
use log::{warn, error, debug};
use dpi::PhysicalSize;
use euclid::{Box2D, Size2D, Scale};
use euclid::Point2D;
use webrender_api::units::DeviceIntRect;
use servo::{Servo, ServoBuilder, WebView, WebViewBuilder, WindowRenderingContext, RenderingContext, OffscreenRenderingContext, WebViewDelegate, EventLoopWaker};
use servo::servo_url::ServoUrl;
use raw_window_handle::{DisplayHandle, WindowHandle};

use crate::rendering_context::create_raw_handles_from_nsview;
use crate::init_crypto;
use gleam::gl;

/// Actual Servo data that Swift will hold as opaque pointer
struct ServoInstance {
    servo: Servo,
    // The rendering_context exposed to Servo (may be an OffscreenRenderingContext wrapped
    // as a trait object so the compositor targets an offscreen FBO).
    rendering_context: Rc<dyn RenderingContext>,
    // Keep the parent window rendering context so Swift can prepare/present the real window
    // surface when compositing the offscreen framebuffer.
    parent_context: Option<Rc<WindowRenderingContext>>,
    // Keep a reference to the offscreen context if we created one so we can call its
    // render_to_parent_callback from the FFI.
    offscreen_context: Option<Rc<OffscreenRenderingContext>>,
}

/// Actual WebView data  
struct WebViewInstance {
    webview: WebView,
}

/// Opaque handle that Swift will manage (not used on Rust side)
pub type ServoHandle = u64;
/// Opaque handle that Swift will manage (not used on Rust side)  
pub type WebViewHandle = u64;

// Instead of relying on global C symbols, message the Swift NSView directly
// using the Objective-C runtime. The Swift side exposes @objc methods on the
// NSView subclass (see ServoNSView.swift) named `swift_notify_new_frame` and
// `swift_wake_event_loop`. We look up the selector and call objc_msgSend.
unsafe extern "C" {
    fn sel_registerName(name: *const c_char) -> *const c_void;
    // objc_msgSend is variadic; declare it as an untyped symbol and transmute at call sites.
    fn objc_msgSend();
}

fn call_objc_with_ptr(nsview_ptr: *mut c_void, sel_name: &str, arg: *mut c_void) {
    use std::ffi::CString;
    let cstr = CString::new(sel_name).unwrap();
    unsafe {
        let sel = sel_registerName(cstr.as_ptr());
        // Transmute objc_msgSend to a function pointer with signature (id, SEL, void*) -> ()
        let f: extern "C" fn(*mut c_void, *const c_void, *mut c_void) =
            std::mem::transmute(objc_msgSend as *const ());
        f(nsview_ptr, sel, arg);
    }
}

// A minimal WebViewDelegate that forwards notify_new_frame_ready to the Swift NSView.
struct SwiftWebViewDelegate {
    nsview_ptr: *mut c_void,
}

impl WebViewDelegate for SwiftWebViewDelegate {
    fn notify_new_frame_ready(&self, _webview: WebView) {
        println!("servoswift: SwiftWebViewDelegate::notify_new_frame_ready called; nsview_ptr={:p}", self.nsview_ptr);
        if !self.nsview_ptr.is_null() {
            call_objc_with_ptr(self.nsview_ptr, "swift_notify_new_frame:", self.nsview_ptr)
        }
    }
}

// A minimal EventLoopWaker implementation that forwards wake() to Swift.
struct SwiftEventLoopWaker {
    // Store as usize so the struct is Send across threads.
    nsview_ptr: usize,
}

impl EventLoopWaker for SwiftEventLoopWaker {
    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(SwiftEventLoopWaker { nsview_ptr: self.nsview_ptr })
    }

    fn wake(&self) {
        if self.nsview_ptr != 0 {
            let ptr = self.nsview_ptr as *mut c_void;
            println!("servoswift: SwiftEventLoopWaker::wake() calling swift_wake_event_loop with nsview_ptr={:p}", ptr);
            call_objc_with_ptr(ptr, "swift_wake_event_loop:", ptr)
        } else {
            println!("servoswift: SwiftEventLoopWaker::wake() called but nsview_ptr == 0");
        }
    }
}

/// Error codes that can be returned from Servo operations
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum ServoError {
    Success = 0,
    InvalidHandle = 1,
    InvalidUrl = 2,
    RenderingError = 3,
    UnknownError = 4,
}

/// Initialization options for Servo
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ServoInitOptions {
    pub enable_subpixel_text_antialiasing: bool,
    pub enable_compositing_debug_overlay: bool,
    pub resources_dir_path: *const c_char,
}

impl Default for ServoInitOptions {
    fn default() -> Self {
        Self {
            enable_subpixel_text_antialiasing: true,
            enable_compositing_debug_overlay: false,
            resources_dir_path: std::ptr::null(),
        }
    }
}

/// Create the single Servo instance and return an opaque pointer to it
/// This handles all initialization and should only be called once
#[unsafe(no_mangle)]
pub extern "C" fn create_servo(
    nsview_ptr: *mut c_void,
    width: u32,
    height: u32,
    scale_factor: f32,
    options: *mut ServoInitOptions,
) -> *mut c_void {
    if nsview_ptr.is_null() {
        warn!("NSView pointer is null");
        return std::ptr::null_mut();
    }

    // Initialize logging, crypto, and resources - only happens once due to Once
    env_logger::init();
    init_crypto();
    crate::resources::init();

    let _servo_options = if options.is_null() {
        ServoInitOptions::default()
    } else {
        unsafe { *options }
    };

    // Create raw window handles from the NSView
    let (display_handle, window_handle) = match create_raw_handles_from_nsview(nsview_ptr) {
        Ok(handles) => handles,
        Err(e) => {
            error!("Failed to create raw window handles: {:?}", e);
            return std::ptr::null_mut();
        }
    };

    // Create the window rendering context
    let size = PhysicalSize::new(width, height);
    
    // Create the WindowRenderingContext (the real surfman-backed window surface)
    let window_rendering_context: Rc<WindowRenderingContext> = {
        let display_handle = unsafe { DisplayHandle::borrow_raw(display_handle) };
        let window_handle = unsafe { WindowHandle::borrow_raw(window_handle) };

        match WindowRenderingContext::new(display_handle, window_handle, size) {
            Ok(ctx) => Rc::new(ctx),
            Err(e) => {
                error!("Failed to create window rendering context: {:?}", e);
                return std::ptr::null_mut();
            }
        }
    };

    // Make sure the gl context is made current.
        window_rendering_context
            .make_current()
            .expect("Could not make window RenderingContext current");

    // Create an OffscreenRenderingContext that targets the window as its parent. This mirrors
    // the pattern used by the headed minibrowser: we render into an offscreen FBO and then
    // the embedder is responsible for compositing the offscreen FBO into the window surface.
    let offscreen_rc: Rc<OffscreenRenderingContext> = Rc::new(window_rendering_context.offscreen_context(size));

    // Use the offscreen context as the RenderingContext trait object for Servo's builder.
    let rendering_context: Rc<dyn RenderingContext> = offscreen_rc.clone();

    // Create Servo builder with the offscreen rendering context
    // Provide an EventLoopWaker that forwards wake() to Swift via the nsview_ptr.
    let waker_box: Box<dyn EventLoopWaker> = Box::new(SwiftEventLoopWaker { nsview_ptr: nsview_ptr as usize });
    let builder = ServoBuilder::new(rendering_context.clone()).event_loop_waker(waker_box);
    
    // Set up resource directory if provided
    // TODO: Use options.resources_dir_path
    
    // Create Servo instance
    let servo = builder.build();

    let instance = ServoInstance {
        servo,
        rendering_context,
        parent_context: Some(window_rendering_context.clone()),
        offscreen_context: Some(offscreen_rc.clone()),
    };

    let instance_ptr = Box::into_raw(Box::new(instance)) as *mut c_void;
    instance_ptr
}

/// Create a WebView within the Servo instance and return an opaque pointer to it
#[unsafe(no_mangle)]
pub extern "C" fn create_webview(
    servo_ptr: *mut c_void,
    url: *const c_char,
    width: u32,
    height: u32,
    scale_factor: f32,
    // Opaque pointer to the Swift NSView (or owner) that will receive frame-ready callbacks.
    delegate_ptr: *mut c_void,
) -> *mut c_void {
    assert!(!servo_ptr.is_null());

    let servo_instance = unsafe { &*(servo_ptr as *const ServoInstance) };

    let url_str = if url.is_null() {
        "about:blank"
    } else {
        unsafe {
            match CStr::from_ptr(url).to_str() {
                Ok(s) => s,
                Err(_) => {
                    error!("Invalid URL string");
                    return std::ptr::null_mut();
                }
            }
        }
    };

    let parsed_url = match ServoUrl::parse(url_str) {
        Ok(url) => url,
        Err(e) => {
            error!("Failed to parse URL '{}': {:?}", url_str, e);
            return std::ptr::null_mut();
        }
    };

    // Create WebView with the specified size
    let size = PhysicalSize::new(width, height);
    let rect = Box2D::from_size(Size2D::new(size.width as f32, size.height as f32));

    // Create a minimal delegate that forwards notify_new_frame_ready to Swift.
    let delegate_rc: std::rc::Rc<dyn WebViewDelegate> = std::rc::Rc::new(SwiftWebViewDelegate {
        nsview_ptr: delegate_ptr,
    });

    let webview = WebViewBuilder::new(&servo_instance.servo)
        .url(parsed_url.clone().into_url())
        .hidpi_scale_factor(Scale::new(scale_factor))
        .delegate(delegate_rc)
        .build();

    // Resize the WebView to the specified dimensions
    webview.move_resize(rect);

    // Navigate to the URL
    webview.load(parsed_url.into_url());
    
    let webview_instance = WebViewInstance { webview };
    let webview_ptr = Box::into_raw(Box::new(webview_instance)) as *mut c_void;

    webview_ptr
}

/// Load a URL in an existing WebView
#[unsafe(no_mangle)]
pub extern "C" fn webview_load_url(
    _servo_ptr: *mut c_void,
    webview_ptr: *mut c_void,
    url: *const c_char,
) -> ServoError {
    assert!(!webview_ptr.is_null());
    if webview_ptr.is_null() {
        return ServoError::InvalidHandle;
    }

    if url.is_null() {
        return ServoError::InvalidUrl;
    }

    let url_str = unsafe {
        match CStr::from_ptr(url).to_str() {
            Ok(s) => s,
            Err(_) => return ServoError::InvalidUrl,
        }
    };

    let parsed_url = match Url::parse(url_str) {
        Ok(url) => url,
        Err(_) => return ServoError::InvalidUrl,
    };

    let webview_instance = unsafe { &*(webview_ptr as *mut WebViewInstance) };
    webview_instance.webview.load(parsed_url);

    ServoError::Success
}

/// Resize a WebView
#[unsafe(no_mangle)]
pub extern "C" fn webview_resize(
    _servo_ptr: *mut c_void,
    webview_ptr: *mut c_void,
    width: u32,
    height: u32,
) -> ServoError {
    if webview_ptr.is_null() {
        return ServoError::InvalidHandle;
    }

    let webview_instance = unsafe { &*(webview_ptr as *const WebViewInstance) };
    
    // Create new size and resize the webview
    let size = PhysicalSize::new(width, height);
    let rect = Box2D::from_size(Size2D::new(size.width as f32, size.height as f32));
    webview_instance.webview.move_resize(rect);

    ServoError::Success
}

/// Resize the underlying WindowRenderingContext surface for this Servo instance.
#[unsafe(no_mangle)]
pub extern "C" fn servo_resize_context(
    servo_ptr: *mut c_void,
    width: u32,
    height: u32,
) -> ServoError {
    if servo_ptr.is_null() {
        return ServoError::InvalidHandle;
    }

    let servo_instance = unsafe { &*(servo_ptr as *const ServoInstance) };
    let size = PhysicalSize::new(width, height);
    servo_instance.rendering_context.resize(size);
    ServoError::Success
}

/// Save the current backbuffer to a PNG at the given path for debugging.
#[unsafe(no_mangle)]
pub extern "C" fn servo_save_screenshot(
    servo_ptr: *mut c_void,
    path: *const c_char,
) -> ServoError {
    if servo_ptr.is_null() || path.is_null() {
        return ServoError::InvalidHandle;
    }

    let servo_instance = unsafe { &*(servo_ptr as *const ServoInstance) };
    let c_str = unsafe { CStr::from_ptr(path) };
    let path_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return ServoError::InvalidUrl,
    };

    // Read the full size of the rendering context
    let size = servo_instance.rendering_context.size();
    let rect = DeviceIntRect::from_origin_and_size(
        Point2D::origin(),
        Size2D::new(size.width as i32, size.height as i32),
    );

    // Ensure the GL context is current and prepared for readback
    if let Err(e) = servo_instance.rendering_context.make_current() {
        error!("servo_save_screenshot: make_current failed: {:?}", e);
        // continue and try readback anyway
    } else {
        // prepare_for_rendering binds the framebuffer so readback reads the right buffer
        servo_instance.rendering_context.prepare_for_rendering();
    }

    debug!("servo_save_screenshot: attempting read_to_image for size={:?} rect={:?}", size, rect);

    match servo_instance.rendering_context.read_to_image(rect) {
        Some(img) => {
            debug!("servo_save_screenshot: read_to_image returned image: {}x{}", img.width(), img.height());

            // Quick pixel inspection: count non-black pixels and find first sample
            let mut non_black = 0usize;
            let mut first_sample: Option<(usize, usize, [u8;4])> = None;
            for (y, row) in img.rows().enumerate() {
                for (x, px) in row.enumerate() {
                    let data = px.0;
                    if data[0] != 0 || data[1] != 0 || data[2] != 0 || data[3] != 0 {
                        non_black += 1;
                        if first_sample.is_none() {
                            first_sample = Some((x, y, data));
                        }
                    }
                }
            }

            println!("servo_save_screenshot: non_black_pixels={} first_sample={:?}", non_black, first_sample.map(|(x,y,px)| (x,y,px)));

            // img is an RgbaImage; write as PNG
            if let Err(e) = img.save(path_str) {
                error!("Failed to save screenshot: {:?}", e);
                return ServoError::UnknownError;
            }
            ServoError::Success
        }
        None => {
            error!("servo_save_screenshot: read_to_image returned None");
            ServoError::UnknownError
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn webview_paint(
    servo_ptr: *mut c_void,
    webview_ptr: *mut c_void,
) -> bool {
    assert!(!(servo_ptr.is_null() || webview_ptr.is_null()));
    
    let servo_instance = unsafe { &*(servo_ptr as *const ServoInstance) };
    let webview_instance = unsafe { &*(webview_ptr as *const WebViewInstance) };
    
    let result = webview_instance.webview.paint();
    
    // Don't call present() here - let Swift call it in draw(_:)
    
    result
}

/// Fill the offscreen framebuffer with a solid RGBA color (0-255 each channel).
/// Useful as a deterministic visual test: call this, then composite+present and you
/// should see a solid block of the requested color in the view (if compositing is
/// working and the host isn't occluding the GL surface).
#[unsafe(no_mangle)]
pub extern "C" fn servo_fill_offscreen_solid_color(
    servo_ptr: *mut c_void,
    r: u8,
    g: u8,
    b: u8,
    a: u8,
) -> ServoError {
    if servo_ptr.is_null() {
        return ServoError::InvalidHandle;
    }

    let servo_instance = unsafe { &*(servo_ptr as *const ServoInstance) };
    let Some(offscreen) = servo_instance.offscreen_context.as_ref() else {
        return ServoError::InvalidHandle;
    };

    println!("servo_fill_offscreen_solid_color: filling offscreen with rgba=({}, {}, {}, {})", r, g, b, a);

    // Make the GL context current for the parent (Offscreen::make_current delegates to parent)
    if let Err(e) = offscreen.make_current() {
        eprintln!("servo_fill_offscreen_solid_color: make_current failed: {:?}", e);
        // continue anyway; try to perform the clear
    }

    // Bind the offscreen framebuffer so the clear affects it
    offscreen.prepare_for_rendering();

    // Use the gleam (gleam::gl) context to clear with the requested color
    let gl = offscreen.gleam_gl_api();
    let rf = (r as f32) / 255.0;
    let gf = (g as f32) / 255.0;
    let bf = (b as f32) / 255.0;
    let af = (a as f32) / 255.0;
    gl.clear_color(rf, gf, bf, af);
    gl.clear(gl::COLOR_BUFFER_BIT);

    // Immediately read back the offscreen buffer and save to /tmp for verification.
    // This helps confirm the clear affected the expected framebuffer.
    let size = offscreen.size();
    let rect = DeviceIntRect::from_origin_and_size(
        Point2D::origin(),
        Size2D::new(size.width as i32, size.height as i32),
    );
    if let Some(img) = offscreen.read_to_image(rect) {
        // Try to save; ignore errors but log
        let _ = img.save("/tmp/servo_fill_offscreen_check.png");
        println!("servo_fill_offscreen_solid_color: saved verification image to /tmp/servo_fill_offscreen_check.png ({}x{})", img.width(), img.height());
    } else {
        eprintln!("servo_fill_offscreen_solid_color: read_to_image returned None");
    }

    ServoError::Success
}

/// Request a WebRender capture for the given webview (writes captures to disk).
#[unsafe(no_mangle)]
pub extern "C" fn webview_capture_webrender(
    servo_ptr: *mut c_void,
    webview_ptr: *mut c_void,
) -> ServoError {
    if servo_ptr.is_null() || webview_ptr.is_null() {
        return ServoError::InvalidHandle;
    }

    let webview_instance = unsafe { &*(webview_ptr as *mut WebViewInstance) };
    webview_instance.webview.capture_webrender();
    ServoError::Success
}

/// Prepare the parent window rendering context for compositing. This calls
/// WindowRenderingContext::prepare_for_rendering() which binds the parent framebuffer.
#[unsafe(no_mangle)]
pub extern "C" fn servo_parent_prepare_for_rendering(
    servo_ptr: *mut c_void,
) -> ServoError {
    if servo_ptr.is_null() {
        return ServoError::InvalidHandle;
    }
    let servo_instance = unsafe { &*(servo_ptr as *const ServoInstance) };
    if let Some(parent) = &servo_instance.parent_context {
        parent.prepare_for_rendering();
        ServoError::Success
    } else {
        ServoError::InvalidHandle
    }
}

/// Present the parent window rendering context (swap buffers). Calls WindowRenderingContext::present().
#[unsafe(no_mangle)]
pub extern "C" fn servo_parent_present(
    servo_ptr: *mut c_void,
) -> ServoError {
    if servo_ptr.is_null() {
        return ServoError::InvalidHandle;
    }
    let servo_instance = unsafe { &*(servo_ptr as *const ServoInstance) };
    if let Some(parent) = &servo_instance.parent_context {
        // Optional diagnostic readback: inspect parent framebuffer contents before present
        if std::env::var_os("SERVO_DEBUG_READBACK").is_some() {
            if let Err(e) = parent.make_current() {
                eprintln!("servo_parent_present: parent.make_current failed: {:?}", e);
            } else {
                parent.prepare_for_rendering();
                let size = parent.size2d().to_i32();
                let rect = DeviceIntRect::from_origin_and_size(
                    Point2D::origin(),
                    Size2D::new(size.width as i32, size.height as i32),
                );
                match parent.read_to_image(rect) {
                    Some(img) => {
                        // simple summary
                        let mut non_black = 0usize;
                        let mut first_sample: Option<(usize, usize, [u8;4])> = None;
                        for (y, row) in img.rows().enumerate() {
                            for (x, px) in row.enumerate() {
                                let data = px.0;
                                if data[0] != 0 || data[1] != 0 || data[2] != 0 || data[3] != 0 {
                                    non_black += 1;
                                    if first_sample.is_none() {
                                        first_sample = Some((x, y, data));
                                    }
                                }
                            }
                        }
                        println!("servo_parent_present: parent readback before present: non_black_pixels={} first_sample={:?}", non_black, first_sample.map(|(x,y,px)| (x,y,px)));
                        if std::env::var_os("SERVO_DEBUG_READBACK").is_some() {
                            if let Err(e) = img.save("/tmp/servo_parent_readback.png") {
                                eprintln!("servo_parent_present: failed to save parent readback PNG: {:?}", e);
                            } else {
                                println!("servo_parent_present: saved parent readback to /tmp/servo_parent_readback.png");
                            }
                        }
                    }
                    None => {
                        eprintln!("servo_parent_present: parent.read_to_image returned None");
                    }
                }
            }
        }

        parent.present();
        ServoError::Success
    } else {
        ServoError::InvalidHandle
    }
}

/// Invoke the offscreen render_to_parent callback to blit the offscreen FBO to the parent
/// context. The callback expects a glow::Context and a rect; we will call it with the
/// parent glow context and the full viewport.
#[unsafe(no_mangle)]
pub extern "C" fn servo_render_offscreen_to_parent(
    servo_ptr: *mut c_void,
) -> ServoError {
    if servo_ptr.is_null() {
        return ServoError::InvalidHandle;
    }
    let servo_instance = unsafe { &*(servo_ptr as *const ServoInstance) };
    let Some(offscreen) = servo_instance.offscreen_context.as_ref() else {
        return ServoError::InvalidHandle;
    };

    println!("servo_render_offscreen_to_parent: called");

    if let Some(callback) = offscreen.render_to_parent_callback() {
    // Determine full target rect from parent size. Convert size to an untyped Size2D
    // so we produce a Rect<i32, UnknownUnit> which matches the callback signature.
    let parent = offscreen.parent_context();
    let size = parent.size2d().to_i32();
    let rect = euclid::Rect::new(euclid::Point2D::origin(), size.to_untyped());
        // Call the callback with the parent's glow context
    let gl = parent.glow_gl_api();
    callback(gl.as_ref(), rect);
        ServoError::Success
    } else {
        ServoError::InvalidHandle
    }
}

/// Present the rendered content to the screen
/// Should be called from Swift's draw(_:) method
#[unsafe(no_mangle)]
pub extern "C" fn webview_present(servo_ptr: *mut c_void) -> bool {
    assert!(!servo_ptr.is_null());
    
    let servo_instance = unsafe { &*(servo_ptr as *const ServoInstance) };
    servo_instance.rendering_context.present();
    
    true
}

/// Check if Servo needs repainting and spin the event loop
/// Returns true if a repaint is needed, false otherwise
#[unsafe(no_mangle)]
pub extern "C" fn servo_needs_repaint(servo_ptr: *mut c_void) -> bool {
    assert!(!servo_ptr.is_null());

    let servo_instance = unsafe { &*(servo_ptr as *const ServoInstance) };
    
    // Handle pending events - returns false if shutting down
    let should_continue = servo_instance.servo.spin_event_loop();
    
    if !should_continue {
        return false;
    }

    println!("Servo needs repaint");
    
    true
}

/// Spin the event loop for the Servo instance
#[unsafe(no_mangle)]
pub extern "C" fn spin_event_loop(servo_ptr: *mut c_void) -> ServoError {
    if servo_ptr.is_null() {
        return ServoError::InvalidHandle;
    }

    let servo_instance = unsafe { &*(servo_ptr as *const ServoInstance) };
    
    // Handle pending events - returns false if shutting down
    let should_continue = servo_instance.servo.spin_event_loop();
    
    if should_continue {
        ServoError::Success
    } else {
        ServoError::UnknownError
    }
}

/// Destroy a WebView (Swift should call this in WebView deinit)
#[unsafe(no_mangle)]
pub extern "C" fn destroy_webview(
    _servo_ptr: *mut c_void,
    webview_ptr: *mut c_void,
) -> ServoError {
    if webview_ptr.is_null() {
        return ServoError::InvalidHandle;
    }

    unsafe {
        let _webview_instance = Box::from_raw(webview_ptr as *mut WebViewInstance);
        // WebView will be dropped automatically
    }

    ServoError::Success
}

/// Destroy the Servo instance (Swift should call this in ServoInstance deinit)
#[unsafe(no_mangle)]
pub extern "C" fn destroy_servo(servo_ptr: *mut c_void) -> ServoError {
    if servo_ptr.is_null() {
        return ServoError::InvalidHandle;
    }

    unsafe {
        let _servo_instance = Box::from_raw(servo_ptr as *mut ServoInstance);
        // Servo instance will be dropped automatically
    }

    ServoError::Success
}

/// Get the version string
#[unsafe(no_mangle)]
pub extern "C" fn version() -> *const c_char {
    static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"));
    VERSION.as_ptr() as *const c_char
}

/// Get a render callback function that can blit the Servo framebuffer to a parent OpenGL context
/// Returns a function pointer that takes (gl_context, x, y, width, height) parameters
#[unsafe(no_mangle)]
pub extern "C" fn get_render_callback(
    servo_ptr: *mut c_void,
) -> Option<extern "C" fn(*const c_void, i32, i32, i32, i32)> {
    if servo_ptr.is_null() {
        error!("Invalid Servo pointer");
        return None;
    }

    // For now, return a simple dummy callback
    // TODO: Implement actual render_to_parent_callback integration
    Some(render_to_parent_static)
}

/// Static function that handles rendering to parent context
/// This will be called from Swift with OpenGL context and coordinates
extern "C" fn render_to_parent_static(
    _gl_context: *const c_void,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    // TODO: Implement actual blit operation
    // This would need access to the specific servo instance and its render callback
}
