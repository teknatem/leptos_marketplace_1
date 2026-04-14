//! General tab — supply metadata

use super::super::view_model::WbSupplyDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::date_utils::format_datetime;
use leptos::prelude::*;
use thaw::*;

fn cargo_type_label(cargo_type: Option<i32>) -> &'static str {
    match cargo_type {
        Some(0) => "Виртуальная",
        Some(1) => "Короб",
        Some(2) => "Монопаллета",
        Some(5) => "Суперсейф",
        _ => "—",
    }
}

#[component]
pub fn GeneralTab(vm: WbSupplyDetailsVm) -> impl IntoView {
    view! {
        {move || {
            let Some(supply) = vm.supply.get() else {
                return view! { <div>"Нет данных"</div> }.into_any();
            };

            let supply_id = supply.header.supply_id.clone();
            let supply_name = supply.info.name.clone().unwrap_or_else(|| "—".to_string());
            let is_done = supply.info.is_done;
            let is_b2b = supply.info.is_b2b;
            let cargo = cargo_type_label(supply.info.cargo_type);
            let created = supply
                .info
                .created_at_wb
                .as_deref()
                .map(format_datetime)
                .unwrap_or_else(|| "—".to_string());
            let closed = supply
                .info
                .closed_at_wb
                .as_deref()
                .map(format_datetime)
                .unwrap_or_else(|| "—".to_string());
            let scan = supply
                .info
                .scan_dt
                .as_deref()
                .map(format_datetime)
                .unwrap_or_else(|| "—".to_string());
            let conn_name = vm
                .connection_info
                .get()
                .map(|c| c.description)
                .unwrap_or_else(|| supply.header.connection_id.clone());
            let org_name = vm
                .organization_info
                .get()
                .map(|o| o.description)
                .unwrap_or_else(|| supply.header.organization_id.clone());
            let version = supply.metadata.version.to_string();
            let created_at = format_datetime(&supply.metadata.created_at);
            let updated_at = format_datetime(&supply.metadata.updated_at);
            let orders_count = supply.supply_orders.len().to_string();

            view! {
                <div class="detail-grid">
                    <div class="detail-grid__col">
                        <CardAnimated nav_id="a029_wb_supply_details_main">
                            <h3 class="details-section__title">"Поставка"</h3>
                            <div class="form__group">
                                <label class="form__label">"ID поставки WB"</label>
                                <div class="form__value"><code>{supply_id}</code></div>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Название"</label>
                                <div class="form__value">{supply_name}</div>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Статус"</label>
                                <div class="form__value">
                                    {if is_done {
                                        view! {
                                            <Badge
                                                appearance=BadgeAppearance::Filled
                                                color=BadgeColor::Success
                                            >
                                                "Завершена"
                                            </Badge>
                                        }
                                        .into_any()
                                    } else {
                                        view! {
                                            <Badge
                                                appearance=BadgeAppearance::Filled
                                                color=BadgeColor::Warning
                                            >
                                                "Открыта"
                                            </Badge>
                                        }
                                        .into_any()
                                    }}
                                </div>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"B2B"</label>
                                <div class="form__value">
                                    {if is_b2b { "Да" } else { "Нет" }}
                                </div>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Тип упаковки"</label>
                                <div class="form__value">{cargo}</div>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Кол-во заказов"</label>
                                <div class="form__value">{orders_count}</div>
                            </div>
                        </CardAnimated>

                        <CardAnimated nav_id="a029_wb_supply_details_dates">
                            <h3 class="details-section__title">"Даты"</h3>
                            <div class="form__group">
                                <label class="form__label">"Создана"</label>
                                <div class="form__value">{created}</div>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Закрыта"</label>
                                <div class="form__value">{closed}</div>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Сканирована"</label>
                                <div class="form__value">{scan}</div>
                            </div>
                        </CardAnimated>
                    </div>

                    <div class="detail-grid__col">
                        <CardAnimated nav_id="a029_wb_supply_details_refs">
                            <h3 class="details-section__title">"Связи"</h3>
                            <div class="form__group">
                                <label class="form__label">"Подключение"</label>
                                <div class="form__value">{conn_name}</div>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Организация"</label>
                                <div class="form__value">{org_name}</div>
                            </div>
                        </CardAnimated>

                        <CardAnimated nav_id="a029_wb_supply_details_meta">
                            <h3 class="details-section__title">"Служебные данные"</h3>
                            <div class="form__group">
                                <label class="form__label">"Создан в системе"</label>
                                <div class="form__value">{created_at}</div>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Обновлён"</label>
                                <div class="form__value">{updated_at}</div>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Версия"</label>
                                <div class="form__value">{version}</div>
                            </div>
                        </CardAnimated>
                    </div>
                </div>
            }
            .into_any()
        }}
    }
}
