/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::result::Result as StdResult;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_str};
use servo::EventLoopWaker;
use servo::base::id::WebViewId;
use servo::servo_url::ServoUrl;

/// Debounce timeout for URL input in milliseconds
const URL_INPUT_DEBOUNCE_TIMEOUT: Duration = Duration::from_millis(500);

struct PromptTemplates {
    browser_action: String,
    url_prediction_general: String,
    url_prediction_with_anchors: String,
}

impl PromptTemplates {
    fn load() -> Self {
        let browser_action = Self::load_prompt("browser_action.md");
        let url_prediction_general = Self::load_prompt("url_prediction.md");
        let url_prediction_with_anchors = Self::load_prompt("url_prediction_with_anchors.md");

        Self {
            browser_action,
            url_prediction_general,
            url_prediction_with_anchors,
        }
    }

    fn load_prompt(filename: &str) -> String {
        let prompt_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("desktop/prompts")
            .join(filename);

        fs::read_to_string(&prompt_path)
            .unwrap_or_else(|e| panic!("Failed to load prompt template '{}': {}", filename, e))
    }

    fn format_browser_action(&self, user_input: &str) -> String {
        self.browser_action.replace("{user_input}", user_input)
    }

    fn format_url_prediction_general(&self, user_input: &str) -> String {
        self.url_prediction_general
            .replace("{user_input}", user_input)
    }

    fn format_url_prediction_with_anchors(
        &self,
        user_input: &str,
        current_url: &str,
        anchor_urls: &[ServoUrl],
    ) -> String {
        let anchor_urls_formatted = Self::format_anchor_urls(anchor_urls);
        self.url_prediction_with_anchors
            .replace("{user_input}", user_input)
            .replace("{current_url}", current_url)
            .replace("{anchor_urls}", &anchor_urls_formatted)
    }

    fn format_anchor_urls(anchor_urls: &[ServoUrl]) -> String {
        if anchor_urls.is_empty() {
            "No anchor links available on the current page.".to_string()
        } else {
            let mut formatted = String::new();
            formatted.push_str("```\n");
            for (index, url) in anchor_urls.iter().enumerate() {
                formatted.push_str(&format!("{{ index: {}, url: \"{}\" }},\n", index + 1, url));
            }
            formatted.push_str("```");
            formatted
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<HashMap<String, Value>>,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub message: Message,
}

pub struct OllamaClient {
    client: Client,
    base_url: String,
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new("http://localhost:11434")
    }
}

impl OllamaClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
        }
    }

    /// Chat with message history
    pub fn chat(&self, model: impl Into<String>, messages: Vec<Message>) -> Option<Message> {
        let request = ChatRequest {
            model: model.into(),
            messages,
            stream: Some(false),
            options: None,
        };

        let url = format!("{}/api/chat", self.base_url);
        let response = self.client.post(&url).json(&request).send().ok()?;

        if !response.status().is_success() {
            return None;
        }

        let chat_response: ChatResponse = response.json().ok()?;
        Some(chat_response.message)
    }
}

// Threaded Ollama client structures
#[derive(Debug)]
pub enum OllamaMessage {
    User(String),
    UrlInput {
        webview_id: WebViewId,
        input: String,
        current_url: String,
    },
    UrlInputWithAnchors {
        webview_id: WebViewId,
        input: String,
        current_url: String,
    },
    AnchorUrls(WebViewId, Vec<ServoUrl>),
    Exit,
}

#[derive(Debug, Clone)]
struct PendingUrlInput {
    webview_id: WebViewId,
    input: String,
    current_url: String,
    timestamp: Instant,
}

#[derive(Debug, Clone)]
pub enum OllamaResponse {
    BrowserAction(BrowserAction),
    UrlPrediction {
        webview_id: WebViewId,
        input: String,
        predicted_urls: Vec<String>,
        anchored: bool,
    },
    RequestAnchors {
        webview_id: WebViewId,
    },
    Error(String),
}

#[derive(Debug, Clone)]
pub enum BrowserAction {
    Navigate(Vec<String>),
    Close,
    Nothing,
}

