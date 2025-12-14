use super::view_model::MarketplaceProductDetailsViewModel;
use crate::domain::a004_nomenclature::ui::picker::NomenclaturePicker;
use crate::shared::icons::icon;
use leptos::prelude::*;
use std::rc::Rc;

#[component]
pub fn MarketplaceProductDetails(
    id: Option<String>,
    on_saved: Rc<dyn Fn(())>,
    on_cancel: Rc<dyn Fn(())>,
) -> impl IntoView {
    let vm = MarketplaceProductDetailsViewModel::new();
    vm.load_if_needed(id);

    let vm_clone = vm.clone();

    view! {
        <div style="display: flex; justify-content: center; padding: 20px;">
            <div class="details-container marketplace-product-details" style="width: 90%; min-width: 600px; max-width: 1500px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); border-radius: 8px; padding: 24px;">
                <div class="details-header" style="margin-bottom: 24px; border-bottom: 2px solid var(--color-primary, #4a90e2); padding-bottom: 12px; display: flex; justify-content: space-between; align-items: center;">
                    <h3 style="margin: 0; color: var(--color-primary, #4a90e2); font-size: 1.2rem;">
                        {
                            let vm = vm_clone.clone();
                            move || if vm.is_edit_mode()() { "Редактирование позиции маркетплейса" } else { "Новый товар маркетплейса" }
                        }
                    </h3>
                    <div style="display: flex; gap: 12px;">
                        <button
                            class="button button--primary"
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
                            class="button button--secondary"
                            on:click=move |_| (on_cancel)(())
                        >
                            {icon("cancel")}
                            {"Отмена"}
                        </button>
                    </div>
                </div>

            {
                let vm = vm_clone.clone();
                move || vm.error.get().map(|e| view! { <div class="error">{e}</div> })
            }

            {
                let vm = vm_clone.clone();
                move || vm.success_message.get().map(|msg| view! { <div class="success">{msg}</div> })
            }

            <div class="detail-form">
                <div class="form__group">
                    <label for="description">{"Описание"}</label>
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
                        placeholder="Краткое описание товара"
                    />
                </div>

                <div class="form-row">
                    <div class="form__group">
                        <label for="marketplace_ref">{"Маркетплейс"}</label>
                        <input
                            type="text"
                            id="marketplace_ref"
                            disabled
                            prop:value={
                                let vm = vm_clone.clone();
                                move || {
                                    let name = vm.marketplace_name.get();
                                    if name.is_empty() {
                                        "Загрузка...".to_string()
                                    } else {
                                        name
                                    }
                                }
                            }
                            placeholder="Маркетплейс"
                        />
                    </div>

                    <div class="form__group">
                        <label for="connection_mp_ref">{"Кабинет"}</label>
                        <input
                            type="text"
                            id="connection_mp_ref"
                            disabled
                            prop:value={
                                let vm = vm_clone.clone();
                                move || {
                                    let name = vm.connection_name.get();
                                    if name.is_empty() {
                                        "Загрузка...".to_string()
                                    } else {
                                        name
                                    }
                                }
                            }
                            placeholder="Кабинет"
                        />
                    </div>
                </div>

                <div class="form-row">
                    <div class="form__group">
                        <label for="marketplace_sku">{"SKU маркетплейса"}</label>
                        <input
                            type="text"
                            id="marketplace_sku"
                            prop:value={
                                let vm = vm_clone.clone();
                                move || vm.form.get().marketplace_sku
                            }
                            on:input={
                                let vm = vm_clone.clone();
                                move |ev| {
                                    vm.form.update(|f| f.marketplace_sku = event_target_value(&ev));
                                }
                            }
                            placeholder="Внутренний ID товара"
                        />
                    </div>
                </div>

                <div class="form-row">
                    <div class="form__group">
                        <label for="article">{"Артикул"}</label>
                        <input
                            type="text"
                            id="article"
                            prop:value={
                                let vm = vm_clone.clone();
                                move || vm.form.get().article
                            }
                            on:input={
                                let vm = vm_clone.clone();
                                move |ev| {
                                    vm.form.update(|f| f.article = event_target_value(&ev));
                                }
                            }
                            placeholder="Артикул товара"
                        />
                    </div>

                    <div class="form__group">
                        <label for="barcode">{"Штрихкод"}</label>
                        <input
                            type="text"
                            id="barcode"
                            prop:value={
                                let vm = vm_clone.clone();
                                move || vm.form.get().barcode.clone().unwrap_or_default()
                            }
                            on:input={
                                let vm = vm_clone.clone();
                                move |ev| {
                                    let value = event_target_value(&ev);
                                    vm.form.update(|f| {
                                        f.barcode = if value.is_empty() { None } else { Some(value) };
                                    });
                                }
                            }
                            placeholder="Штрихкод (необязательно)"
                        />
                    </div>
                </div>

                <div class="form-row">
                    <div class="form__group">
                        <label for="brand">{"Бренд"}</label>
                        <input
                            type="text"
                            id="brand"
                            prop:value={
                                let vm = vm_clone.clone();
                                move || vm.form.get().brand.clone().unwrap_or_default()
                            }
                            on:input={
                                let vm = vm_clone.clone();
                                move |ev| {
                                    let value = event_target_value(&ev);
                                    vm.form.update(|f| {
                                        f.brand = if value.is_empty() { None } else { Some(value) };
                                    });
                                }
                            }
                            placeholder="Бренд товара"
                        />
                    </div>

                    <div class="form__group">
                        <label for="category_name">{"Категория"}</label>
                        <input
                            type="text"
                            id="category_name"
                            prop:value={
                                let vm = vm_clone.clone();
                                move || vm.form.get().category_name.clone().unwrap_or_default()
                            }
                            on:input={
                                let vm = vm_clone.clone();
                                move |ev| {
                                    let value = event_target_value(&ev);
                                    vm.form.update(|f| {
                                        f.category_name = if value.is_empty() { None } else { Some(value) };
                                    });
                                }
                            }
                            placeholder="Название категории"
                        />
                    </div>
                </div>

                <fieldset style={
                    let vm = vm_clone.clone();
                    move || {
                        let bg_color = if vm.form.get().nomenclature_ref.is_some() {
                            "#d4edda" // светло-зеленый
                        } else {
                            "#f8d7da" // светло-розовый
                        };
                        format!("border: 2px solid #ddd; border-radius: 6px; padding: 16px; margin: 16px 0; background: {};", bg_color)
                    }
                }>
                    <legend style="font-weight: 600; color: #333; padding: 0 8px;">{"Номенклатура (1С УТ)"}</legend>

                    <div class="form__group" style="margin-bottom: 12px;">
                        <label for="nomenclature_ref" style="font-weight: 500;">{"Наименование"}</label>
                        <div style="display: flex; gap: 8px; align-items: center;">
                            <input
                                type="text"
                                id="nomenclature_ref"
                                disabled
                                prop:value={
                                    let vm = vm_clone.clone();
                                    move || {
                                        let name = vm.nomenclature_name.get();
                                        if name.is_empty() {
                                            "Не выбрано".to_string()
                                        } else {
                                            name
                                        }
                                    }
                                }
                                placeholder="Номенклатура"
                                style="flex: 1; background: #fff;"
                            />
                            <button
                                type="button"
                                class="button button--small button--primary"
                                on:click={
                                    let vm = vm_clone.clone();
                                    move |_| vm.search_nomenclature_by_article()
                                }
                                disabled={
                                    let vm = vm_clone.clone();
                                    move || vm.form.get().article.trim().is_empty()
                                }
                                title="Поиск по артикулу"
                                style="white-space: nowrap;"
                            >
                                {icon("search")}
                                {"Поиск"}
                            </button>
                            <button
                                type="button"
                                class="button button--small button--secondary"
                                on:click={
                                    let vm = vm_clone.clone();
                                    move |_| vm.open_picker()
                                }
                                title="Выбрать из списка"
                                style="white-space: nowrap;"
                            >
                                {icon("list")}
                                {"Выбрать"}
                            </button>
                            <button
                                type="button"
                                class="button button--small button--secondary"
                                on:click={
                                    let vm = vm_clone.clone();
                                    move |_| vm.clear_nomenclature()
                                }
                                disabled={
                                    let vm = vm_clone.clone();
                                    move || vm.form.get().nomenclature_ref.is_none()
                                }
                                title="Очистить"
                            >
                                {icon("cancel")}
                            </button>
                        </div>
                    </div>

                    <div class="form-row" style="gap: 12px;">
                        <div class="form__group" style="flex: 1;">
                            <label for="nomenclature_code_display" style="font-weight: 500;">{"Код"}</label>
                            <input
                                type="text"
                                id="nomenclature_code_display"
                                disabled
                                prop:value={
                                    let vm = vm_clone.clone();
                                    move || vm.nomenclature_code.get()
                                }
                                placeholder="—"
                                style="background: #fff;"
                            />
                        </div>
                        <div class="form__group" style="flex: 1;">
                            <label for="nomenclature_article_display" style="font-weight: 500;">{"Артикул"}</label>
                            <input
                                type="text"
                                id="nomenclature_article_display"
                                disabled
                                prop:value={
                                    let vm = vm_clone.clone();
                                    move || vm.nomenclature_article.get()
                                }
                                placeholder="—"
                                style="background: #fff;"
                            />
                        </div>
                    </div>
                </fieldset>

                <div class="form__group">
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
                        placeholder="Дополнительная информация (необязательно)"
                        rows="3"
                    />
                </div>
            </div>

            {
                let vm = vm_clone.clone();
                move || {
                    if vm.show_picker.get() {
                        let vm_for_selected = vm.clone();
                        let vm_for_cancel = vm.clone();

                        let on_selected_handler = {
                            let vm = vm_for_selected.clone();
                            move |item: Option<crate::domain::a004_nomenclature::ui::picker::NomenclaturePickerItem>| {
                                if let Some(nom) = item {
                                    vm.form.update(|f| f.nomenclature_ref = Some(nom.id.clone()));
                                    vm.nomenclature_name.set(nom.description);
                                    vm.nomenclature_code.set(nom.code);
                                    vm.nomenclature_article.set(nom.article);
                                    vm.success_message.set(Some("Номенклатура выбрана".to_string()));
                                }
                                vm.show_picker.set(false);
                                vm.search_results.set(None);
                            }
                        };

                        let on_cancel_handler = {
                            let vm = vm_for_cancel.clone();
                            move |_| {
                                vm.show_picker.set(false);
                                vm.search_results.set(None);
                            }
                        };

                        // Если есть результаты поиска, передаем их в picker
                        let prefiltered = vm.search_results.get();

                        if let Some(filtered_list) = prefiltered {
                            view! {
                                <div class="modal-overlay">
                                    <NomenclaturePicker
                                        initial_selected_id=None
                                        prefiltered_items=filtered_list
                                        on_selected=on_selected_handler
                                        on_cancel=on_cancel_handler
                                    />
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="modal-overlay">
                                    <NomenclaturePicker
                                        initial_selected_id=None
                                        on_selected=on_selected_handler
                                        on_cancel=on_cancel_handler
                                    />
                                </div>
                            }.into_any()
                        }
                    } else {
                        view! {}.into_any()
                    }
                }
            }
        </div>
        </div>
    }
}
