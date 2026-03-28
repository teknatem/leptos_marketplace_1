use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::date_utils::{format_date, format_datetime, format_datetime_space};
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_DETAIL;
use contracts::projections::general_ledger::GeneralLedgerEntryDto;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

use super::model::fetch_general_ledger_entry_by_id;

fn short_id(value: &str) -> &str {
    if value.len() >= 8 {
        &value[..8]
    } else {
        value
    }
}

fn parse_registrator_ref(value: &str) -> (&str, &str) {
    if let Some(pos) = value.find(':') {
        (&value[..pos], &value[pos + 1..])
    } else {
        ("", value)
    }
}

fn p903_tab_key_from_ref(value: &str) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(format!(
            "p903_wb_finance_report_details_id_{}",
            urlencoding::encode(value)
        ))
    }
}

fn p903_tab_label(value: &str) -> String {
    format!("WB Finance {}", short_id(value))
}

fn registrator_tab_key(registrator_type: &str, id: &str) -> Option<String> {
    match registrator_type {
        "a012_wb_sales" => Some(format!("a012_wb_sales_details_{id}")),
        "a013_ym_order" => Some(format!("a013_ym_order_details_{id}")),
        "a014_ozon_transactions" => Some(format!("a014_ozon_transactions_details_{id}")),
        "a015_wb_orders" => Some(format!("a015_wb_orders_details_{id}")),
        "a016_ym_returns" => Some(format!("a016_ym_returns_details_{id}")),
        "a021_production_output" => Some(format!("a021_production_output_details_{id}")),
        "a022_kit_variant" => Some(format!("a022_kit_variant_details_{id}")),
        "a023_purchase_of_goods" => Some(format!("a023_purchase_of_goods_details_{id}")),
        "a026_wb_advert_daily" => Some(format!("a026_wb_advert_daily_details_{id}")),
        "p903_wb_finance_report" => p903_tab_key_from_ref(id),
        _ => None,
    }
}

fn registrator_tab_label(registrator_type: &str, id: &str) -> String {
    match registrator_type {
        "a012_wb_sales" => format!("WB Sale {}", short_id(id)),
        "a013_ym_order" => format!("YM Order {}", short_id(id)),
        "a014_ozon_transactions" => format!("OZON Transaction {}", short_id(id)),
        "a015_wb_orders" => format!("WB Order {}", short_id(id)),
        "a016_ym_returns" => format!("YM Return {}", short_id(id)),
        "a026_wb_advert_daily" => format!("WB Ads {}", short_id(id)),
        "p903_wb_finance_report" => p903_tab_label(id),
        _ => format!("{registrator_type} :: {}", short_id(id)),
    }
}

fn resource_tab_key(resource_table: &str, registrator_ref: &str) -> Option<String> {
    match resource_table {
        "p903_wb_finance_report" => p903_tab_key_from_ref(registrator_ref),
        _ => None,
    }
}

fn resource_tab_label(resource_table: &str, registrator_ref: &str) -> String {
    match resource_table {
        "p903_wb_finance_report" => p903_tab_label(registrator_ref),
        _ => format!("{resource_table} :: {}", short_id(registrator_ref)),
    }
}

fn format_general_ledger_datetime(value: &str) -> String {
    if value.contains('T') {
        format_datetime(value)
    } else if value.contains(' ') {
        format_datetime_space(value)
    } else {
        format_date(value)
    }
}

fn format_optional_number(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "-".to_string())
}

#[component]
fn ReadonlyField(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="form__group">
            <label class="form__label">{label}</label>
            <Input value=RwSignal::new(value) attr:readonly=true />
        </div>
    }
}