#[derive(Debug, Clone)]
pub enum AnchorState {
    Pending,
    Done(Vec<ServoUrl>),
}

pub struct OllamaHandle {
    sender: Sender<OllamaMessage>,
    receiver: Receiver<OllamaResponse>,
    join_handle: Option<JoinHandle<()>>,
}

impl OllamaHandle {
    pub fn send_user_message(&self, message: String) {
        let _ = self.sender.send(OllamaMessage::User(message));
    }

    pub fn send_url_input(&self, webview_id: WebViewId, input: String, current_url: String) {
        let _ = self.sender.send(OllamaMessage::UrlInput {
            webview_id,
            input,
            current_url,
        });
    }

    pub fn send_url_input_with_anchors(
        &self,
        webview_id: WebViewId,
        input: String,
        current_url: String,
    ) {
        let _ = self.sender.send(OllamaMessage::UrlInputWithAnchors {
            webview_id,
            input,
            current_url,
        });
    }

    pub fn send_anchor_urls(&self, webview_id: WebViewId, urls: Vec<ServoUrl>) {
        let _ = self
            .sender
            .send(OllamaMessage::AnchorUrls(webview_id, urls));
    }

    pub fn send_exit(&self) {
        let _ = self.sender.send(OllamaMessage::Exit);
    }

    pub fn try_recv_response(&self) -> StdResult<OllamaResponse, TryRecvError> {
        self.receiver.try_recv()
    }

    pub fn join(&mut self) {
        if let Some(join_handle) = self.join_handle.take() {
            // Check if thread is finished before joining to avoid blocking
            if join_handle.is_finished() {
                match join_handle.join() {
                    Ok(_) => {},
                    Err(_) => {
                        eprintln!("Ollama client thread panicked during shutdown");
                    },
                }
            }
        }
    }
}

struct OllamaWorker {
    client: OllamaClient,
    message_receiver: Receiver<OllamaMessage>,
    response_sender: Sender<OllamaResponse>,
    event_loop_waker: Box<dyn EventLoopWaker>,
    prompts: Option<PromptTemplates>,
    webview_anchors: HashMap<WebViewId, AnchorState>,
    models: HashMap<Model, String>,
    // Debounce state for URL input - single pending input (overwrite previous)
    pending_url_input: Option<PendingUrlInput>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum Model {
    Low,
    High,
}

impl OllamaWorker {
    fn new(
        message_receiver: Receiver<OllamaMessage>,
        response_sender: Sender<OllamaResponse>,
        event_loop_waker: Box<dyn EventLoopWaker>,
    ) -> Self {
        Self {
            client: OllamaClient::default(),
            message_receiver,
            response_sender,
            event_loop_waker,
            prompts: None, // Load lazily in the worker thread
            webview_anchors: HashMap::new(),
            models: {
                let mut m = HashMap::new();
                m.insert(Model::Low, "gemma3:1b".to_string());
                m.insert(Model::High, "gemma3n:e2b".to_string());
                m
            },
            pending_url_input: None,
        }
    }

    /// Send a response and wake the event loop
    fn send_response(&self, response: OllamaResponse) {
        if let Err(err) = self.response_sender.send(response) {
            eprintln!("Failed to send ollama response: {:?}", err);
        } else {
            self.event_loop_waker.wake();
        }
    }

    /// Check if we need to request anchors for this webview and do so if needed
    fn ensure_anchors_requested(&mut self, webview_id: WebViewId) {
        if !self.webview_anchors.contains_key(&webview_id) {
            // No anchors fetched yet, request them and mark as pending
            self.webview_anchors
                .insert(webview_id, AnchorState::Pending);
            self.send_response(OllamaResponse::RequestAnchors { webview_id });
        }
        // If anchors are already Pending or Done, no action needed
    }

