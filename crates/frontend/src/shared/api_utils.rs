//! API utilities for frontend-backend communication
//!
//! Provides helper functions for constructing API URLs and making requests.

/// Get the base URL for API requests
///
/// Constructs the API base URL from the current window location,
/// using port 3000 for the backend server.
///
/// # Returns
/// - API base URL like "http://localhost:3000" or "https://example.com:3000"
/// - Empty string if window is not available
///
/// # Example
/// ```rust
/// let url = format!("{}/api/nomenclature/{}", api_base(), id);
/// ```
pub fn api_base() -> String {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return String::new(),
    };
    let location = window.location();
    let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    format!("{}//{}:3000", protocol, hostname)
}

/// Build a full API URL from a path
///
/// # Arguments
/// * `path` - The API path (should start with "/api/")
///
/// # Example
/// ```rust
/// let url = api_url("/api/nomenclature/123");
/// ```
pub fn api_url(path: &str) -> String {
    format!("{}{}", api_base(), path)
}
