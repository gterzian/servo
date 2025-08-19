/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! ServoView management
//!
//! This module handles the lifecycle and management of individual WebViews
//! within a Servo instance. It provides higher-level abstractions over
//! the raw WebView API for easier Swift integration.

use servo::WebView;
use url::Url;

/// A managed WebView with additional metadata for Swift integration
pub struct ManagedWebView {
    pub webview: WebView,
    pub url: Option<Url>,
    pub title: Option<String>,
    pub loading: bool,
}

impl ManagedWebView {
    pub fn new(webview: WebView) -> Self {
        Self {
            webview,
            url: None,
            title: None,
            loading: false,
        }
    }

    pub fn update_url(&mut self, url: Url) {
        self.url = Some(url);
    }

    pub fn update_title(&mut self, title: String) {
        self.title = Some(title);
    }

    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }
}
