use web_sys::window;

const ACCESS_TOKEN_KEY: &str = "auth_access_token";
const REFRESH_TOKEN_KEY: &str = "auth_refresh_token";

fn get_local_storage() -> Option<web_sys::Storage> {
    window()?.local_storage().ok()?
}

/// Save access token to localStorage
pub fn save_access_token(token: &str) {
    if let Some(storage) = get_local_storage() {
        let _ = storage.set_item(ACCESS_TOKEN_KEY, token);
    }
}

/// Get access token from localStorage
pub fn get_access_token() -> Option<String> {
    get_local_storage()?.get_item(ACCESS_TOKEN_KEY).ok()?
}

/// Save refresh token to localStorage
pub fn save_refresh_token(token: &str) {
    if let Some(storage) = get_local_storage() {
        let _ = storage.set_item(REFRESH_TOKEN_KEY, token);
    }
}

/// Get refresh token from localStorage
pub fn get_refresh_token() -> Option<String> {
    get_local_storage()?.get_item(REFRESH_TOKEN_KEY).ok()?
}

/// Clear all authentication tokens
pub fn clear_tokens() {
    if let Some(storage) = get_local_storage() {
        let _ = storage.remove_item(ACCESS_TOKEN_KEY);
        let _ = storage.remove_item(REFRESH_TOKEN_KEY);
    }
}
