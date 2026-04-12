use super::view_model::MarketplaceProductDetailsViewModel;
use crate::domain::a004_nomenclature::ui::picker::NomenclaturePicker;
use crate::shared::date_utils::format_datetime;
use crate::shared::icons::icon;
use crate::shared::modal_stack::ModalStackService;
use crate::shared::page_frame::PageFrame;
use leptos::prelude::*;
use std::rc::Rc;
use thaw::*;

#[component]
pub fn MarketplaceProductDetails(
    id: Option<String>,
    #[prop(into, optional)] on_saved: Option<Callback<()>>,
    #[prop(into, optional)] on_close: Option<Callback<()>>,
) -> impl IntoView {
    let modal_stack =
        use_context::<ModalStackService>().expect("ModalStackService not found in context");
    let vm = MarketplaceProductDetailsViewModel::new();
    vm.load_if_needed(id);

    let on_saved_clone = on_saved.clone();
    let on_close_clone = on_close.clone();

    let vm_title = vm.clone();
    let vm_save = vm.clone();
    let vm_is_valid = vm.clone();
    let vm_button_label = vm.clone();
    let vm_error = vm.clone();
    let vm_success = vm.clone();
    let vm_search_nom = vm.clone();
    let vm_search_nom_disabled = vm.clone();
    let vm_open_picker = vm.clone();
    let vm_clear_nom = vm.clone();
    let vm_clear_nom_disabled = vm.clone();
    let vm_picker = vm.clone();
    let vm_product_link = vm.clone();
    let vm_operational_info = vm.clone();

    let description = RwSignal::new(vm.form.get_untracked().description.clone());
    let marketplace_sku = RwSignal::new(vm.form.get_untracked().marketplace_sku.clone());
    let article = RwSignal::new(vm.form.get_untracked().article.clone());
    let barcode = RwSignal::new(vm.form.get_untracked().barcode.clone().unwrap_or_default());
    let brand = RwSignal::new(vm.form.get_untracked().brand.clone().unwrap_or_default());
    let comment = RwSignal::new(vm.form.get_untracked().comment.clone().unwrap_or_default());

    Effect::new({
        let vm = vm.clone();
        move || {
            let v = description.get();
            untrack(move || vm.form.update(|f| f.description = v.clone()));
        }
    });
    Effect::new({
        let vm = vm.clone();
        move || {
            let v = marketplace_sku.get();
            untrack(move || vm.form.update(|f| f.marketplace_sku = v.clone()));
        }
    });
    Effect::new({
        let vm = vm.clone();
        move || {
            let v = article.get();
            untrack(move || vm.form.update(|f| f.article = v.clone()));
        }
    });
    Effect::new({
        let vm = vm.clone();
        move || {
            let v = barcode.get();
            untrack(move || {
                vm.form.update(|f| {
                    f.barcode = if v.trim().is_empty() {
                        None
                    } else {
                        Some(v.clone())
                    }
                })
            });
        }
    });
    Effect::new({
        let vm = vm.clone();
        move || {
            let v = brand.get();
            untrack(move || {
                vm.form.update(|f| {
                    f.brand = if v.trim().is_empty() {
                        None
                    } else {
                        Some(v.clone())
                    }
                })
            });
        }
    });
    Effect::new({
        let vm = vm.clone();
        move || {
            let v = comment.get();
            untrack(move || {
                vm.form.update(|f| {
                    f.comment = if v.trim().is_empty() {
                        None
                    } else {
                        Some(v.clone())
                    }
                })
            });
        }
    });

    Effect::new({
        let vm = vm.clone();
        move || {
            let f = vm.form.get();
            if description.get_untracked() != f.description {
                description.set(f.description);
            }
            if marketplace_sku.get_untracked() != f.marketplace_sku {
                marketplace_sku.set(f.marketplace_sku);
            }
            if article.get_untracked() != f.article {
                article.set(f.article);
            }
            let bc = f.barcode.unwrap_or_default();
            if barcode.get_untracked() != bc {
                barcode.set(bc);
            }
            let br = f.brand.unwrap_or_default();
            if brand.get_untracked() != br {
                brand.set(br);
            }
            let c = f.comment.unwrap_or_default();
            if comment.get_untracked() != c {
                comment.set(c);
            }
        }
    });

    view! {
        <PageFrame page_id="a007_marketplace_product--detail" category="detail">
            <div class="page__header">
                <div class="page__header-left">
                    <h2>
                        {
                            let vm = vm_title.clone();
                            move || {
                                if vm.is_edit_mode()() {
                                    format!("Редактирование: {}", vm.form.get().description)
                                } else {
                                    "Новый товар маркетплейса".to_string()
                                }
                            }
                        }
                    </h2>
                </div>
                <div class="page__header-right">
                    <Flex gap=FlexGap::Small>
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click={
                                let vm = vm_save.clone();
                                let on_saved = on_saved_clone.clone();
                                move |_| {
                                    let on_saved = on_saved.clone();
                                    vm.save_command(Rc::new(move |_| {
                                        if let Some(ref cb) = on_saved {
                                            cb.run(());
                                        }
                                    }))
                                }
                            }
                            disabled=Signal::derive({
                                let vm = vm_is_valid.clone();
                                move || !vm.is_form_valid()()
                            })
                        >
                            {icon("save")}
                            {
                                let vm = vm_button_label.clone();
                                move || if vm.is_edit_mode()() { " Сохранить" } else { " Создать" }
                            }
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            on_click=move |_| {
                                if let Some(ref cb) = on_close_clone {
                                    cb.run(());
                                }
                            }
                        >
                            {icon("x")}
                            " Закрыть"
                        </Button>
                    </Flex>
                </div>
            </div>

            <div class="page__content">
                {
                    let vm = vm_error.clone();
                    move || vm.error.get().map(|e| view! {
                        <div class="warning-box warning-box--error" style="margin-bottom: var(--spacing-md);">
                            <span class="warning-box__icon">"⚠"</span>
                            <span class="warning-box__text">{e}</span>
                        </div>
                    })
                }

                {
                    let vm = vm_success.clone();
                    move || vm.success_message.get().map(|msg| view! {
                        <div class="info-box" style="margin-bottom: var(--spacing-md);">
                            {msg}
                        </div>
                    })
                }

                <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(400px, 1fr)); gap: var(--spacing-xl);">
                    <Card>
                        <Flex vertical=true gap=FlexGap::Medium>
                            <h3 style="margin: 0; font-size: var(--font-size-base); font-weight: 600;">"Основная информация"</h3>

                            <Flex vertical=true gap=FlexGap::Small>
                                <Label>"Описание"</Label>
                                <Input value=description placeholder="Краткое описание товара" />
                            </Flex>

                            <Flex gap=FlexGap::Medium>
                                <Flex vertical=true gap=FlexGap::Small style="flex: 1;">
                                    <Label>"Маркетплейс"</Label>
                                    <Input value=vm.marketplace_name.clone() disabled=Signal::derive(|| true) />
                                </Flex>
                                <Flex vertical=true gap=FlexGap::Small style="flex: 1;">
                                    <Label>"Кабинет"</Label>
                                    <Input value=vm.connection_name.clone() disabled=Signal::derive(|| true) />
                                </Flex>
                            </Flex>

                            <Flex gap=FlexGap::Medium>
                                <Flex vertical=true gap=FlexGap::Small style="flex: 1;">
                                    <Label>"SKU маркетплейса"</Label>
                                    <Input value=marketplace_sku />
                                </Flex>
                                <Flex vertical=true gap=FlexGap::Small style="flex: 1;">
                                    <Label>"Артикул"</Label>
                                    <Input value=article />
                                </Flex>
                            </Flex>

                            <div class="a007-link-card__summary">
                                <div class="a007-link-card__summary-label">"Быстрый доступ"</div>
                                <div class="a007-link-card__summary-value">
                                    {move || {
                                        let sku = vm_product_link.form.get().marketplace_sku.trim().to_string();
                                        match vm_product_link.marketplace_product_url() {
                                            Some(url) => view! {
                                                <a
                                                    href=url
                                                    target="_blank"
                                                    rel="noopener noreferrer"
                                                    style="display: inline-flex; align-items: center; gap: 8px;"
                                                >
                                                    {icon("link")}
                                                    "Открыть карточку товара на WB"
                                                </a>
                                            }
                                            .into_any(),
                                            None if sku.is_empty() => view! {
                                                <span>"Заполните SKU, чтобы собрать ссылку на товар."</span>
                                            }
                                            .into_any(),
                                            None => view! {
                                                <span>"Быстрая внешняя ссылка сейчас поддерживается только для Wildberries."</span>
                                            }
                                            .into_any(),
                                        }
                                    }}
                                </div>
                            </div>

                            <div class="a007-link-card__summary">
                                <div class="a007-link-card__summary-label">"Оперативная сводка"</div>
                                <div
                                    class="a007-link-card__summary-value"
                                    style="display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 12px;"
                                >
                                    {move || {
                                        let form = vm_operational_info.form.get();
                                        let last_update = form
                                            .last_update
                                            .map(|d| format_datetime(&d.to_rfc3339()))
                                            .unwrap_or_else(|| "—".to_string());
                                        let category = form
                                            .category_name
                                            .clone()
                                            .filter(|v| !v.trim().is_empty())
                                            .unwrap_or_else(|| "—".to_string());
                                        let brand = form
                                            .brand
                                            .clone()
                                            .filter(|v| !v.trim().is_empty())
                                            .unwrap_or_else(|| "—".to_string());
                                        let barcode = form
                                            .barcode
                                            .clone()
                                            .filter(|v| !v.trim().is_empty())
                                            .unwrap_or_else(|| "—".to_string());

                                        view! {
                                            <>
                                                <div>
                                                    <strong>"Обновлено:"</strong>
                                                    <div>{last_update}</div>
                                                </div>
                                                <div>
                                                    <strong>"Категория:"</strong>
                                                    <div>{category}</div>
                                                </div>
                                                <div>
                                                    <strong>"Бренд:"</strong>
                                                    <div>{brand}</div>
                                                </div>
                                                <div>
                                                    <strong>"Штрихкод:"</strong>
                                                    <div>{barcode}</div>
                                                </div>
                                            </>
                                        }
                                    }}
                                </div>
                            </div>

                            <Flex gap=FlexGap::Medium>
                                <Flex vertical=true gap=FlexGap::Small style="flex: 1;">
                                    <Label>"Штрихкод"</Label>
                                    <Input value=barcode />
                                </Flex>
                                <Flex vertical=true gap=FlexGap::Small style="flex: 1;">
                                    <Label>"Бренд"</Label>
                                    <Input value=brand />
                                </Flex>
                            </Flex>
                        </Flex>
                    </Card>

                    <Card>
                        <div class="a007-link-card">
                            <div class="a007-link-card__header">
                                <h3 class="a007-link-card__title">"Связь с 1С УТ"</h3>
                                <span class=move || {
                                    if vm.form.get().nomenclature_ref.is_some() {
                                        "a007-link-card__status a007-link-card__status--linked"
                                    } else {
                                        "a007-link-card__status a007-link-card__status--empty"
                                    }
                                }>
                                    {move || {
                                        if vm.form.get().nomenclature_ref.is_some() {
                                            "Связана"
                                        } else {
                                            "Не связана"
                                        }
                                    }}
                                </span>
                            </div>

                            <p class="a007-link-card__hint">
                                "Автоподбор ищет точное совпадение по артикулу товара маркетплейса. Если найдено несколько вариантов, откроется выбор."
                            </p>

                            <div class="a007-link-card__summary">
                                <div class="a007-link-card__summary-label">"Основание для автоподбора"</div>
                                <div class="a007-link-card__summary-value">
                                    {move || {
                                        let form = vm.form.get();
                                        if form.article.trim().is_empty() {
                                            "Артикул товара маркетплейса не заполнен".to_string()
                                        } else {
                                            format!("Артикул: {} | SKU: {}", form.article, form.marketplace_sku)
                                        }
                                    }}
                                </div>
                            </div>

                            <Flex vertical=true gap=FlexGap::Small>
                                <Label>"Связанная номенклатура 1С"</Label>
                                <Input
                                    value=vm.nomenclature_name.clone()
                                    disabled=Signal::derive(|| true)
                                    placeholder="Связь еще не выбрана"
                                />
                            </Flex>

                            <div class="a007-link-card__actions">
                                <Button
                                    appearance=ButtonAppearance::Primary
                                    on_click={
                                        let vm = vm_search_nom.clone();
                                        move |_| vm.search_nomenclature_by_article()
                                    }
                                    disabled=Signal::derive({
                                        let vm = vm_search_nom_disabled.clone();
                                        move || vm.form.get().article.trim().is_empty()
                                    })
                                >
                                    {icon("search")}
                                    " Автоподбор по артикулу"
                                </Button>
                                <Button
                                    appearance=ButtonAppearance::Secondary
                                    on_click={
                                        let vm = vm_open_picker.clone();
                                        move |_| vm.open_picker()
                                    }
                                >
                                    {icon("list")}
                                    " Выбрать вручную"
                                </Button>
                                <Button
                                    appearance=ButtonAppearance::Subtle
                                    on_click={
                                        let vm = vm_clear_nom.clone();
                                        move |_| vm.clear_nomenclature()
                                    }
                                    disabled=Signal::derive({
                                        let vm = vm_clear_nom_disabled.clone();
                                        move || vm.form.get().nomenclature_ref.is_none()
                                    })
                                >
                                    {icon("x")}
                                    " Очистить связь"
                                </Button>
                            </div>

                            <div class="a007-link-card__actions-note">
                                "Ручной выбор открывает список номенклатуры 1С. Очистка снимает текущую связь и не меняет карточку товара маркетплейса."
                            </div>

                            <Flex gap=FlexGap::Medium>
                                <Flex vertical=true gap=FlexGap::Small style="flex: 1;">
                                    <Label>"Код 1С"</Label>
                                    <Input value=vm.nomenclature_code.clone() disabled=Signal::derive(|| true) />
                                </Flex>
                                <Flex vertical=true gap=FlexGap::Small style="flex: 1;">
                                    <Label>"Артикул 1С"</Label>
                                    <Input value=vm.nomenclature_article.clone() disabled=Signal::derive(|| true) />
                                </Flex>
                            </Flex>
                        </div>
                    </Card>
                </div>

                <div style="margin-top: var(--spacing-xl);">
                    <Card>
                        <Flex vertical=true gap=FlexGap::Small>
                            <Label>"Комментарий"</Label>
                            <Input
                                value=comment
                                placeholder="Дополнительная информация (необязательно)"
                            />
                        </Flex>
                    </Card>
                </div>
            </div>

            {
                let vm = vm_picker.clone();
                move || {
                    if vm.show_picker.get() {
                        let vm_for_selected = vm.clone();
                        let vm_for_cancel = vm.clone();
                        let prefiltered = vm.search_results.get();
                        let article_for_picker = vm.form.get().article;

                        modal_stack.push_with_frame(
                            Some(
                                "max-width: min(1100px, 95vw); width: min(1100px, 95vw); height: 85vh; overflow: hidden;"
                                    .to_string(),
                            ),
                            Some("nomenclature-picker-modal".to_string()),
                            move |handle| {
                                let on_selected_handler = {
                                    let vm = vm_for_selected.clone();
                                    let handle = handle.clone();
                                    move |item: Option<crate::domain::a004_nomenclature::ui::picker::NomenclaturePickerItem>| {
                                        if let Some(nom) = item {
                                            vm.form.update(|f| f.nomenclature_ref = Some(nom.id.clone()));
                                            vm.nomenclature_name.set(nom.description);
                                            vm.nomenclature_code.set(nom.code);
                                            vm.nomenclature_article.set(nom.article);
                                            vm.success_message.set(Some("Связь с 1С УТ обновлена вручную".to_string()));
                                        }
                                        vm.search_results.set(None);
                                        handle.close();
                                    }
                                };

                                let on_cancel_handler = {
                                    let vm = vm_for_cancel.clone();
                                    let handle = handle.clone();
                                    move |_| {
                                        vm.search_results.set(None);
                                        handle.close();
                                    }
                                };

                                if let Some(filtered_list) = prefiltered.clone() {
                                    view! {
                                        <NomenclaturePicker
                                            initial_selected_id=None
                                            prefiltered_items=filtered_list.clone()
                                            title="Выберите позицию 1С из найденных вариантов"
                                            subtitle=format!(
                                                "По артикулу \"{}\" найдено {} вариантов. Выберите нужную позицию 1С УТ.",
                                                article_for_picker,
                                                filtered_list.len()
                                            )
                                            search_placeholder="Уточните список по артикулу, коду или наименованию"
                                            empty_state_text="Список вариантов пуст"
                                            on_selected=on_selected_handler
                                            on_cancel=on_cancel_handler
                                        />
                                    }.into_any()
                                } else {
                                    view! {
                                        <NomenclaturePicker
                                            initial_selected_id=None
                                            title="Ручной выбор позиции 1С УТ"
                                            subtitle="Откройте полный список номенклатуры 1С и выберите позицию, которую нужно связать с текущим товаром маркетплейса."
                                            search_placeholder="Поиск по артикулу, коду или наименованию"
                                            empty_state_text="Подходящие позиции не найдены"
                                            on_selected=on_selected_handler
                                            on_cancel=on_cancel_handler
                                        />
                                    }.into_any()
                                }
                            },
                        );

                        vm.show_picker.set(false);
                        view! { <></> }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }
                }
            }
        </PageFrame>
    }
}
