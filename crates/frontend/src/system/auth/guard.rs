use leptos::prelude::*;

use super::context::use_auth;

/// Component that requires authentication
/// Shows fallback if not authenticated
#[component]
pub fn RequireAuth(
    #[prop(optional)] redirect_to: Option<String>,
    children: ChildrenFn,
) -> impl IntoView {
    let (auth_state, _) = use_auth();

    view! {
        <Show
            when=move || auth_state.get().access_token.is_some()
            fallback=|| view! { <div>"Not authenticated. Please login."</div> }
        >
            {children()}
        </Show>
    }
}

/// Component that requires admin privileges
/// Shows fallback if not admin
#[component]
pub fn RequireAdmin(
    #[prop(optional)] redirect_to: Option<String>,
    children: ChildrenFn,
) -> impl IntoView {
    let (auth_state, _) = use_auth();

    view! {
        <Show
            when=move || {
                let state = auth_state.get();
                state.access_token.is_some() && state.user_info.as_ref().map(|u| u.is_admin).unwrap_or(false)
            }
            fallback=|| view! { <div>"Access denied. Admin privileges required."</div> }
        >
            {children()}
        </Show>
    }
}