    fn run(mut self) {
        // Load prompts once at startup
        self.prompts = Some(PromptTemplates::load());

        // Warm up models with a short "test" prompt.
        let test_msg = Message::user("test");
        for model_name in self.models.values() {
            let _ = self.client.chat(model_name.clone(), vec![test_msg.clone()]);
        }

        loop {
            // Check for expired debounce timer for the single pending input
            let now = Instant::now();
            if let Some(pending) = &self.pending_url_input {
                if now.duration_since(pending.timestamp) >= URL_INPUT_DEBOUNCE_TIMEOUT {
                    // Take the pending input and process it
                    let pending = self.pending_url_input.take().unwrap();
                    self.run_general_url_prediction(pending.webview_id, pending.input.clone());

                    // Continue to next loop iteration to recalc timeouts
                    continue;
                }
            }

            // Calculate the next timeout based on the pending input
            let timeout_duration = self.pending_url_input.as_ref().map(|pending| {
                let elapsed = now.duration_since(pending.timestamp);
                if elapsed >= URL_INPUT_DEBOUNCE_TIMEOUT {
                    Duration::from_millis(0)
                } else {
                    URL_INPUT_DEBOUNCE_TIMEOUT - elapsed
                }
            });

            // Receive message with optional timeout for debouncing
            let received_msg = if let Some(timeout) = timeout_duration {
                if timeout.as_millis() == 0 {
                    // Process any remaining expired inputs and continue
                    continue;
                }

                match self.message_receiver.recv_timeout(timeout) {
                    Ok(msg) => Some(msg),
                    Err(RecvTimeoutError::Timeout) => {
                        continue; // Check for expired timers again
                    },
                    Err(RecvTimeoutError::Disconnected) => {
                        break; // Channel closed
                    },
                }
            } else {
                // No pending inputs, block indefinitely
                match self.message_receiver.recv() {
                    Ok(msg) => Some(msg),
                    Err(_) => break, // Channel closed
                }
            };

            // Process the received message
            if let Some(msg) = received_msg {
                match msg {
                    OllamaMessage::User(text) => {
                        self.handle_user_message(text);
                    },
                    OllamaMessage::UrlInput {
                        webview_id,
                        input,
                        current_url,
                    } => {
                        // Ensure anchors are requested for this webview so the debounced anchored prediction
                        // can run when ready or when user requests it explicitly.
                        self.ensure_anchors_requested(webview_id);

                        // Overwrite any existing pending input with the new one for the anchored prediction
                        self.pending_url_input = Some(PendingUrlInput {
                            webview_id,
                            input,
                            current_url,
                            timestamp: Instant::now(),
                        });
                    },
                    OllamaMessage::UrlInputWithAnchors {
                        webview_id,
                        input,
                        current_url,
                    } => {
                        // Explicit request to run anchored prediction now (user clicked "predict using current page context")
                        // If anchors are available, run immediately; otherwise request anchors and keep pending state.
                        let anchor_urls = match self.webview_anchors.get(&webview_id) {
                            Some(AnchorState::Done(urls)) => urls.clone(),
                            _ => Vec::new(),
                        };
                        // Always make the prediction, even if we haven't received the anchors yet.
                        self.process_url_input_with_anchors(webview_id, input, current_url);
                    },
                    OllamaMessage::AnchorUrls(webview_id, urls) => {
                        self.handle_anchor_urls(webview_id, urls);
                    },
                    OllamaMessage::Exit => {
                        break;
                    },
                }

                // Drain any additional messages that arrived while we were processing
                loop {
                    match self.message_receiver.try_recv() {
                        Ok(msg) => match msg {
                            OllamaMessage::User(text) => {
                                self.handle_user_message(text);
                            },
                            OllamaMessage::UrlInput {
                                webview_id,
                                input,
                                current_url,
                            } => {
                                // Overwrite any existing pending input with the new one
                                self.pending_url_input = Some(PendingUrlInput {
                                    webview_id,
                                    input,
                                    current_url,
                                    timestamp: Instant::now(),
                                });
                                // Ensure anchors are requested for this webview
                                self.ensure_anchors_requested(webview_id);
                            },
                            OllamaMessage::UrlInputWithAnchors {
                                webview_id,
                                input,
                                current_url,
                            } => {
                                // Immediate request to run anchored prediction now
                                let anchor_urls = match self.webview_anchors.get(&webview_id) {
                                    Some(AnchorState::Done(urls)) => urls.clone(),
                                    _ => Vec::new(),
                                };
                                self.process_url_input_with_anchors(webview_id, input, current_url);
                            },
                            OllamaMessage::AnchorUrls(webview_id, urls) => {
                                self.handle_anchor_urls(webview_id, urls);
                            },
                            OllamaMessage::Exit => {
                                return;
                            },
                        },
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => {
                            return;
                        },
                    }
                }
            }
        }
    }

