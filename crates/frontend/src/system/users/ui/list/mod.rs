use contracts::system::users::User;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::system::{auth::guard::RequireAdmin, users::api};

#[component]
pub fn UsersListPage() -> impl IntoView {
    view! {
        <RequireAdmin>
            <UsersList />
        </RequireAdmin>
    }
}

#[component]
fn UsersList() -> impl IntoView {
    let (users, set_users) = create_signal(Vec::<User>::new());
    let (is_loading, set_is_loading) = create_signal(true);
    let (error_message, set_error_message) = create_signal(Option::<String>::None);
    let (show_create_form, set_show_create_form) = create_signal(false);

    // Load users on mount
    create_effect(move |_| {
        spawn_local(async move {
            match api::fetch_users().await {
                Ok(data) => {
                    set_users.set(data);
                    set_is_loading.set(false);
                }
                Err(e) => {
                    set_error_message.set(Some(format!("Failed to load users: {}", e)));
                    set_is_loading.set(false);
                }
            }
        });
    });

    let on_delete = move |user_id: String| {
        let confirmed = web_sys::window()
            .and_then(|w| w.confirm_with_message("Are you sure you want to delete this user?").ok())
            .unwrap_or(false);

        if confirmed {
            spawn_local(async move {
                match api::delete_user(&user_id).await {
                    Ok(_) => {
                        // Reload users
                        match api::fetch_users().await {
                            Ok(data) => set_users.set(data),
                            Err(e) => set_error_message.set(Some(format!("Failed to reload: {}", e))),
                        }
                    }
                    Err(e) => {
                        set_error_message.set(Some(format!("Failed to delete user: {}", e)));
                    }
                }
            });
        }
    };

    view! {
        <div class="users-management">
            <div class="page-header">
                <h2>"Управление пользователями"</h2>
                <button
                    class="btn-primary"
                    on:click=move |_| set_show_create_form.set(true)
                >
                    "Добавить пользователя"
                </button>
            </div>

            <Show when=move || error_message.get().is_some()>
                <div class="error-message">
                    {move || error_message.get().unwrap_or_default()}
                </div>
            </Show>

            <Show
                when=move || show_create_form.get()
                fallback=|| view! { <div></div> }
            >
                <super::details::CreateUserForm
                    on_close=move || set_show_create_form.set(false)
                    on_created=move || {
                        set_show_create_form.set(false);
                        // Reload users
                        spawn_local(async move {
                            if let Ok(data) = api::fetch_users().await {
                                set_users.set(data);
                            }
                        });
                    }
                />
            </Show>

            <Show
                when=move || !is_loading.get()
                fallback=|| view! { <div>"Loading users..."</div> }
            >
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>"Username"</th>
                            <th>"Full Name"</th>
                            <th>"Email"</th>
                            <th>"Admin"</th>
                            <th>"Active"</th>
                            <th>"Last Login"</th>
                            <th>"Actions"</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || users.get()
                            key=|user| user.id.clone()
                            let:user
                        >
                            {
                                let user_id = user.id.clone();
                                view! {
                                    <tr>
                                        <td>{user.username.clone()}</td>
                                        <td>{user.full_name.clone().unwrap_or_default()}</td>
                                        <td>{user.email.clone().unwrap_or_default()}</td>
                                        <td>{if user.is_admin { "Yes" } else { "No" }}</td>
                                        <td>{if user.is_active { "Active" } else { "Inactive" }}</td>
                                        <td>{user.last_login_at.clone().unwrap_or_default()}</td>
                                        <td>
                                            <button
                                                class="btn-danger btn-small"
                                                on:click=move |_| on_delete(user_id.clone())
                                            >
                                                "Delete"
                                            </button>
                                        </td>
                                    </tr>
                                }
                            }
                        </For>
                    </tbody>
                </table>
            </Show>
        </div>
    }
}