#[component]
pub fn GeneralLedgerDetailsPage(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let detail_id = StoredValue::new(id.clone());
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal::<Option<String>>(None);
    let (entry, set_entry) = signal::<Option<GeneralLedgerEntryDto>>(None);

    Effect::new(move |_| {
        let id = id.clone();
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            match fetch_general_ledger_entry_by_id(&id).await {
                Ok(item) => set_entry.set(Some(item)),
                Err(err) => set_error.set(Some(err)),
            }
            set_loading.set(false);
        });
    });

    Effect::new(move |_| {
        if let Some(item) = entry.get() {
            tabs_store.update_tab_title(
                &format!("general_ledger_details_{}", detail_id.get_value()),
                &format!("General Ledger {}", short_id(&item.id)),
            );
        }
    });

    let open_registrator = move |registrator_type: String, registrator_ref: String| {
        let (_, id) = parse_registrator_ref(&registrator_ref);
        let id = id.to_string();
        if let Some(key) = registrator_tab_key(&registrator_type, &id) {
            tabs_store.open_tab(&key, &registrator_tab_label(&registrator_type, &id));
        }
    };

    let open_resource_target = move |resource_table: String, registrator_ref: String| {
        if let Some(key) = resource_tab_key(&resource_table, &registrator_ref) {
            tabs_store.open_tab(&key, &resource_tab_label(&resource_table, &registrator_ref));
        }
    };

    view! {
        <PageFrame page_id="general_ledger--detail" category=PAGE_CAT_DETAIL>
            <div class="page__header">
                <div class="page__header-left">
                    {icon("database")}
                    <h1 class="page__title">
                        {move || {
                            entry.get()
                                .map(|item| format!("General Ledger {}", short_id(&item.id)))
                                .unwrap_or_else(|| "General Ledger".to_string())
                        }}
                    </h1>
                    <Show when=move || entry.get().is_some()>
                        {move || {
                            let turnover_code = entry.get().map(|item| item.turnover_code).unwrap_or_default();
                            view! {
                                <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                                    {turnover_code}
                                </Badge>
                            }
                        }}
                    </Show>
                </div>
                <div class="page__header-right">
                    <Show when=move || {
                        entry.get().and_then(|item| {
                            let (_, reg_id) = parse_registrator_ref(&item.registrator_ref);
                            registrator_tab_key(&item.registrator_type, reg_id)
                        }).is_some()
                    }>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            on_click=move |_| {
                                if let Some(item) = entry.get() {
                                    open_registrator(item.registrator_type, item.registrator_ref);
                                }
                            }
                        >
                            {icon("external-link")}
                            " Open Registrator"
                        </Button>
                    </Show>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| on_close.run(())
                    >
                        "Close"
                    </Button>
                </div>
            </div>

            <div class="page__content">
                {move || {
                    if loading.get() {
                        return view! {
                            <Flex gap=FlexGap::Small style="align-items: center; justify-content: center; padding: var(--spacing-4xl);">
                                <Spinner />
                                <span>"Loading..."</span>
                            </Flex>
                        }.into_any();
                    }

                    if let Some(err) = error.get() {
                        return view! { <div class="alert alert--error">{err}</div> }.into_any();
                    }

                    let Some(item) = entry.get() else {
                        return view! { <div class="alert">"Entry not found."</div> }.into_any();
                    };

                    let (_, reg_id) = parse_registrator_ref(&item.registrator_ref);
                    let has_registrator_link =
                        registrator_tab_key(&item.registrator_type, reg_id).is_some();
                    let registrator_type_for_click = item.registrator_type.clone();
                    let registrator_ref_for_click = item.registrator_ref.clone();
                    let has_resource_link =
                        resource_tab_key(&item.resource_table, &item.registrator_ref).is_some();
                    let resource_table_for_click = item.resource_table.clone();
                    let registrator_ref_for_resource_click = item.registrator_ref.clone();
                    let comment = if item.comment.trim().is_empty() {
                        "-".to_string()
                    } else {
                        item.comment.clone()
                    };

                    let registrator_button = if has_registrator_link {
                        view! {
                            <div class="form__group">
                                <label class="form__label">"Registrator"</label>
                                <Button
                                    appearance=ButtonAppearance::Secondary
                                    on_click=move |_| open_registrator(
                                        registrator_type_for_click.clone(),
                                        registrator_ref_for_click.clone(),
                                    )
                                    attr:style="width: 100%; justify-content: flex-start;"
                                >
                                    "Open registrator"
                                </Button>
                            </div>
                        }
                        .into_any()
                    } else {
                        view! { <></> }.into_any()
                    };

                    let detail_button = if has_resource_link {
                        view! {
                            <div class="form__group">
                                <label class="form__label">"Resource"</label>
                                <Button
                                    appearance=ButtonAppearance::Secondary
                                    on_click=move |_| open_resource_target(
                                        resource_table_for_click.clone(),
                                        registrator_ref_for_resource_click.clone(),
                                    )
                                    attr:style="width: 100%; justify-content: flex-start;"
                                >
                                    "Open resource"
                                </Button>
                            </div>
                        }
                        .into_any()
                    } else {
                        view! { <></> }.into_any()
                    };

                    view! {
                        <div class="detail-grid">
                            <div class="detail-grid__col">
                                <CardAnimated delay_ms=0 nav_id="general_ledger_details_main">
                                    <h4 class="details-section__title">"Main"</h4>
                                    <ReadonlyField label="ID" value=item.id.clone() />
                                    <ReadonlyField label="Entry Date" value=format_general_ledger_datetime(&item.entry_date) />
                                    <ReadonlyField label="Created At" value=format_general_ledger_datetime(&item.created_at) />
                                    <ReadonlyField label="Amount" value=format!("{:.2}", item.amount) />
                                    <ReadonlyField label="Qty" value=format_optional_number(item.qty) />
                                </CardAnimated>

                                <CardAnimated delay_ms=80 nav_id="general_ledger_details_accounts">
                                    <h4 class="details-section__title">"Accounts"</h4>
                                    <ReadonlyField label="Layer" value=item.layer.as_str().to_string() />
                                    <ReadonlyField label="Debit" value=item.debit_account.clone() />
                                    <ReadonlyField label="Credit" value=item.credit_account.clone() />
                                    <ReadonlyField label="Turnover Code" value=item.turnover_code.clone() />
                                    <ReadonlyField
                                        label="Cabinet MP"
                                        value=item.cabinet_mp.clone().unwrap_or_else(|| "-".to_string())
                                    />
                                    <ReadonlyField label="Resource Table" value=item.resource_table.clone() />
                                    <ReadonlyField label="Resource Field" value=item.resource_field.clone() />
                                    <ReadonlyField label="Resource Sign" value=item.resource_sign.to_string() />
                                </CardAnimated>
                            </div>

                            <div class="detail-grid__col">
                                <CardAnimated delay_ms=40 nav_id="general_ledger_details_registrator">
                                    <h4 class="details-section__title">"Registrator"</h4>
                                    <ReadonlyField label="Registrator Type" value=item.registrator_type.clone() />
                                    <ReadonlyField label="Registrator Ref" value=item.registrator_ref.clone() />
                                    {registrator_button}
                                    {detail_button}
                                </CardAnimated>

                                <CardAnimated delay_ms=120 nav_id="general_ledger_details_comment">
                                    <h4 class="details-section__title">"Comment"</h4>
                                    <div class="form__group">
                                        <label class="form__label">"Comment"</label>
                                        <Textarea value=RwSignal::new(comment) attr:rows=6 attr:readonly=true />
                                    </div>
                                </CardAnimated>
                            </div>
                        </div>
                    }.into_any()
                }}
            </div>
        </PageFrame>
    }
}
