/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use dpi::PhysicalSize;
use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;
use egui::{
    Button, CentralPanel, Frame, Key, Label, Modifiers, PaintCallback, TopBottomPanel, Vec2, pos2,
};
use egui_glow::CallbackFn;
use egui_winit::EventResponse;
use euclid::{Box2D, Length, Point2D, Rect, Scale, Size2D};
use log::{trace, warn};
use servo::base::id::WebViewId;
use servo::servo_geometry::DeviceIndependentPixel;
use servo::servo_url::ServoUrl;
use servo::webrender_api::units::DevicePixel;
use servo::{LoadStatus, OffscreenRenderingContext, RenderingContext, WebView};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use super::app_state::RunningAppState;
use super::egui_glue::EguiGlow;
use super::events_loop::EventLoopProxy;
use super::geometry::winit_position_to_euclid_point;
use super::headed_window::Window as ServoWindow;
use crate::desktop::window_trait::WindowPortsMethods;

pub struct Minibrowser {
    rendering_context: Rc<OffscreenRenderingContext>,
    pub context: EguiGlow,
    pub event_queue: RefCell<Vec<MinibrowserEvent>>,
    pub toolbar_height: Length<f32, DeviceIndependentPixel>,

    last_update: Instant,
    last_mouse_position: Option<Point2D<f32, DeviceIndependentPixel>>,
    location: RefCell<String>,

    /// Whether the location has been edited by the user without clicking Go.
    location_dirty: Cell<bool>,

    load_status: LoadStatus,

    status_text: Option<String>,

    // Add the new text input field
    bottom_text_input: RefCell<String>,

    /// Stores the current URL predictions for the address bar
    predicted_urls: RefCell<Option<Vec<String>>>,

    /// Stores the input text that the current predictions were made for
    prediction_input: RefCell<Option<String>>,
}

pub enum MinibrowserEvent {
    /// Go button clicked.
    Go(String),
    Back,
    Forward,
    Reload,
    NewWebView,
    CloseWebView(WebViewId),
    /// LLM input submitted.
    LLMInput(String),
    /// Address bar text changed.
    AddressBarInput(String),
    /// Clear URL predictions.
    ClearUrlPredictions,
}

fn truncate_with_ellipsis(input: &str, max_length: usize) -> String {
    if input.chars().count() > max_length {
        let truncated: String = input.chars().take(max_length.saturating_sub(1)).collect();
        format!("{}…", truncated)
    } else {
        input.to_string()
    }
}

impl Drop for Minibrowser {
    fn drop(&mut self) {
        self.context.destroy();
    }
}

impl Minibrowser {
    pub fn new(
        window: &ServoWindow,
        event_loop: &ActiveEventLoop,
        event_loop_proxy: EventLoopProxy,
        initial_url: ServoUrl,
    ) -> Self {
        let rendering_context = window.offscreen_rendering_context();
        // Adapted from https://github.com/emilk/egui/blob/9478e50d012c5138551c38cbee16b07bc1fcf283/crates/egui_glow/examples/pure_glow.rs
        #[allow(clippy::arc_with_non_send_sync)]
        let context = EguiGlow::new(
            window,
            event_loop,
            event_loop_proxy,
            rendering_context.glow_gl_api(),
            None,
        );

        // Disable the builtin egui handlers for the Ctrl+Plus, Ctrl+Minus and Ctrl+0
        // shortcuts as they don't work well with servoshell's `device-pixel-ratio` CLI argument.
        context
            .egui_ctx
            .options_mut(|options| options.zoom_with_keyboard = false);

        Self {
            rendering_context,
            context,
            event_queue: RefCell::new(vec![]),
            toolbar_height: Default::default(),
            last_update: Instant::now(),
            last_mouse_position: None,
            location: RefCell::new(initial_url.to_string()),
            location_dirty: false.into(),
            load_status: LoadStatus::Complete,
            status_text: None,
            // Initialize the new text input field
            bottom_text_input: RefCell::new(String::new()),
            // Initialize URL prediction tracking
            predicted_urls: RefCell::new(None),
            // Initialize prediction input tracking
            prediction_input: RefCell::new(None),
        }
    }

