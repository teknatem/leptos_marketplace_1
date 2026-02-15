//! General tab for Connection MP details

use crate::domain::a002_organization::ui::{OrganizationPicker, OrganizationPickerItem};
use crate::domain::a005_marketplace::ui::details::MarketplaceDetails;
use crate::domain::a005_marketplace::ui::{MarketplacePicker, MarketplacePickerItem};
use crate::domain::a006_connection_mp::ui::details::view_model::ConnectionMPDetailsVm;
use crate::shared::icons::icon;
use crate::shared::modal_stack::ModalStackService;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn GeneralTab(vm: ConnectionMPDetailsVm) -> impl IntoView {
    let modal_stack =
        use_context::<ModalStackService>().expect("ModalStackService not found in context");

    // Создаем клоны vm для каждого использования
    let vm_mp_picker = vm.clone();
    let vm_mp_view = vm.clone();
    let vm_org_picker = vm.clone();
    let vm_test_result = vm.clone();
    let vm_marketplace_name = vm.clone();
    let vm_marketplace_disabled = vm.clone();
    let vm_organization_name = vm.clone();

    // Клоны для всех полей формы
    let vm_description = vm.clone();
    let vm_planned_commission = vm.clone();
    let vm_comment = vm.clone();
    let vm_api_key = vm.clone();
    let vm_supplier_id = vm.clone();
    let vm_application_id = vm.clone();
    let vm_business_account_id = vm.clone();
    let vm_api_key_stats = vm.clone();
    let vm_is_used = vm.clone();
    let vm_test_mode = vm.clone();

    let modal_stack_mp = modal_stack.clone();
    let open_marketplace_picker = move |_| {
        let modal_stack_mp = modal_stack_mp.clone();
        let vm_mp_picker = vm_mp_picker.clone();
        let form = vm_mp_picker.form.get();
        let selected_id = if form.marketplace_id.is_empty() {
            None
        } else {
            Some(form.marketplace_id)
        };

        modal_stack_mp.push_with_frame(
            Some("max-width: min(1100px, 95vw); width: min(1100px, 95vw);".to_string()),
            Some("marketplace-picker-modal".to_string()),
            move |handle| {
                let vm = vm_mp_picker.clone();
                view! {
                    <MarketplacePicker
                        initial_selected_id=selected_id.clone()
                        on_selected={
                            let handle = handle.clone();
                            move |selected: Option<MarketplacePickerItem>| {
                                if let Some(item) = selected {
                                    vm.update_marketplace_info(item.id.clone(), item.description.clone(), item.code.clone());
                                }
                                handle.close();
                            }
                        }
                        on_cancel={
                            let handle = handle.clone();
                            move |_| handle.close()
                        }
                    />
                }
                .into_any()
            },
        );
    };

    let modal_stack_view = modal_stack.clone();
    let open_marketplace_view = move |_| {
        let modal_stack_view = modal_stack_view.clone();
        let mp_id = vm_mp_view.form.get().marketplace_id;
        if mp_id.is_empty() {
            return;
        }

        modal_stack_view.push_with_frame(
            Some("max-width: min(1100px, 95vw); width: min(1100px, 95vw);".to_string()),
            Some("marketplace-view-modal".to_string()),
            move |handle| {
                view! {
                    <MarketplaceDetails
                        id=Some(mp_id.clone())
                        readonly=true
                        on_saved=Callback::new(move |_| {})
                        on_cancel=Callback::new({
                            let handle = handle.clone();
                            move |_| handle.close()
                        })
                    />
                }
                .into_any()
            },
        );
    };

    let modal_stack_org = modal_stack.clone();
    let open_organization_picker = move |_| {
        let modal_stack_org = modal_stack_org.clone();
        let vm_org_picker = vm_org_picker.clone();
        let form = vm_org_picker.form.get();
        let selected_id = if form.organization_ref.is_empty() {
            None
        } else {
            Some(form.organization_ref)
        };
        modal_stack_org.push_with_frame(
            Some("max-width: min(1100px, 95vw); width: min(1100px, 95vw);".to_string()),
            Some("organization-picker-modal".to_string()),
            move |handle| {
                let vm = vm_org_picker.clone();
                view! {
                    <OrganizationPicker
                        initial_selected_id=selected_id.clone()
                        on_confirm={
                            let handle = handle.clone();
                            move |selected: Option<OrganizationPickerItem>| {
                                if let Some(item) = selected {
                                    vm.update_organization_info(item.id.clone(), item.description.clone());
                                }
                                handle.close();
                            }
                        }
                        on_cancel={
                            let handle = handle.clone();
                            move |_| handle.close()
                        }
                    />
                }
                .into_any()
            },
        );
    };

    view! {
        <div class="details-container">
            // Секция 1: Основная информация
            <div class="details-section">
                <h4 class="details-section__title" style="margin-bottom: 16px;">
                    "Основная информация"
                </h4>
                <div class="details-grid--3col">
                    <div class="form__group">
                        <label class="form__label">{"Наименование"}</label>
                        <input
                            class="form__input"
                            type="text"
                            prop:value={
                                let vm = vm_description.clone();
                                move || vm.form.get().description
                            }
                            on:input={
                                let vm = vm_description.clone();
                                move |ev| {
                                    vm.form.update(|f| f.description = event_target_value(&ev));
                                }
                            }
                            placeholder="Например: Озон (Сантехсистем)"
                        />
                    </div>

                    <div class="form__group">
                        <label class="form__label">{"Маркетплейс"}</label>
                        <Input
                            value=vm_marketplace_name.marketplace_name
                            placeholder="Выберите"
                            readonly=true
                            attr:style="width: 100%;"
                        >
                            <InputSuffix slot>
                                <div style="display: flex; gap: 4px;">
                                    <Button
                                        appearance=ButtonAppearance::Subtle
                                        shape=ButtonShape::Square
                                        size=ButtonSize::Small
                                        on_click=open_marketplace_picker
                                        attr:style="width: 28px; height: 28px; min-width: 28px; padding: 0; display: flex; align-items: center; justify-content: center;"
                                        attr:title="Выбрать маркетплейс"
                                    >
                                        {icon("search")}
                                    </Button>
                                    <Button
                                        appearance=ButtonAppearance::Subtle
                                        shape=ButtonShape::Square
                                        size=ButtonSize::Small
                                        disabled=Signal::derive({
                                            let vm = vm_marketplace_disabled.clone();
                                            move || vm.form.get().marketplace_id.is_empty()
                                        })
                                        on_click=open_marketplace_view
                                        attr:style="width: 28px; height: 28px; min-width: 28px; padding: 0; display: flex; align-items: center; justify-content: center;"
                                        attr:title="Просмотр маркетплейса"
                                    >
                                        {icon("eye")}
                                    </Button>
                                </div>
                            </InputSuffix>
                        </Input>
                    </div>

                    <div class="form__group">
                        <label class="form__label">{"Организация"}</label>
                        <Input
                            value=vm_organization_name.organization_name
                            placeholder="Выберите"
                            readonly=true
                            attr:style="width: 100%;"
                        >
                            <InputSuffix slot>
                                <Button
                                    appearance=ButtonAppearance::Subtle
                                    shape=ButtonShape::Square
                                    size=ButtonSize::Small
                                    on_click=open_organization_picker
                                    attr:style="width: 28px; height: 28px; min-width: 28px; padding: 0; display: flex; align-items: center; justify-content: center;"
                                    attr:title="Выбрать организацию"
                                >
                                    {icon("search")}
                                </Button>
                            </InputSuffix>
                        </Input>
                    </div>

                    <div class="form__group">
                        <label class="form__label">{"Плановый процент комиссии, %"}</label>
                        <input
                            class="form__input"
                            type="number"
                            step="0.01"
                            min="0"
                            max="100"
                            prop:value={
                                let vm = vm_planned_commission.clone();
                                move || {
                                    vm.form.get().planned_commission_percent
                                        .map(|v| v.to_string())
                                        .unwrap_or_default()
                                }
                            }
                            on:input={
                                let vm = vm_planned_commission.clone();
                                move |ev| {
                                    let val = event_target_value(&ev);
                                    vm.form.update(|f| {
                                        f.planned_commission_percent = if val.is_empty() {
                                            None
                                        } else {
                                            val.parse::<f64>().ok()
                                        };
                                    });
                                }
                            }
                            placeholder="Например: 15.5"
                        />
                        <small class="help-text" style="font-size: 11px; color: var(--colorNeutralForeground3); margin-top: 4px; display: block;">
                            {"Плановый процент комиссии маркетплейса"}
                        </small>
                    </div>

                    <div class="form__group" style="grid-column: 1 / -1;">
                        <label class="form__label">{"Комментарий"}</label>
                        <textarea
                            class="form__input form__textarea"
                            rows="3"
                            prop:value={
                                let vm = vm_comment.clone();
                                move || vm.form.get().comment.unwrap_or_default()
                            }
                            on:input={
                                let vm = vm_comment.clone();
                                move |ev| {
                                    let val = event_target_value(&ev);
                                    vm.form.update(|f| {
                                        f.comment = if val.is_empty() { None } else { Some(val) };
                                    });
                                }
                            }
                            placeholder="Дополнительная информация"
                        />
                    </div>
                </div>
            </div>

            // Секция 2: API конфигурация
            <div class="details-section">
                <h4 class="details-section__title" style="margin-bottom: 16px;">
                    "API конфигурация"
                </h4>
                <div class="details-grid--api">
                    <div class="form__group" style="grid-row: span 2;">
                        <label class="form__label">{"API Key"}</label>
                        <textarea
                            class="form__input form__textarea"
                            style="min-height: 100px;"
                            rows="6"
                            prop:value={
                                let vm = vm_api_key.clone();
                                move || vm.form.get().api_key
                            }
                            on:input={
                                let vm = vm_api_key.clone();
                                move |ev| {
                                    vm.form.update(|f| f.api_key = event_target_value(&ev));
                                }
                            }
                            placeholder="Вставьте API ключ"
                        />
                        <small class="help-text help-text--tiny">
                            {"• WB: Bearer • Ozon: Api-Key • Яндекс: OAuth"}
                        </small>
                    </div>

                    <div class="form__group">
                        <label class="form__label">{"Client ID"}</label>
                        <input
                            class="form__input"
                            type="text"
                            prop:value={
                                let vm = vm_supplier_id.clone();
                                move || vm.form.get().supplier_id.unwrap_or_default()
                            }
                            on:input={
                                let vm = vm_supplier_id.clone();
                                move |ev| {
                                    let val = event_target_value(&ev);
                                    vm.form.update(|f| {
                                        f.supplier_id = if val.is_empty() { None } else { Some(val) };
                                    });
                                }
                            }
                            placeholder="Ozon, Яндекс"
                        />
                        <small class="help-text help-text--tiny">{"Ozon, Яндекс"}</small>
                    </div>

                    <div class="form__group">
                        <label class="form__label">{"App ID"}</label>
                        <input
                            class="form__input"
                            type="text"
                            prop:value={
                                let vm = vm_application_id.clone();
                                move || vm.form.get().application_id.unwrap_or_default()
                            }
                            on:input={
                                let vm = vm_application_id.clone();
                                move |ev| {
                                    let val = event_target_value(&ev);
                                    vm.form.update(|f| {
                                        f.application_id = if val.is_empty() { None } else { Some(val) };
                                    });
                                }
                            }
                            placeholder="Ozon"
                        />
                        <small class="help-text help-text--tiny">{"Ozon"}</small>
                    </div>

                    <div class="form__group">
                        <label class="form__label">{"Business ID"}</label>
                        <input
                            class="form__input"
                            type="text"
                            prop:value={
                                let vm = vm_business_account_id.clone();
                                move || vm.form.get().business_account_id.unwrap_or_default()
                            }
                            on:input={
                                let vm = vm_business_account_id.clone();
                                move |ev| {
                                    let val = event_target_value(&ev);
                                    vm.form.update(|f| {
                                        f.business_account_id = if val.is_empty() { None } else { Some(val) };
                                    });
                                }
                            }
                            placeholder="Яндекс"
                        />
                        <small class="help-text help-text--tiny">{"Яндекс"}</small>
                    </div>

                    <div class="form__group">
                        <label class="form__label">{"Stats Key"}</label>
                        <input
                            class="form__input"
                            type="text"
                            prop:value={
                                let vm = vm_api_key_stats.clone();
                                move || vm.form.get().api_key_stats.unwrap_or_default()
                            }
                            on:input={
                                let vm = vm_api_key_stats.clone();
                                move |ev| {
                                    let val = event_target_value(&ev);
                                    vm.form.update(|f| {
                                        f.api_key_stats = if val.is_empty() { None } else { Some(val) };
                                    });
                                }
                            }
                            placeholder="Опционально"
                        />
                    </div>
                </div>
            </div>

            // Секция 3: Настройки
            <div class="details-flags">
                <label style="display: flex; align-items: center; gap: 8px; cursor: pointer;">
                    <input
                        type="checkbox"
                        class="table__checkbox"
                        prop:checked={
                            let vm = vm_is_used.clone();
                            move || vm.form.get().is_used
                        }
                        on:change={
                            let vm = vm_is_used.clone();
                            move |ev| {
                                vm.form.update(|f| f.is_used = event_target_checked(&ev));
                            }
                        }
                    />
                    "Используется"
                </label>
                <label style="display: flex; align-items: center; gap: 8px; cursor: pointer;">
                    <input
                        type="checkbox"
                        class="table__checkbox"
                        prop:checked={
                            let vm = vm_test_mode.clone();
                            move || vm.form.get().test_mode
                        }
                        on:change={
                            let vm = vm_test_mode.clone();
                            move |ev| {
                                vm.form.update(|f| f.test_mode = event_target_checked(&ev));
                            }
                        }
                    />
                    "Тестовый режим"
                </label>
            </div>

            // Результат теста (упрощенная версия)
            {move || {
                vm_test_result.test_result.get().map(|result| {
                    let class = if result.success { "success" } else { "error" };
                    view! {
                        <div class=format!("test-result {}", class) style="margin-top: 16px; padding: 12px; border-radius: 8px; background: var(--color-bg-subtle);">
                            <h4 class="test-result__title" style="margin: 0 0 8px 0;">
                                {if result.success { "✅ Тест успешен" } else { "❌ Тест не пройден" }}
                            </h4>
                            <div style="margin-bottom: 6px;">
                                <strong>{"Статус: "}</strong>
                                {result.message.clone()}
                                {" "}
                                <span style="color: #666; font-size: 11px;">{"("}{result.duration_ms}{"ms)"}</span>
                            </div>
                            {if let Some(details) = result.details.as_ref() {
                                view! {
                                    <div style="margin-top: 8px; padding: 10px; background: rgba(255,193,7,0.1); border-left: 3px solid #ffc107; border-radius: 4px; font-size: 12px;">
                                        {details.clone()}
                                    </div>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }}
                        </div>
                    }
                })
            }}
        </div>
    }
}
