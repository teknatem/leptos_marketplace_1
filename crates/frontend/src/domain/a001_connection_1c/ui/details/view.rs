use super::view_model::Connection1CDetailsViewModel;
use crate::shared::icons::icon;
use crate::shared::modal::Modal;
use leptos::prelude::*;

#[component]
pub fn Connection1CDetails(
    id: ReadSignal<Option<String>>,
    on_saved: Callback<()>,
    on_close: Callback<()>,
) -> impl IntoView {
    let vm = Connection1CDetailsViewModel::new();

    // Load data when id changes
    {
        let vm_for_effect = vm.clone();
        Effect::new(move |_| {
            let current_id = id.get();
            if current_id.is_some() {
                let id_to_load = if current_id.as_ref().map(|s| s.is_empty()).unwrap_or(false) {
                    None // Создание нового
                } else {
                    current_id // Редактирование существующего
                };
                vm_for_effect.load_or_reset(id_to_load);
            }
        });
    }

    view! {
        <Show when=move || id.get().is_some()>
            {
                let vm = vm.clone();
                move || {
                    // Create all clones needed before view! macro
                    let vm_test_click = vm.clone();
                    let vm_test_disabled = vm.clone();
                    let vm_test_label = vm.clone();
                    let vm_save_click = vm.clone();
                    let vm_save_disabled = vm.clone();
                    let vm_error = vm.clone();
                    let vm_desc_value = vm.clone();
                    let vm_desc_input = vm.clone();
                    let vm_url_value = vm.clone();
                    let vm_url_input = vm.clone();
                    let vm_login_value = vm.clone();
                    let vm_login_input = vm.clone();
                    let vm_password_value = vm.clone();
                    let vm_password_input = vm.clone();
                    let vm_comment_value = vm.clone();
                    let vm_comment_input = vm.clone();
                    let vm_primary_checked = vm.clone();
                    let vm_primary_change = vm.clone();
                    let vm_test_result = vm.clone();

                    let is_edit = vm.is_edit_mode()();
                    let modal_title = if is_edit {
                        "Edit 1C Connection"
                    } else {
                        "New 1C Connection"
                    };

                    view! {
                        <Modal title=modal_title.to_string() on_close=on_close>
                        // Action buttons at the top
                        <div class="modal-actions-top">
            <button
                class="button button--secondary"
                on:click=move |_| vm_test_click.test_command()
                disabled=move || vm_test_disabled.is_testing.get()
            >
                {icon("check")}
                {move || if vm_test_label.is_testing.get() { "Testing..." } else { "Test" }}
            </button>
            <button
                class="button button--primary"
                on:click=move |_| vm_save_click.save_command(on_saved)
                disabled=move || !vm_save_disabled.is_form_valid()()
            >
                {icon("save")}
                "Save"
            </button>
        </div>

        {move || vm_error.error.get().map(|e| view! { <div class="warning-box text-error">{e}</div> })}

        <div class="detail-form">
            <div class="form__group">
                <label class="form__label" for="description">{"Description"}</label>
                <input
                    class="form__input"
                    type="text"
                    id="description"
                    prop:value=move || vm_desc_value.form.get().description
                    on:input=move |ev| {
                        vm_desc_input.form.update(|f| f.description = event_target_value(&ev));
                    }
                    placeholder="Enter connection description"
                />
            </div>

            <div class="form__group">
                <label class="form__label" for="url">{"URL"}</label>
                <input
                    class="form__input"
                    type="url"
                    id="url"
                    prop:value=move || vm_url_value.form.get().url
                    on:input=move |ev| {
                        vm_url_input.form.update(|f| f.url = event_target_value(&ev));
                    }
                    placeholder="Enter 1C database URL"
                />
            </div>

            <div class="form__group">
                <label class="form__label" for="login">{"Login"}</label>
                <input
                    class="form__input"
                    type="text"
                    id="login"
                    prop:value=move || vm_login_value.form.get().login
                    on:input=move |ev| {
                        vm_login_input.form.update(|f| f.login = event_target_value(&ev));
                    }
                    placeholder="Enter login"
                />
            </div>

            <div class="form__group">
                <label class="form__label" for="password">{"Password"}</label>
                <input
                    class="form__input"
                    type="password"
                    id="password"
                    prop:value=move || vm_password_value.form.get().password
                    on:input=move |ev| {
                        vm_password_input.form.update(|f| f.password = event_target_value(&ev));
                    }
                    placeholder="Enter password"
                />
            </div>

            <div class="form__group">
                <label class="form__label" for="comment">{"Comment"}</label>
                <textarea
                    class="form__textarea"
                    id="comment"
                    prop:value=move || vm_comment_value.form.get().comment.clone().unwrap_or_default()
                    on:input=move |ev| {
                        let value = event_target_value(&ev);
                        vm_comment_input.form.update(|f| {
                            f.comment = if value.is_empty() { None } else { Some(value) };
                        });
                    }
                    placeholder="Enter optional comment"
                    rows="3"
                />
            </div>

            <div class="form-group checkbox-group">
                <label class="form__checkbox-wrapper">
                    <input
                        type="checkbox"
                        prop:checked=move || vm_primary_checked.form.get().is_primary
                        on:change=move |ev| {
                            vm_primary_change.form.update(|f| f.is_primary = event_target_checked(&ev));
                        }
                    />
                    <span class="form__checkbox-label">{"Primary Connection"}</span>
                </label>
                <small class="help-text">
                    {"Only one connection can be marked as primary"}
                </small>
            </div>
        </div>

        {move || {
            vm_test_result.test_result
                .get()
                .map(|result| {
                    let class = if result.success {
                        "info-box text-success"
                    } else {
                        "warning-box text-error"
                    };
                    let icon = if result.success { "✓" } else { "✗" };
                    view! {
                        <div class=class>
                            {format!(
                                "{} {} ({}ms)",
                                icon,
                                result.message,
                                result.duration_ms
                            )}
                        </div>
                    }
                    })
            }
        }
                    </Modal>
                }
            }
        }
        </Show>
    }
}