    pub(crate) fn take_events(&self) -> Vec<MinibrowserEvent> {
        self.event_queue.take()
    }

    /// Preprocess the given [winit::event::WindowEvent], returning unconsumed for mouse events in
    /// the Servo browser rect. This is needed because the CentralPanel we create for our webview
    /// would otherwise make egui report events in that area as consumed.
    pub fn on_window_event(
        &mut self,
        window: &Window,
        app_state: &RunningAppState,
        event: &WindowEvent,
    ) -> EventResponse {
        let mut result = self.context.on_window_event(window, event);

        if app_state.has_active_dialog() {
            result.consumed = true;
            return result;
        }

        result.consumed &= match event {
            WindowEvent::CursorMoved { position, .. } => {
                let scale = Scale::<_, DeviceIndependentPixel, _>::new(
                    self.context.egui_ctx.pixels_per_point(),
                );
                self.last_mouse_position =
                    Some(winit_position_to_euclid_point(*position).to_f32() / scale);
                self.last_mouse_position
                    .is_some_and(|p| self.is_in_egui_toolbar_rect(p))
            },
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Forward,
                ..
            } => {
                self.event_queue
                    .borrow_mut()
                    .push(MinibrowserEvent::Forward);
                true
            },
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Back,
                ..
            } => {
                self.event_queue.borrow_mut().push(MinibrowserEvent::Back);
                true
            },
            WindowEvent::MouseWheel { .. } | WindowEvent::MouseInput { .. } => self
                .last_mouse_position
                .is_some_and(|p| self.is_in_egui_toolbar_rect(p)),
            _ => true,
        };
        result
    }

    /// Return true iff the given position is over the egui toolbar.
    fn is_in_egui_toolbar_rect(&self, position: Point2D<f32, DeviceIndependentPixel>) -> bool {
        position.y < self.toolbar_height.get()
    }

    /// Create a frameless button with square sizing, as used in the toolbar.
    fn toolbar_button(text: &str) -> egui::Button {
        egui::Button::new(text)
            .frame(false)
            .min_size(Vec2 { x: 20.0, y: 20.0 })
    }

    /// Draws a browser tab, checking for clicks and queues appropriate `MinibrowserEvent`s.
    /// Using a custom widget here would've been nice, but it doesn't seem as though egui
    /// supports that, so we arrange multiple Widgets in a way that they look connected.
    fn browser_tab(ui: &mut egui::Ui, webview: WebView, event_queue: &mut Vec<MinibrowserEvent>) {
        let label = match (webview.page_title(), webview.url()) {
            (Some(title), _) if !title.is_empty() => title,
            (_, Some(url)) => url.to_string(),
            _ => "New Tab".into(),
        };

        let old_item_spacing = ui.spacing().item_spacing;
        let old_visuals = ui.visuals().clone();
        let active_bg_color = old_visuals.widgets.active.weak_bg_fill;
        let inactive_bg_color = old_visuals.window_fill;
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

        let visuals = ui.visuals_mut();
        // Remove the stroke so we don't see the border between the close button and the label
        visuals.widgets.active.bg_stroke.width = 0.0;
        visuals.widgets.hovered.bg_stroke.width = 0.0;
        // Now we make sure the fill color is always the same, irrespective of state, that way
        // we can make sure that both the label and close button have the same background color
        visuals.widgets.noninteractive.weak_bg_fill = inactive_bg_color;
        visuals.widgets.inactive.weak_bg_fill = inactive_bg_color;
        visuals.widgets.hovered.weak_bg_fill = active_bg_color;
        visuals.widgets.active.weak_bg_fill = active_bg_color;
        visuals.selection.bg_fill = active_bg_color;
        visuals.selection.stroke.color = visuals.widgets.active.fg_stroke.color;
        visuals.widgets.hovered.fg_stroke.color = visuals.widgets.active.fg_stroke.color;

        // Expansion would also show that they are 2 separate widgets
        visuals.widgets.active.expansion = 0.0;
        visuals.widgets.hovered.expansion = 0.0;
        // The rounding is changed so it looks as though the 2 widgets are a single widget
        // with a uniform rounding
        let corner_radius = egui::CornerRadius {
            ne: 0,
            nw: 4,
            sw: 4,
            se: 0,
        };
        visuals.widgets.active.corner_radius = corner_radius;
        visuals.widgets.hovered.corner_radius = corner_radius;
        visuals.widgets.inactive.corner_radius = corner_radius;

        let selected = webview.focused();
        let tab = ui.add(Button::selectable(
            selected,
            truncate_with_ellipsis(&label, 20),
        ));
        let tab = tab.on_hover_ui(|ui| {
            ui.label(label);
        });

        let corner_radius = egui::CornerRadius {
            ne: 4,
            nw: 0,
            sw: 0,
            se: 4,
        };
        let visuals = ui.visuals_mut();
        visuals.widgets.active.corner_radius = corner_radius;
        visuals.widgets.hovered.corner_radius = corner_radius;
        visuals.widgets.inactive.corner_radius = corner_radius;

        let fill_color = if selected || tab.hovered() {
            active_bg_color
        } else {
            inactive_bg_color
        };

        ui.spacing_mut().item_spacing = old_item_spacing;
        let close_button = ui.add(egui::Button::new("X").fill(fill_color));
        *ui.visuals_mut() = old_visuals;
        if close_button.clicked() || close_button.middle_clicked() || tab.middle_clicked() {
            event_queue.push(MinibrowserEvent::CloseWebView(webview.id()))
        } else if !selected && tab.clicked() {
            webview.focus();
        }
    }

    /// Update the minibrowser, but don’t paint.
    /// If `servo_framebuffer_id` is given, set up a paint callback to blit its contents to our
    /// CentralPanel when [`Minibrowser::paint`] is called.
    pub fn update(
        &mut self,
        window: &dyn WindowPortsMethods,
        state: &RunningAppState,
        reason: &'static str,
    ) {
        let now = Instant::now();
        let winit_window = window.winit_window().unwrap();
        trace!(
            "{:?} since last update ({})",
            now - self.last_update,
            reason
        );
        let Self {
            rendering_context,
            context,
            event_queue,
            toolbar_height,
            last_update,
            location,
            location_dirty,
            bottom_text_input,
            predicted_urls,
            prediction_input,
            load_status,
            ..
        } = self;

        let _duration = context.run(winit_window, |ctx| {
            // A simple Tab header strip
            TopBottomPanel::top("tabs").show(ctx, |ui| {
                ui.allocate_ui_with_layout(
                    ui.available_size(),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        for (_, webview) in state.webviews().into_iter() {
                            Self::browser_tab(ui, webview, &mut event_queue.borrow_mut());
                        }
                        if ui.add(Minibrowser::toolbar_button("+")).clicked() {
                            event_queue.borrow_mut().push(MinibrowserEvent::NewWebView);
                        }
                    },
                );
            });

            // TODO: While in fullscreen add some way to mitigate the increased phishing risk
            // when not displaying the URL bar: https://github.com/servo/servo/issues/32443
            if winit_window.fullscreen().is_none() {
                let frame = egui::Frame::default()
                    .fill(ctx.style().visuals.window_fill)
                    .inner_margin(4.0);
                TopBottomPanel::top("toolbar").frame(frame).show(ctx, |ui| {
                    ui.allocate_ui_with_layout(
                        ui.available_size(),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            if ui.add(Minibrowser::toolbar_button("⏴")).clicked() {
                                event_queue.borrow_mut().push(MinibrowserEvent::Back);
                            }
                            if ui.add(Minibrowser::toolbar_button("⏵")).clicked() {
                                event_queue.borrow_mut().push(MinibrowserEvent::Forward);
                            }

                            match *load_status {
                                LoadStatus::Started | LoadStatus::HeadParsed => {
                                    if ui.add(Minibrowser::toolbar_button("X")).clicked() {
                                        warn!("Do not support stop yet.");
                                    }
                                },
                                LoadStatus::Complete => {
                                    if ui.add(Minibrowser::toolbar_button("↻")).clicked() {
                                        event_queue.borrow_mut().push(MinibrowserEvent::Reload);
                                    }
                                },
                            }
                            ui.add_space(2.0);

                            ui.allocate_ui_with_layout(
                                ui.available_size(),
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let location_id = egui::Id::new("location_input");

                                    // Get current location text
                                    let mut location_binding = location.borrow_mut();

                                    let text_edit =
                                        egui::TextEdit::singleline(&mut *location_binding)
                                            .id(location_id);

                                    let location_field =
                                        ui.add_sized(ui.available_size(), text_edit);

                                    // Release the borrow before continuing
                                    drop(location_binding);

                                    if location_field.changed() {
                                        location_dirty.set(true);
                                        // Clear any existing predictions when user types
                                        *predicted_urls.borrow_mut() = None;
                                        *prediction_input.borrow_mut() = None;
                                        // Send address bar input event immediately
                                        let current_text = location.borrow().clone();
                                        if current_text.trim().len() > 1 {
                                            event_queue.borrow_mut().push(
                                                MinibrowserEvent::AddressBarInput(current_text),
                                            );
                                        }
                                    }
                                    // Handle adddress bar shortcut.
                                    if ui.input(|i| {
                                        if cfg!(target_os = "macos") {
                                            i.clone().consume_key(Modifiers::COMMAND, Key::L)
                                        } else {
                                            i.clone().consume_key(Modifiers::COMMAND, Key::L) ||
                                                i.clone().consume_key(Modifiers::ALT, Key::D)
                                        }
                                    }) {
                                        // The focus request immediately makes gained_focus return true.
                                        location_field.request_focus();
                                    }
                                    // Select address bar text when it's focused (click or shortcut).
                                    if location_field.gained_focus() {
                                        if let Some(mut state) =
                                            TextEditState::load(ui.ctx(), location_id)
                                        {
                                            // Select the whole input.
                                            state.cursor.set_char_range(Some(CCursorRange::two(
                                                CCursor::new(0),
                                                CCursor::new(location.borrow().len()),
                                            )));
                                            state.store(ui.ctx(), location_id);
                                        }
                                    }
                                    // Navigate to address when enter is pressed in the address bar.
                                    if location_field.lost_focus() &&
                                        ui.input(|i| i.clone().key_pressed(Key::Enter))
                                    {
                                        event_queue
                                            .borrow_mut()
                                            .push(MinibrowserEvent::Go(location.borrow().clone()));
                                    }
                                },
                            );
                        },
                    );
                });
            };

            // URL predictions dropdown below address bar
            let has_pending = state.has_pending_url_predictions();
            let predictions = predicted_urls.borrow().clone();
            let prediction_input_text = prediction_input.borrow().clone();
            let current_location_text = location.borrow().clone();

            // Only show predictions if they match the current input or if we have pending predictions
            let should_show_predictions = has_pending ||
                (predictions.is_some() &&
                    !predictions.as_ref().unwrap().is_empty() &&
                    prediction_input_text.as_ref() == Some(&current_location_text));

            if should_show_predictions {
                TopBottomPanel::top("url_predictions")
                    .frame(
                        egui::Frame::default()
                            .fill(ctx.style().visuals.window_fill)
                            .inner_margin(4.0)
                            .stroke(egui::Stroke::new(1.0, ctx.style().visuals.faint_bg_color)),
                    )
                    .show(ctx, |ui| {
                        ui.vertical(|ui| {
                            if has_pending {
                                // Show spinning indicator or "pending..." text
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label("Predicting URLs...");
                                });
                            } else if let Some(predicted_urls_vec) = predictions.as_ref() {
                                for predicted_url in predicted_urls_vec.iter() {
                                    if ui.button(predicted_url).clicked() {
                                        // Immediately update the address bar to show the selected URL
                                        *location.borrow_mut() = predicted_url.clone();
                                        location_dirty.set(false);

                                        // Clear predictions to hide the dropdown immediately
                                        *predicted_urls.borrow_mut() = None;
                                        *prediction_input.borrow_mut() = None;

                                        // Clear predictions from app state too
                                        event_queue
                                            .borrow_mut()
                                            .push(MinibrowserEvent::ClearUrlPredictions);

                                        event_queue
                                            .borrow_mut()
                                            .push(MinibrowserEvent::Go(predicted_url.clone()));
                                    }
                                }
                            }
                        });
                    });
            }

            // Add the new bottom panel with simple text input
            TopBottomPanel::bottom("llm_input").show(ctx, |ui| {
                ui.allocate_ui_with_layout(
                    ui.available_size(),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        // Text input area
                        ui.label("Ask the LLM:");
                        let text_edit = ui.add_sized(
                            [ui.available_width(), 60.0], // Make it 60 pixels tall
                            egui::TextEdit::multiline(&mut *bottom_text_input.borrow_mut())
                                .hint_text("Type your question and press Enter..."),
                        );

                        // Handle Enter key press
                        if text_edit.has_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                            let text = bottom_text_input.borrow().clone();
                            if !text.trim().is_empty() {
                                // Send LLMInput event instead of calling ollama directly
                                event_queue
                                    .borrow_mut()
                                    .push(MinibrowserEvent::LLMInput(text));

                                // Clear the input after sending
                                bottom_text_input.replace(String::new());
                            }
                        }
                    },
                );
            }); // The toolbar height is where the Context’s available rect starts.
            // For reasons that are unclear, the TopBottomPanel’s ui cursor exceeds this by one egui
            // point, but the Context is correct and the TopBottomPanel is wrong.
            *toolbar_height = Length::new(ctx.available_rect().min.y);
            window.set_toolbar_height(*toolbar_height);

            let scale =
                Scale::<_, DeviceIndependentPixel, DevicePixel>::new(ctx.pixels_per_point());

            egui::CentralPanel::default().show(ctx, |_| {
                state.for_each_active_dialog(|dialog| dialog.update(ctx));
            });

            let Some(webview) = state.focused_webview() else {
                return;
            };
            CentralPanel::default().frame(Frame::NONE).show(ctx, |ui| {
                // If the top parts of the GUI changed size, then update the size of the WebView and also
                // the size of its RenderingContext.
                let available_size = ui.available_size();
                let size = Size2D::new(available_size.x, available_size.y) * scale;
                let rect = Box2D::from_origin_and_size(Point2D::origin(), size);
                if rect != webview.rect() {
                    webview.move_resize(rect);
                    // `rect` is sized to just the WebView viewport, which is required by
                    // `OffscreenRenderingContext` See:
                    // <https://github.com/servo/servo/issues/38369#issuecomment-3138378527>
                    webview.resize(PhysicalSize::new(size.width as u32, size.height as u32))
                }

                let min = ui.cursor().min;
                let size = ui.available_size();
                let rect = egui::Rect::from_min_size(min, size);
                ui.allocate_space(size);

                if let Some(status_text) = &self.status_text {
                    egui::Tooltip::always_open(
                        ctx.clone(),
                        ui.layer_id(),
                        "tooltip layer".into(),
                        pos2(0.0, ctx.available_rect().max.y),
                    )
                    .show(|ui| ui.add(Label::new(status_text.clone()).extend()))
                    .map(|response| response.inner);
                }

                state.repaint_servo_if_necessary();

                if let Some(render_to_parent) = rendering_context.render_to_parent_callback() {
                    ui.painter().add(PaintCallback {
                        rect,
                        callback: Arc::new(CallbackFn::new(move |info, painter| {
                            let clip = info.viewport_in_pixels();
                            let rect_in_parent = Rect::new(
                                Point2D::new(clip.left_px, clip.from_bottom_px),
                                Size2D::new(clip.width_px, clip.height_px),
                            );
                            render_to_parent(painter.gl(), rect_in_parent)
                        })),
                    });
                }
            });

            *last_update = now;
        });
    }

    /// Paint the minibrowser, as of the last update.
    pub fn paint(&mut self, window: &Window) {
        self.rendering_context
            .parent_context()
            .prepare_for_rendering();
        self.context.paint(window);
        self.rendering_context.parent_context().present();
    }

    /// Updates the location field from the given [WebViewManager], unless the user has started
    /// editing it without clicking Go, returning true iff it has changed (needing an egui update).
    pub fn update_location_in_toolbar(&mut self, state: &RunningAppState) -> bool {
        // User edited without clicking Go?
        if self.location_dirty.get() {
            return false;
        }

        let current_url_string = state
            .focused_webview()
            .and_then(|webview| Some(webview.url()?.to_string()));
        match current_url_string {
            Some(location) if location != *self.location.get_mut() => {
                self.location = RefCell::new(location.to_owned());
                true
            },
            _ => false,
        }
    }

    pub fn update_location_dirty(&self, dirty: bool) {
        self.location_dirty.set(dirty);
    }

    pub fn update_load_status(&mut self, state: &RunningAppState) -> bool {
        let state_status = state
            .focused_webview()
            .map(|webview| webview.load_status())
            .unwrap_or(LoadStatus::Complete);
        let old_status = std::mem::replace(&mut self.load_status, state_status);
        old_status != self.load_status
    }

    pub fn update_status_text(&mut self, state: &RunningAppState) -> bool {
        let state_status = state
            .focused_webview()
            .and_then(|webview| webview.status_text());
        let old_status = std::mem::replace(&mut self.status_text, state_status);
        old_status != self.status_text
    }

    /// Updates all fields taken from the given [WebViewManager], such as the location field.
    /// Returns true iff the egui needs an update.
    pub fn update_webview_data(&mut self, state: &RunningAppState) -> bool {
        // Update URL prediction from the app state (global prediction state)
        let new_prediction_data = state
            .get_url_prediction_state()
            .map(|(_origin, input, predicted_urls)| (input, predicted_urls));
        let old_predictions = self.predicted_urls.borrow().clone();
        let old_input = self.prediction_input.borrow().clone();

        // Extract new values or use None
        let (new_predictions, new_input) = match new_prediction_data {
            Some((input, predicted_urls)) => (Some(predicted_urls), Some(input)),
            None => (None, None),
        };

        // Update stored values
        *self.predicted_urls.borrow_mut() = new_predictions;
        *self.prediction_input.borrow_mut() = new_input;

        // Check if anything changed
        let prediction_changed = old_predictions != *self.predicted_urls.borrow() ||
            old_input != *self.prediction_input.borrow();

        // Note: We must use the "bitwise OR" (|) operator here instead of "logical OR" (||)
        //       because logical OR would short-circuit if any of the functions return true.
        //       We want to ensure that all functions are called. The "bitwise OR" operator
        //       does not short-circuit.
        self.update_location_in_toolbar(state) |
            self.update_load_status(state) |
            self.update_status_text(state) |
            prediction_changed
    }

    /// Returns true if a redraw is required after handling the provided event.
    pub(crate) fn handle_accesskit_event(&mut self, event: &accesskit_winit::WindowEvent) -> bool {
        match event {
            accesskit_winit::WindowEvent::InitialTreeRequested => {
                self.context.egui_ctx.enable_accesskit();
                true
            },
            accesskit_winit::WindowEvent::ActionRequested(req) => {
                self.context
                    .egui_winit
                    .on_accesskit_action_request(req.clone());
                true
            },
            accesskit_winit::WindowEvent::AccessibilityDeactivated => {
                self.context.egui_ctx.disable_accesskit();
                false
            },
        }
    }
}
