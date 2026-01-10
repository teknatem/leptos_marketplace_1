---
title: API Client Standardization Patterns
date: 2025-12-22
tags: rust, reqwest, api, patterns
---

# Lesson: API Client Standardization

## Context

When building multiple marketplace integrations (WB, Ozon, Yandex), inconsistent HTTP client configuration leads to hard-to-debug "ghost" errors where one integration works while another fails due to environment-specific factors.

## Standard Configuration Pattern

Every `reqwest::Client` in the project should follow this initialization pattern:

```rust
pub fn new() -> Self {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

    Self {
        client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .default_headers(headers)
            .danger_accept_invalid_certs(true) // For debugging/local environments
            .no_proxy() // Avoid system proxy interference
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .expect("Failed to create HTTP client"),
    }
}
```

## Error Handling Pattern

Don't just use `?` on the final `.send().await`. Wrap the call to catch specific network states:

```rust
let response = match self.client.get(url).send().await {
    Ok(resp) => resp,
    Err(e) => {
        if e.is_timeout() {
            anyhow::bail!("Request timeout: API did not respond in 60s");
        } else if e.is_connect() {
            anyhow::bail!("Connection error: Could not reach the server");
        } else {
            anyhow::bail!("Network error: {}", e);
        }
    }
};
```

## Impact

- Reduced "403 Forbidden" or "Connection Reset" errors caused by missing User-Agents.
- Much faster troubleshooting thanks to localized, specific error messages in logs.