    fn handle_user_message(&mut self, text: String) {
        let prompt = self.prompts.as_ref().unwrap().format_browser_action(&text);
        let user_message = Message::user(prompt);

        let model_name = self
            .models
            .get(&Model::Low)
            .expect("Model::Low missing")
            .clone();
        match self.client.chat(&model_name, vec![user_message]) {
            Some(response) => {
                let browser_action = self.parse_browser_action_response(&response.content);
                self.send_response(OllamaResponse::BrowserAction(browser_action));
            },
            None => {
                eprintln!("Ollama browser action error");
                self.send_response(OllamaResponse::BrowserAction(BrowserAction::Nothing));
            },
        }
    }

    fn handle_anchor_urls(&mut self, webview_id: WebViewId, urls: Vec<ServoUrl>) {
        // Update the anchor state to Done with the received URLs
        self.webview_anchors
            .insert(webview_id, AnchorState::Done(urls));
        // The main run loop will check if there are pending inputs ready to be processed
    }

    /// Process URL input when timer has expired (anchors used if available)
    fn process_url_input_with_anchors(
        &mut self,
        webview_id: WebViewId,
        input: String,
        current_url: String,
    ) {
        // Get anchor URLs if available, otherwise use empty list
        let anchor_urls = match self.webview_anchors.get(&webview_id) {
            Some(AnchorState::Done(urls)) => urls.clone(),
            _ => Vec::new(), // No anchors available yet, proceed with empty list
        };

        let prompt = self
            .prompts
            .as_ref()
            .unwrap()
            .format_url_prediction_with_anchors(&input, &current_url, &anchor_urls);
        let user_message = Message::user(prompt);
        let model_name = self
            .models
            .get(&Model::High)
            .expect("model missing")
            .clone();
        match self.client.chat(&model_name, vec![user_message]) {
            Some(response) => {
                let predicted_urls = self.parse_url_prediction_response(&response.content);
                self.send_response(OllamaResponse::UrlPrediction {
                    webview_id,
                    input: input.clone(),
                    predicted_urls,
                    anchored: true,
                });
            },
            None => {
                eprintln!("URL prediction error");
                self.send_response(OllamaResponse::UrlPrediction {
                    webview_id,
                    input: input.clone(),
                    predicted_urls: Vec::new(),
                    anchored: true,
                });
            },
        }
    }

    /// Run a quick general URL prediction using only the user input and the generic prompt
    /// This is called immediately on UrlInput so the UI can show instant suggestions.
    fn run_general_url_prediction(&mut self, webview_id: WebViewId, input: String) {
        let prompt = self
            .prompts
            .as_ref()
            .unwrap()
            .format_url_prediction_general(&input);
        let user_message = Message::user(prompt);
        let model_name = self
            .models
            .get(&Model::Low)
            .expect("Model::Low missing")
            .clone();
        match self.client.chat(&model_name, vec![user_message]) {
            Some(response) => {
                let predicted_urls = self.parse_url_prediction_response(&response.content);
                self.send_response(OllamaResponse::UrlPrediction {
                    webview_id,
                    input: input.clone(),
                    predicted_urls,
                    anchored: false,
                });
            },
            None => {
                eprintln!("General URL prediction error");
                self.send_response(OllamaResponse::UrlPrediction {
                    webview_id,
                    input: input.clone(),
                    predicted_urls: Vec::new(),
                    anchored: false,
                });
            },
        }
    }

    fn parse_browser_action_response(&self, content: &str) -> BrowserAction {
        let json_content = match self.extract_json_from_response(content) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to extract JSON from response: {}", e);
                return BrowserAction::Nothing;
            },
        };

