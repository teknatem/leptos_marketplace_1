use super::super::view_model::NomenclatureDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::card_animated::CardAnimated;
use leptos::prelude::*;
use thaw::*;

fn format_period(value: &str) -> String {
    if let Some((date, _)) = value.split_once('T') {
        let parts: Vec<&str> = date.split('-').collect();
        if parts.len() == 3 {
            return format!("{}.{}.{}", parts[2], parts[1], parts[0]);
        }
        return date.to_string();
    }

    let parts: Vec<&str> = value.split('-').collect();
    if parts.len() == 3 {
        format!("{}.{}.{}", parts[2], parts[1], parts[0])
    } else {
        value.to_string()
    }
}

fn format_price(price: f64) -> String {
    let formatted = format!("{price:.2}");
    let parts: Vec<&str> = formatted.split('.').collect();
    if parts.len() != 2 {
        return formatted;
    }

    let integer_part = parts[0];
    let decimal_part = parts[1];
    let chars: Vec<char> = integer_part.chars().collect();
    let mut result = String::new();

    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(' ');
        }
        result.push(*ch);
    }

    format!("{}.{}", result, decimal_part)
}

fn short_id(value: &str) -> String {
    value.chars().take(8).collect()
}

fn registrator_tab_key(registrator_type: &str, id: &str) -> Option<String> {
    match registrator_type {
        "a021_production_output" => Some(format!("a021_production_output_details_{id}")),
        "a023_purchase_of_goods" => Some(format!("a023_purchase_of_goods_details_{id}")),
        "a028_missing_cost_registry" => Some(format!("a028_missing_cost_registry_details_{id}")),
        _ => None,
    }
}

fn registrator_title(registrator_type: &str, id: &str) -> String {
    match registrator_type {
        "a021_production_output" => format!("Выпуск продукции {}", short_id(id)),
        "a023_purchase_of_goods" => format!("Закупка товаров {}", short_id(id)),
        "a028_missing_cost_registry" => format!("Реестр цен {}", short_id(id)),
        _ => format!("{registrator_type} {}", short_id(id)),
    }
}

fn registrator_label(registrator_type: &str, id: &str) -> String {
    match registrator_type {
        "a021_production_output" => format!("Выпуск продукции {}", short_id(id)),
        "a023_purchase_of_goods" => format!("Закупка товаров {}", short_id(id)),
        "a028_missing_cost_registry" => format!("Реестр цен {}", short_id(id)),
        _ => format!("{registrator_type} {}", short_id(id)),
    }
}

#[component]
pub fn ProductionTab(vm: NomenclatureDetailsVm) -> impl IntoView {
    let production_costs = vm.production_costs;
    let production_costs_loading = vm.production_costs_loading;
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    view! {
        <CardAnimated delay_ms=0 nav_id="a004_nomenclature_details_production_main">
            <h4 class="details-section__title">
                {move || format!("Производство ({})", production_costs.get().len())}
            </h4>

            <Show when=move || production_costs_loading.get()>
                <div style="padding: var(--spacing-md); display: flex; align-items: center; gap: var(--spacing-sm);">
                    <Spinner size=SpinnerSize::Small />
                    <span style="color: var(--color-text-tertiary);">"Загрузка записей..."</span>
                </div>
            </Show>

            <Show when=move || !production_costs_loading.get()>
                <Show
                    when=move || !production_costs.get().is_empty()
                    fallback=|| view! {
                        <div style="padding: var(--spacing-md); color: var(--color-text-tertiary);">
                            "Нет данных о производственной себестоимости"
                        </div>
                    }
                >
                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell resizable=true min_width=120.0>"Дата"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=240.0>"Документ"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=120.0>"Цена"</TableHeaderCell>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            <For
                                each=move || production_costs.get()
                                key=|item| item.id.clone()
                                children={
                                    let tabs_store = tabs_store.clone();
                                    move |item| {
                                        let tab_key = registrator_tab_key(
                                            &item.registrator_type,
                                            &item.registrator_ref,
                                        );
                                        let title = registrator_title(
                                            &item.registrator_type,
                                            &item.registrator_ref,
                                        );
                                        let label = registrator_label(
                                            &item.registrator_type,
                                            &item.registrator_ref,
                                        );

                                        view! {
                                            <TableRow>
                                                <TableCell>
                                                    <TableCellLayout truncate=true>
                                                        {format_period(&item.period)}
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell>
                                                    <TableCellLayout truncate=true>
                                                        {if let Some(tab_key) = tab_key {
                                                            view! {
                                                                <a
                                                                    href="#"
                                                                    style="color: var(--color-primary); text-decoration: underline; cursor: pointer;"
                                                                    on:click={
                                                                        let title = title.clone();
                                                                        let tabs_store = tabs_store.clone();
                                                                        move |ev: web_sys::MouseEvent| {
                                                                            ev.prevent_default();
                                                                            tabs_store.open_tab(&tab_key, &title);
                                                                        }
                                                                    }
                                                                >
                                                                    {label}
                                                                </a>
                                                            }.into_any()
                                                        } else {
                                                            view! { <span>{label}</span> }.into_any()
                                                        }}
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell>
                                                    <TableCellLayout truncate=true>
                                                        <div style="text-align: right; width: 100%; font-variant-numeric: tabular-nums;">
                                                            {format_price(item.cost)}
                                                        </div>
                                                    </TableCellLayout>
                                                </TableCell>
                                            </TableRow>
                                        }
                                    }
                                }
                            />
                        </TableBody>
                    </Table>
                </Show>
            </Show>
        </CardAnimated>
    }
}
