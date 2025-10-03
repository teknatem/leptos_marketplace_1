use crate::domain::connection_1c::ui::details::model::{
    fetch_by_id, save_form, test_connection, validate_form, FormState,
};
use contracts::domain::connection_1c::aggregate::ConnectionTestResult;
use leptos::prelude::*;
use std::rc::Rc;

#[component]
pub fn Connection1CDetails(
    id: Option<i32>,
    on_saved: Rc<dyn Fn(())>,
    on_cancel: Rc<dyn Fn(())>,
) -> impl IntoView {
    let form_state = RwSignal::new(FormState::default());
    let (error, set_error) = signal::<Option<String>>(None);
    let (test_result, set_test_result) = signal::<Option<ConnectionTestResult>>(None);
    let (is_testing, set_is_testing) = signal(false);
    let is_edit_mode = move || form_state.get().id.is_some();

    if let Some(existing_id) = id {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_by_id(existing_id).await {
                Ok(conn) => form_state.set(FormState::from(conn)),
                Err(e) => set_error.set(Some(format!("Failed to load: {}", e))),
            }
        });
    }

    let handle_save = move || {
        let current = form_state.get();
        match validate_form(&current) {
            Ok(()) => {
                let cloned = current.clone();
                let on_saved_cb = on_saved.clone();
                let set_error_cb = set_error.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match save_form(&cloned).await {
                        Ok(()) => (on_saved_cb)(()),
                        Err(e) => set_error_cb.set(Some(e)),
                    }
                });
            }
            Err(msg) => set_error.set(Some(msg)),
        }
    };

    let handle_test = move || {
        set_is_testing.set(true);
        set_test_result.set(None);
        set_error.set(None);

        let current = form_state.get();
        wasm_bindgen_futures::spawn_local(async move {
            match test_connection(&current).await {
                Ok(result) => {
                    set_test_result.set(Some(result));
                    set_is_testing.set(false);
                }
                Err(e) => {
                    set_error.set(Some(format!("Test failed: {}", e)));
                    set_is_testing.set(false);
                }
            }
        });
    };

    view! {
        <div class="connection-1c-details">
            <div class="details-header">
                <h3>
                    {move || if is_edit_mode() { "Edit Connection" } else { "New Connection" }}
                </h3>
            </div>

            {move || error.get().map(|e| view! { <div class="error">{e}</div> })}

            <div class="details-form">
                <div class="form-group">
                    <label for="description">{"Description"}</label>
                    <input
                        type="text"
                        id="description"
                        prop:value=move || form_state.get().description
                        on:input=move |ev| {
                            form_state.update(|state| {
                                state.description = event_target_value(&ev);
                            });
                        }
                        placeholder="Enter connection description"
                    />
                </div>

                <div class="form-group">
                    <label for="url">{"URL"}</label>
                    <input
                        type="url"
                        id="url"
                        prop:value=move || form_state.get().url
                        on:input=move |ev| {
                            form_state.update(|state| {
                                state.url = event_target_value(&ev);
                            });
                        }
                        placeholder="Enter 1C database URL"
                    />
                </div>

                <div class="form-group">
                    <label for="login">{"Login"}</label>
                    <input
                        type="text"
                        id="login"
                        prop:value=move || form_state.get().login
                        on:input=move |ev| {
                            form_state.update(|state| {
                                state.login = event_target_value(&ev);
                            });
                        }
                        placeholder="Enter login"
                    />
                </div>

                <div class="form-group">
                    <label for="password">{"Password"}</label>
                    <input
                        type="password"
                        id="password"
                        prop:value=move || form_state.get().password
                        on:input=move |ev| {
                            form_state.update(|state| {
                                state.password = event_target_value(&ev);
                            });
                        }
                        placeholder="Enter password"
                    />
                </div>

                <div class="form-group">
                    <label for="comment">{"Comment"}</label>
                    <textarea
                        id="comment"
                        prop:value=move || form_state.get().comment
                        on:input=move |ev| {
                            form_state.update(|state| {
                                state.comment = event_target_value(&ev);
                            });
                        }
                        placeholder="Enter optional comment"
                        rows="3"
                    />
                </div>

                <div class="form-group checkbox-group">
                    <label class="checkbox-label">
                        <input
                            type="checkbox"
                            prop:checked=move || form_state.get().is_primary
                            on:change=move |ev| {
                                form_state.update(|state| {
                                    state.is_primary = event_target_checked(&ev);
                                });
                            }
                        />
                        <span class="checkbox-text">{"Primary Connection"}</span>
                    </label>
                    <small class="help-text">
                        {"Only one connection can be marked as primary"}
                    </small>
                </div>
            </div>

            <div class="details-actions">
                <button
                    class="btn btn-test"
                    on:click=move |_| handle_test()
                    disabled=move || is_testing.get()
                >
                    {move || if is_testing.get() { "Testing..." } else { "Test Connection" }}
                </button>
                <button
                    class="btn btn-primary"
                    on:click=move |_| handle_save()
                    disabled=move || validate_form(&form_state.get()).is_err()
                >
                    {move || if is_edit_mode() { "Update" } else { "Create" }}
                </button>
                <button
                    class="btn btn-secondary"
                    on:click=move |_| (on_cancel)(())
                >
                    {"Cancel"}
                </button>
            </div>

            {move || {
                test_result
                    .get()
                    .map(|result| {
                        let style = if result.success {
                            "color: green; margin-top: 10px;"
                        } else {
                            "color: red; margin-top: 10px;"
                        };
                        let icon = if result.success { "✓" } else { "✗" };
                        view! {
                            <div style=style>
                                {format!(
                                    "{} {} ({}ms)",
                                    icon,
                                    result.message,
                                    result.duration_ms
                                )}
                            </div>
                        }
                    })
            }}
        </div>
    }
}
