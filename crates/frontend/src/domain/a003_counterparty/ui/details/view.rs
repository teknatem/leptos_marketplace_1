use super::view_model::CounterpartyDetailsViewModel;
use leptos::prelude::*;
use std::rc::Rc;

#[component]
pub fn CounterpartyDetails(
    id: Option<String>,
    on_saved: Rc<dyn Fn(())>,
    on_cancel: Rc<dyn Fn(())>,
) -> impl IntoView {
    let vm = CounterpartyDetailsViewModel::new();
    vm.load_if_needed(id);

    let vm_clone = vm.clone();

    view! {
        <div class="details-container">
            <div class="details-header">
                <h3>
                    {
                        let vm = vm_clone.clone();
                        move || if vm.is_edit_mode()() { "Редактирование контрагента" } else { "Новый контрагент" }
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
                        placeholder="Введите наименование"
                    />
                </div>

                <div class="form-group">
                    <label for="code">{"Код"}</label>
                    <input
                        type="text"
                        id="code"
                        prop:value={
                            let vm = vm_clone.clone();
                            move || vm.form.get().code.clone().unwrap_or_default()
                        }
                        on:input={
                            let vm = vm_clone.clone();
                            move |ev| {
                                vm.form.update(|f| f.code = Some(event_target_value(&ev)));
                            }
                        }
                        placeholder="Введите код (необязательно)"
                    />
                </div>

                <div class="form-group">
                    <label for="is_folder">{"Это папка"}</label>
                    <input
                        type="checkbox"
                        id="is_folder"
                        prop:checked={
                            let vm = vm_clone.clone();
                            move || vm.form.get().is_folder
                        }
                        on:change={
                            let vm = vm_clone.clone();
                            move |ev| {
                                vm.form.update(|f| f.is_folder = event_target_checked(&ev));
                            }
                        }
                    />
                </div>

                <div class="form-group">
                    <label for="parent_id">{"Родитель (UUID)"}</label>
                    <input
                        type="text"
                        id="parent_id"
                        prop:value={
                            let vm = vm_clone.clone();
                            move || vm.form.get().parent_id.clone().unwrap_or_default()
                        }
                        on:input={
                            let vm = vm_clone.clone();
                            move |ev| {
                                let v = event_target_value(&ev);
                                vm.form.update(|f| f.parent_id = if v.trim().is_empty() { None } else { Some(v) });
                            }
                        }
                        placeholder="UUID родителя (опционально)"
                    />
                </div>

                <div class="form-group">
                    <label for="inn">{"ИНН"}</label>
                    <input
                        type="text"
                        id="inn"
                        prop:value={
                            let vm = vm_clone.clone();
                            move || vm.form.get().inn.clone().unwrap_or_default()
                        }
                        on:input={
                            let vm = vm_clone.clone();
                            move |ev| {
                                let v = event_target_value(&ev);
                                vm.form.update(|f| f.inn = if v.trim().is_empty() { None } else { Some(v) });
                            }
                        }
                        placeholder="ИНН (опционально)"
                    />
                </div>

                <div class="form-group">
                    <label for="kpp">{"КПП"}</label>
                    <input
                        type="text"
                        id="kpp"
                        prop:value={
                            let vm = vm_clone.clone();
                            move || vm.form.get().kpp.clone().unwrap_or_default()
                        }
                        on:input={
                            let vm = vm_clone.clone();
                            move |ev| {
                                let v = event_target_value(&ev);
                                vm.form.update(|f| f.kpp = if v.trim().is_empty() { None } else { Some(v) });
                            }
                        }
                        placeholder="КПП (опционально)"
                    />
                </div>

                <div class="form-group">
                    <label for="comment">{"Комментарий"}</label>
                    <textarea id="comment"
                        prop:value={
                            let vm = vm_clone.clone();
                            move || vm.form.get().comment.clone().unwrap_or_default()
                        }
                        on:input={
                            let vm = vm_clone.clone();
                            move |ev| {
                                let v = event_target_value(&ev);
                                vm.form.update(|f| f.comment = if v.trim().is_empty() { None } else { Some(v) });
                            }
                        }
                    />
                </div>

                {
                    let vm = vm_clone.clone();
                    move || {
                        if let Some(updated_at) = vm.form.get().updated_at {
                            view! {
                                <div class="form-group">
                                    <label>{"Последнее обновление"}</label>
                                    <div class="readonly-field">
                                        {format!("{}", updated_at.format("%Y-%m-%d %H:%M:%S"))}
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }
                    }
                }

                <div class="form-actions">
                    <button class="btn btn-primary"
                        disabled={
                            let vm = vm_clone.clone();
                            move || !vm.is_form_valid()()
                        }
                        on:click={
                            let vm = vm_clone.clone();
                            let on_saved = on_saved.clone();
                            move |_| {
                                vm.save_command(on_saved.clone())();
                            }
                        }
                    >{"💾 Сохранить"}</button>
                    <button class="btn btn-secondary" on:click=move |_| on_cancel(())>{"Отмена"}</button>
                </div>
            </div>
        </div>
    }
}
