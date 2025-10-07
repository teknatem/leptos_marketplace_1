use super::view_model::MarketplaceDetailsViewModel;
use crate::shared::icons::icon;
use leptos::prelude::*;
use std::rc::Rc;

#[component]
pub fn MarketplaceDetails(
    id: Option<String>,
    on_saved: Rc<dyn Fn(())>,
    on_cancel: Rc<dyn Fn(())>,
) -> impl IntoView {
    let vm = MarketplaceDetailsViewModel::new();
    vm.load_if_needed(id);

    // Clone vm for multiple closures
    let vm_clone = vm.clone();

    view! {
        <div class="details-container marketplace-details">
            <div class="details-header">
                <h3>
                    {
                        let vm = vm_clone.clone();
                        move || if vm.is_edit_mode()() { "Редактирование маркетплейса" } else { "Новый маркетплейс" }
                    }
                </h3>
            </div>

            {
                let vm = vm_clone.clone();
                move || vm.error.get().map(|e| view! { <div class="error">{e}</div> })
            }

            <div class="details-form">
                <div class="form-group">
                    <label for="description">{"Наименование"}</label>
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
                        placeholder="Введите наименование маркетплейса"
                    />
                </div>

                <div class="form-group">
                    <label for="url">{"URL"}</label>
                    <input
                        type="text"
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
                        placeholder="https://example.com"
                    />
                </div>

                <div class="form-group">
                    <label for="logo_path">{"Путь к логотипу"}</label>
                    <input
                        type="text"
                        id="logo_path"
                        prop:value={
                            let vm = vm_clone.clone();
                            move || vm.form.get().logo_path.clone().unwrap_or_default()
                        }
                        on:input={
                            let vm = vm_clone.clone();
                            move |ev| {
                                let value = event_target_value(&ev);
                                vm.form.update(|f| {
                                    f.logo_path = if value.is_empty() { None } else { Some(value) };
                                });
                            }
                        }
                        placeholder="/assets/images/logo.svg"
                    />
                </div>

                <div class="form-group">
                    <label for="comment">{"Комментарий"}</label>
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
                        placeholder="Введите дополнительную информацию (необязательно)"
                        rows="3"
                    />
                </div>
            </div>

            <div class="details-actions">
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
                        move || if vm.is_edit_mode()() { "Сохранить" } else { "Создать" }
                    }
                </button>
                <button
                    class="btn btn-secondary"
                    on:click=move |_| (on_cancel)(())
                >
                    {icon("cancel")}
                    {"Отмена"}
                </button>
            </div>
        </div>
    }
}
