use super::view_model::OrganizationDetailsViewModel;
use crate::shared::icons::icon;
use leptos::prelude::*;
use std::rc::Rc;

#[component]
pub fn OrganizationDetails(
    id: Option<String>,
    on_saved: Rc<dyn Fn(())>,
    on_cancel: Rc<dyn Fn(())>,
) -> impl IntoView {
    let vm = OrganizationDetailsViewModel::new();
    vm.load_if_needed(id);

    // Clone vm for multiple closures
    let vm_clone = vm.clone();

    view! {
        <div class="details-container organization-details">
            <div class="details-header">
                <h3>
                    {
                        let vm = vm_clone.clone();
                        move || if vm.is_edit_mode()() { "Редактирование организации" } else { "Новая организация" }
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
                        placeholder="Введите краткое наименование"
                    />
                </div>

                <div class="form-group">
                    <label for="full_name">{"Полное наименование"}</label>
                    <input
                        type="text"
                        id="full_name"
                        prop:value={
                            let vm = vm_clone.clone();
                            move || vm.form.get().full_name
                        }
                        on:input={
                            let vm = vm_clone.clone();
                            move |ev| {
                                vm.form.update(|f| f.full_name = event_target_value(&ev));
                            }
                        }
                        placeholder="Введите полное наименование организации"
                    />
                </div>

                <div class="form-group">
                    <label for="inn">{"ИНН"}</label>
                    <input
                        type="text"
                        id="inn"
                        prop:value={
                            let vm = vm_clone.clone();
                            move || vm.form.get().inn
                        }
                        on:input={
                            let vm = vm_clone.clone();
                            move |ev| {
                                vm.form.update(|f| f.inn = event_target_value(&ev));
                            }
                        }
                        placeholder="10 или 12 цифр"
                        maxlength="12"
                    />
                </div>

                <div class="form-group">
                    <label for="kpp">{"КПП"}</label>
                    <input
                        type="text"
                        id="kpp"
                        prop:value={
                            let vm = vm_clone.clone();
                            move || vm.form.get().kpp
                        }
                        on:input={
                            let vm = vm_clone.clone();
                            move |ev| {
                                vm.form.update(|f| f.kpp = event_target_value(&ev));
                            }
                        }
                        placeholder="9 цифр (необязательно для ИП)"
                        maxlength="9"
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
