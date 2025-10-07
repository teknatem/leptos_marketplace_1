use super::view_model::Connection1CDetailsViewModel;
use crate::shared::icons::icon;
use leptos::prelude::*;
use std::rc::Rc;

#[component]
pub fn Connection1CDetails(
    id: Option<String>,
    on_saved: Rc<dyn Fn(())>,
    on_cancel: Rc<dyn Fn(())>,
) -> impl IntoView {
    let vm = Connection1CDetailsViewModel::new();
    vm.load_if_needed(id);

    // Clone vm for multiple closures
    let vm_clone = vm.clone();

    view! {
        <div class="details-container connection-1c-details">
            <div class="details-header">
                <h3>
                    {
                        let vm = vm_clone.clone();
                        move || if vm.is_edit_mode()() { "Edit Connection" } else { "New Connection" }
                    }
                </h3>
            </div>

            {
                let vm = vm_clone.clone();
                move || vm.error.get().map(|e| view! { <div class="error">{e}</div> })
            }

            <div class="details-form">
                <div class="form-group">
                    <label for="description">{"Description"}</label>
                    <input
                        type="text"
                        id="description"
                        prop:value={
                            let vm = vm_clone.clone();
                            move || vm.form.get().description
                        }
                        on:input={
                            let vm = vm_clone.clone();
                            move |ev| {
                                vm.form.update(|f| f.description = event_target_value(&ev));
                            }
                        }
                        placeholder="Enter connection description"
                    />
                </div>

                <div class="form-group">
                    <label for="url">{"URL"}</label>
                    <input
                        type="url"
                        id="url"
                        prop:value={
                            let vm = vm_clone.clone();
                            move || vm.form.get().url
                        }
                        on:input={
                            let vm = vm_clone.clone();
                            move |ev| {
                                vm.form.update(|f| f.url = event_target_value(&ev));
                            }
                        }
                        placeholder="Enter 1C database URL"
                    />
                </div>

                <div class="form-group">
                    <label for="login">{"Login"}</label>
                    <input
                        type="text"
                        id="login"
                        prop:value={
                            let vm = vm_clone.clone();
                            move || vm.form.get().login
                        }
                        on:input={
                            let vm = vm_clone.clone();
                            move |ev| {
                                vm.form.update(|f| f.login = event_target_value(&ev));
                            }
                        }
                        placeholder="Enter login"
                    />
                </div>

                <div class="form-group">
                    <label for="password">{"Password"}</label>
                    <input
                        type="password"
                        id="password"
                        prop:value={
                            let vm = vm_clone.clone();
                            move || vm.form.get().password
                        }
                        on:input={
                            let vm = vm_clone.clone();
                            move |ev| {
                                vm.form.update(|f| f.password = event_target_value(&ev));
                            }
                        }
                        placeholder="Enter password"
                    />
                </div>

                <div class="form-group">
                    <label for="comment">{"Comment"}</label>
                    <textarea
                        id="comment"
                        prop:value={
                            let vm = vm_clone.clone();
                            move || vm.form.get().comment.clone().unwrap_or_default()
                        }
                        on:input={
                            let vm = vm_clone.clone();
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
                                let vm = vm_clone.clone();
                                move || vm.form.get().is_primary
                            }
                            on:change={
                                let vm = vm_clone.clone();
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

            <div class="details-actions">
                <button
                    class="btn btn-test"
                    on:click={
                        let vm = vm_clone.clone();
                        move |_| vm.test_command()
                    }
                    disabled={
                        let vm = vm_clone.clone();
                        move || vm.is_testing.get()
                    }
                >
                    {icon("check")}
                    {
                        let vm = vm_clone.clone();
                        move || if vm.is_testing.get() { "Testing..." } else { "Test Connection" }
                    }
                </button>
                <button
                    class="btn btn-primary"
                    on:click={
                        let vm = vm_clone.clone();
                        let on_saved = on_saved.clone();
                        move |_| vm.save_command(on_saved.clone())
                    }
                    disabled={
                        let vm = vm_clone.clone();
                        move || !vm.is_form_valid()()
                    }
                >
                    {icon("save")}
                    {
                        let vm = vm_clone.clone();
                        move || if vm.is_edit_mode()() { "Update" } else { "Create" }
                    }
                </button>
                <button
                    class="btn btn-secondary"
                    on:click=move |_| (on_cancel)(())
                >
                    {icon("cancel")}
                    {"Cancel"}
                </button>
            </div>

            {
                let vm = vm_clone.clone();
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
            }}
        </div>
    }
}
