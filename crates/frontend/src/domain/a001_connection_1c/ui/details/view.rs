use super::view_model::Connection1CDetailsViewModel;
use crate::shared::icons::icon;
use crate::shared::modal::Modal;
use leptos::prelude::*;

#[component]
pub fn Connection1CDetails(
    id: Signal<Option<String>>,
    show: Signal<bool>,
    on_saved: Callback<()>,
    on_close: Callback<()>,
) -> impl IntoView {
    let vm = Connection1CDetailsViewModel::new();

    // Load data when modal opens or id changes
    {
        let vm_for_effect = vm.clone();
        Effect::new(move |_| {
            if show.get() {
                vm_for_effect.load_if_needed(id.get());
            }
        });
    }

    // Clone vm for multiple closures
    let vm_clone = vm.clone();

    view! {
        <Show when=move || show.get()>
            {
                let vm_for_content = vm_clone.clone();
                move || {
                    let vm = vm_for_content.clone();
                    let is_edit = vm.is_edit_mode()();
                    let modal_title = if is_edit {
                        "Edit 1C Connection".to_string()
                    } else {
                        "New 1C Connection".to_string()
                    };

                    view! {
                        <Modal
                            title="".to_string()
                            on_close=on_close
                        >
                            <div class="connection-1c-details">
                                // Custom header with title and action buttons in one line
                                <div class="modal-header" style="border-bottom: 1px solid var(--color-border-light); margin-bottom: var(--spacing-lg); padding: var(--spacing-md); margin: calc(var(--spacing-md) * -1) calc(var(--spacing-md) * -1) var(--spacing-lg) calc(var(--spacing-md) * -1);">
                                    <h2 class="modal-title">{modal_title}</h2>
                                    <div class="modal-header-actions" style="display: flex; gap: var(--spacing-sm); align-items: center;">
                                        <button
                                            class="btn btn-test"
                                            on:click={
                                                let vm = vm.clone();
                                                move |_| vm.test_command()
                                            }
                                            disabled={
                                                let vm = vm.clone();
                                                move || vm.is_testing.get()
                                            }
                                        >
                                            {icon("check")}
                                            {
                                                let vm = vm.clone();
                                                move || if vm.is_testing.get() { "Testing..." } else { "Test" }
                                            }
                                        </button>
                                        <button
                                            class="btn btn-primary"
                                            on:click={
                                                let vm = vm.clone();
                                                move |_| vm.save_command(on_saved)
                                            }
                                            disabled={
                                                let vm = vm.clone();
                                                move || !vm.is_form_valid()()
                                            }
                                        >
                                            {icon("save")}
                                            "Save"
                                        </button>
                                        <button
                                            class="btn btn-ghost btn-close"
                                            on:click=move |_| on_close.run(())
                                        >
                                            {icon("x")}
                                        </button>
                                    </div>
                                </div>
                                {
                                    let vm = vm.clone();
                                    move || vm.error.get().map(|e| view! { <div class="error">{e}</div> })
                                }

                                <div class="details-form">
                                    <div class="form-group">
                                        <label class="form-label" for="description">{"Description"}</label>
                                        <input
                                            class="form-input"
                                            type="text"
                                            id="description"
                                            prop:value={
                                                let vm = vm.clone();
                                                move || vm.form.get().description
                                            }
                                            on:input={
                                                let vm = vm.clone();
                                                move |ev| {
                                                    vm.form.update(|f| f.description = event_target_value(&ev));
                                                }
                                            }
                                            placeholder="Enter connection description"
                                        />
                                    </div>

                                    <div class="form-group">
                                        <label class="form-label" for="url">{"URL"}</label>
                                        <input
                                            class="form-input"
                                            type="url"
                                            id="url"
                                            prop:value={
                                                let vm = vm.clone();
                                                move || vm.form.get().url
                                            }
                                            on:input={
                                                let vm = vm.clone();
                                                move |ev| {
                                                    vm.form.update(|f| f.url = event_target_value(&ev));
                                                }
                                            }
                                            placeholder="Enter 1C database URL"
                                        />
                                    </div>

                                    <div class="form-group">
                                        <label class="form-label" for="login">{"Login"}</label>
                                        <input
                                            class="form-input"
                                            type="text"
                                            id="login"
                                            prop:value={
                                                let vm = vm.clone();
                                                move || vm.form.get().login
                                            }
                                            on:input={
                                                let vm = vm.clone();
                                                move |ev| {
                                                    vm.form.update(|f| f.login = event_target_value(&ev));
                                                }
                                            }
                                            placeholder="Enter login"
                                        />
                                    </div>

                                    <div class="form-group">
                                        <label class="form-label" for="password">{"Password"}</label>
                                        <input
                                            class="form-input"
                                            type="password"
                                            id="password"
                                            prop:value={
                                                let vm = vm.clone();
                                                move || vm.form.get().password
                                            }
                                            on:input={
                                                let vm = vm.clone();
                                                move |ev| {
                                                    vm.form.update(|f| f.password = event_target_value(&ev));
                                                }
                                            }
                                            placeholder="Enter password"
                                        />
                                    </div>

                                    <div class="form-group">
                                        <label class="form-label" for="comment">{"Comment"}</label>
                                        <textarea
                                            class="form-textarea"
                                            id="comment"
                                            prop:value={
                                                let vm = vm.clone();
                                                move || vm.form.get().comment.clone().unwrap_or_default()
                                            }
                                            on:input={
                                                let vm = vm.clone();
                                                move |ev| {
                                                    let value = event_target_value(&ev);
                                                    vm.form.update(|f| {
                                                        f.comment = if value.is_empty() { None } else { Some(value) };
                                                    });
                                                }
                                            }
                                            placeholder="Enter optional comment"
                                            rows="3"
                                        />
                                    </div>

                                    <div class="form-group checkbox-group">
                                        <label class="checkbox-label">
                                            <input
                                                type="checkbox"
                                                prop:checked={
                                                    let vm = vm.clone();
                                                    move || vm.form.get().is_primary
                                                }
                                                on:change={
                                                    let vm = vm.clone();
                                                    move |ev| {
                                                        vm.form.update(|f| f.is_primary = event_target_checked(&ev));
                                                    }
                                                }
                                            />
                                            <span class="checkbox-text">{"Primary Connection"}</span>
                                        </label>
                                        <small class="help-text">
                                            {"Only one connection can be marked as primary"}
                                        </small>
                                    </div>
                                </div>

                                {
                                    let vm = vm.clone();
                                    move || {
                                        vm.test_result
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
                                    }
                                }
                            </div>
                        </Modal>
                    }
                }
            }
        </Show>
    }
}