        match from_str::<Value>(json_content) {
            Ok(json) => {
                if let Some(action) = json.get("action").and_then(|a| a.as_str()) {
                    match action {
                        "NAVIGATE" => {
                            if let Some(value) = json.get("value").and_then(|v| v.as_array()) {
                                let urls: Vec<String> = value
                                    .iter()
                                    .filter_map(|url| url.as_str())
                                    .map(|s| s.to_string())
                                    .collect();
                                BrowserAction::Navigate(urls)
                            } else {
                                BrowserAction::Nothing
                            }
                        },
                        "CLOSE" => BrowserAction::Close,
                        "NOTHING" => BrowserAction::Nothing,
                        _ => BrowserAction::Nothing,
                    }
                } else {
                    BrowserAction::Nothing
                }
            },
            Err(e) => {
                eprintln!("Failed to parse JSON response: {:?}", e);
                eprintln!("Raw response: {}", content);
                BrowserAction::Nothing
            },
        }
    }

    fn parse_url_prediction_response(&self, content: &str) -> Vec<String> {
        let json_content = match self.extract_json_from_response(content) {
            Ok(s) => s,
            Err(_) => {
                // If JSON cannot be extracted, fall back to previous heuristic: treat
                // the whole content as a possible single URL or return empty.
                let trimmed = content.trim();
                if !trimmed.is_empty() && (trimmed.starts_with("http") || trimmed.contains(".")) {
                    return vec![trimmed.to_string()];
                } else {
                    return Vec::new();
                }
            },
        };

        match from_str::<Value>(json_content) {
            Ok(json) => {
                // Try to get "urls" array first, fallback to "predictions" or direct array
                if let Some(urls_array) = json
                    .get("urls")
                    .and_then(|v| v.as_array())
                    .or_else(|| json.get("predictions").and_then(|v| v.as_array()))
                    .or_else(|| json.as_array())
                {
                    urls_array
                        .iter()
                        .filter_map(|url| url.as_str())
                        .map(|s| s.to_string())
                        .collect()
                } else {
                    // Fallback: try to extract as a single URL
                    if let Some(single_url) = json.get("url").and_then(|v| v.as_str()) {
                        vec![single_url.to_string()]
                    } else {
                        Vec::new()
                    }
                }
            },
            Err(_) => Vec::new(),
        }
    }

    fn extract_json_from_response<'a>(&self, content: &'a str) -> Result<&'a str, String> {
        let content = content.trim();

        if content.starts_with("```json") && content.ends_with("```") {
            let inner = content
                .strip_prefix("```json")
                .and_then(|s| s.strip_suffix("```"))
                .unwrap_or(content)
                .trim();
            if inner.is_empty() {
                Err("empty json block".to_string())
            } else {
                Ok(inner)
            }
        } else if content.starts_with("```") && content.ends_with("```") {
            let inner = content
                .strip_prefix("```")
                .and_then(|s| s.strip_suffix("```"))
                .unwrap_or(content)
                .trim();
            if inner.is_empty() {
                Err("empty fenced block".to_string())
            } else {
                Ok(inner)
            }
        } else if content.starts_with('{') || content.starts_with('[') {
            Ok(content)
        } else {
            Err("no JSON found in response".to_string())
        }
    }
}

/// Starts the ollama client worker thread and returns a handle.
/// Only available on macOS.
#[cfg(target_os = "macos")]
pub fn start_ollama_client(event_loop_waker: Box<dyn EventLoopWaker>) -> Option<OllamaHandle> {
    // Set up channels for communication
    let (message_sender, message_receiver) = mpsc::channel();
    let (response_sender, response_receiver) = mpsc::channel();

    // Spawn the ollama worker thread
    let worker = OllamaWorker::new(message_receiver, response_sender, event_loop_waker);
    let join_handle = thread::spawn(move || {
        worker.run();
    });

    Some(OllamaHandle {
        sender: message_sender,
        receiver: response_receiver,
        join_handle: Some(join_handle),
    })
}

/// For non-macOS platforms, always return None
#[cfg(not(target_os = "macos"))]
pub fn start_ollama_client(_event_loop_waker: Box<dyn EventLoopWaker>) -> Option<OllamaHandle> {
    None
}
