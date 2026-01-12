//! Clipboard utilities for copying text to clipboard
//!
//! Provides a simple async function to copy text to the system clipboard
//! using the Web Clipboard API.

use wasm_bindgen_futures::spawn_local;

/// Copy text to the system clipboard
///
/// Uses the Web Clipboard API to copy the provided text.
/// This function is async-safe and handles errors silently.
///
/// # Example
/// ```rust
/// copy_to_clipboard("Hello, World!");
/// ```
pub fn copy_to_clipboard(text: &str) {
    let text = text.to_owned();
    spawn_local(async move {
        if let Some(window) = web_sys::window() {
            let clipboard = window.navigator().clipboard();
            let _ = wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&text)).await;
        }
    });
}

/// Copy text to clipboard with a callback on success
///
/// Useful when you need to show a notification after copying.
pub fn copy_to_clipboard_with_callback<F>(text: &str, on_success: F)
where
    F: FnOnce() + 'static,
{
    let text = text.to_owned();
    spawn_local(async move {
        if let Some(window) = web_sys::window() {
            let clipboard = window.navigator().clipboard();
            if wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&text))
                .await
                .is_ok()
            {
                on_success();
            }
        }
    });
}
