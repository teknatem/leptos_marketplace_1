use contracts::system::auth::UserInfo;
use leptos::prelude::*;
use leptos::task::spawn_local;

use super::{api, storage};

#[derive(Clone, Debug, Default)]
pub struct AuthState {
    pub access_token: Option<String>,
    pub user_info: Option<UserInfo>,
}

/// Auth context provider component
#[component]
pub fn AuthProvider(children: ChildrenFn) -> impl IntoView {
    let (auth_state, set_auth_state) = create_signal(AuthState::default());

    // Try to restore session from localStorage on mount
    create_effect(move |_| {
        spawn_local(async move {
            if let Some(access_token) = storage::get_access_token() {
                // Validate token by fetching current user
                match api::get_current_user(&access_token).await {
                    Ok(user_info) => {
                        set_auth_state.set(AuthState {
                            access_token: Some(access_token),
                            user_info: Some(user_info),
                        });
                    }
                    Err(_) => {
                        // Token invalid, try refresh
                        if let Some(refresh_token) = storage::get_refresh_token() {
                            match api::refresh_token(refresh_token).await {
                                Ok(response) => {
                                    storage::save_access_token(&response.access_token);

                                    // Get user info with new token
                                    if let Ok(user_info) =
                                        api::get_current_user(&response.access_token).await
                                    {
                                        set_auth_state.set(AuthState {
                                            access_token: Some(response.access_token),
                                            user_info: Some(user_info),
                                        });
                                    }
                                }
                                Err(_) => {
                                    // Refresh failed, clear tokens
                                    storage::clear_tokens();
                                }
                            }
                        } else {
                            // No refresh token, clear everything
                            storage::clear_tokens();
                        }
                    }
                }
            }
        });
    });

    provide_context(auth_state);
    provide_context(set_auth_state);

    children()
}

/// Hook to access auth state
pub fn use_auth() -> (ReadSignal<AuthState>, WriteSignal<AuthState>) {
    let auth_state =
        use_context::<ReadSignal<AuthState>>().expect("AuthProvider not found in component tree");
    let set_auth_state =
        use_context::<WriteSignal<AuthState>>().expect("AuthProvider not found in component tree");

    (auth_state, set_auth_state)
}

/// Helper: Check if user is authenticated
pub fn is_authenticated() -> bool {
    let (auth_state, _) = use_auth();
    auth_state.get().access_token.is_some()
}

/// Helper: Check if user is admin
pub fn is_admin() -> bool {
    let (auth_state, _) = use_auth();
    auth_state
        .get()
        .user_info
        .as_ref()
        .map(|u| u.is_admin)
        .unwrap_or(false)
}

/// Helper: Perform login
pub async fn do_login(username: String, password: String) -> Result<(), String> {
    let response = api::login(username, password).await?;

    // Save tokens
    storage::save_access_token(&response.access_token);
    storage::save_refresh_token(&response.refresh_token);

    // Update auth state
    let (_, set_auth_state) = use_auth();
    set_auth_state.set(AuthState {
        access_token: Some(response.access_token),
        user_info: Some(response.user),
    });

    Ok(())
}

/// Helper: Perform logout
pub async fn do_logout() -> Result<(), String> {
    if let Some(refresh_token) = storage::get_refresh_token() {
        let _ = api::logout(refresh_token).await;
    }

    // Clear tokens
    storage::clear_tokens();

    // Clear auth state
    let (_, set_auth_state) = use_auth();
    set_auth_state.set(AuthState::default());

    Ok(())
}
