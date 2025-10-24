use super::view_model::MarketplaceProductDetailsViewModel;
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
        <div class="details-container marketplace-product-details">
            <div class="details-header">
                <h3>
                    {
                        let vm = vm_clone.clone();
                        move || if vm.is_edit_mode()() { "Редактирование товара" } else { "Новый товар маркетплейса" }
                    }
                </h3>
            </div>

            {
                let vm = vm_clone.clone();
                move || vm.error.get().map(|e| view! { <div class="error">{e}</div> })
            }

            <div class="details-form">
                <div class="form-group">
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

                <div class="form-group">
                    <label for="product_name">{"Наименование товара"}</label>
                    <input
                        type="text"
                        id="product_name"
                        prop:value={
                            let vm = vm_clone.clone();
                            move || vm.form.get().product_name
                        }
                        on:input={
                            let vm = vm_clone.clone();
                            move |ev| {
                                vm.form.update(|f| f.product_name = event_target_value(&ev));
                            }
                        }
                        placeholder="Наименование на маркетплейсе"
                    />
                </div>

                <div class="form-row">
                    <div class="form-group">
                        <label for="marketplace_id">{"ID Маркетплейса"}</label>
                        <input
                            type="text"
                            id="marketplace_id"
                            prop:value={
                                let vm = vm_clone.clone();
                                move || vm.form.get().marketplace_id
                            }
                            on:input={
                                let vm = vm_clone.clone();
                                move |ev| {
                                    vm.form.update(|f| f.marketplace_id = event_target_value(&ev));
                                }
                            }
                            placeholder="UUID маркетплейса"
                        />
                    </div>

                    <div class="form-group">
                        <label for="connection_mp_id">{"Кабинет"}</label>
                        <input
                            type="text"
                            id="connection_mp_id"
                            disabled
                            prop:value={
                                let vm = vm_clone.clone();
                                move || vm.form.get().connection_mp_id
                            }
                            placeholder="ID кабинета (авто)"
                        />
                    </div>
                </div>

                <div class="form-row">
                    <div class="form-group">
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
                    <div class="form-group">
                        <label for="art">{"Артикул"}</label>
                        <input
                            type="text"
                            id="art"
                            prop:value={
                                let vm = vm_clone.clone();
                                move || vm.form.get().art
                            }
                            on:input={
                                let vm = vm_clone.clone();
                                move |ev| {
                                    vm.form.update(|f| f.art = event_target_value(&ev));
                                }
                            }
                            placeholder="Артикул товара"
                        />
                    </div>

                    <div class="form-group">
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
                    <div class="form-group">
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

                    <div class="form-group">
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

                <div class="form-row">
                    <div class="form-group">
                        <label for="price">{"Цена"}</label>
                        <input
                            type="number"
                            step="0.01"
                            id="price"
                            prop:value={
                                let vm = vm_clone.clone();
                                move || vm.form.get().price.map(|p| p.to_string()).unwrap_or_default()
                            }
                            on:input={
                                let vm = vm_clone.clone();
                                move |ev| {
                                    let value = event_target_value(&ev);
                                    vm.form.update(|f| {
                                        f.price = value.parse::<f64>().ok();
                                    });
                                }
                            }
                            placeholder="0.00"
                        />
                    </div>

                    <div class="form-group">
                        <label for="stock">{"Остаток"}</label>
                        <input
                            type="number"
                            id="stock"
                            prop:value={
                                let vm = vm_clone.clone();
                                move || vm.form.get().stock.map(|s| s.to_string()).unwrap_or_default()
                            }
                            on:input={
                                let vm = vm_clone.clone();
                                move |ev| {
                                    let value = event_target_value(&ev);
                                    vm.form.update(|f| {
                                        f.stock = value.parse::<i32>().ok();
                                    });
                                }
                            }
                            placeholder="0"
                        />
                    </div>
                </div>

                <div class="form-group">
                    <label for="marketplace_url">{"URL товара на маркетплейсе"}</label>
                    <input
                        type="text"
                        id="marketplace_url"
                        prop:value={
                            let vm = vm_clone.clone();
                            move || vm.form.get().marketplace_url.clone().unwrap_or_default()
                        }
                        on:input={
                            let vm = vm_clone.clone();
                            move |ev| {
                                let value = event_target_value(&ev);
                                vm.form.update(|f| {
                                    f.marketplace_url = if value.is_empty() { None } else { Some(value) };
                                });
                            }
                        }
                        placeholder="https://marketplace.com/product/123"
                    />
                </div>

                <div class="form-group">
                    <label for="nomenclature_id">{"ID номенклатуры (1С)"}</label>
                    <input
                        type="text"
                        id="nomenclature_id"
                        prop:value={
                            let vm = vm_clone.clone();
                            move || vm.form.get().nomenclature_id.clone().unwrap_or_default()
                        }
                        on:input={
                            let vm = vm_clone.clone();
                            move |ev| {
                                let value = event_target_value(&ev);
                                vm.form.update(|f| {
                                    f.nomenclature_id = if value.is_empty() { None } else { Some(value) };
                                });
                            }
                        }
                        placeholder="UUID номенклатуры"
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
                        placeholder="Дополнительная информация (необязательно)"
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
