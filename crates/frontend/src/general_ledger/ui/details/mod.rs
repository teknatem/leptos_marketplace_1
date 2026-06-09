use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::date_utils::{format_date, format_datetime, format_datetime_space};
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_DETAIL;
use contracts::general_ledger::{
    GeneralLedgerEntryDto, GeneralLedgerTurnoverDto, GlResourceDetailResponse,
};
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::Value as JsonValue;
use std::collections::{BTreeSet, HashMap};
use thaw::*;

use crate::general_ledger::api::{
    fetch_general_ledger_entry_by_id, fetch_gl_resource_details, fetch_gl_turnover_by_code,
};

/// Тип ссылочного поля → ключ для кэша описаний.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum RefKind {
    ConnectionMp,
    Nomenclature,
    MarketplaceProduct,
    Organization,
}

impl RefKind {
    fn from_field_name(field: &str) -> Option<Self> {
        match field {
            "connection_mp_ref" => Some(Self::ConnectionMp),
            "nomenclature_ref" | "a004_nomenclature_ref" => Some(Self::Nomenclature),
            "marketplace_product_ref" => Some(Self::MarketplaceProduct),
            "organization_ref" => Some(Self::Organization),
            _ => None,
        }
    }

    fn tab_key(self, id: &str) -> String {
        match self {
            Self::ConnectionMp => format!("a006_connection_mp_details_{id}"),
            Self::Nomenclature => format!("a004_nomenclature_details_{id}"),
            Self::MarketplaceProduct => format!("a007_marketplace_product_details_{id}"),
            Self::Organization => format!("a002_organization_details_{id}"),
        }
    }

    fn tab_title_prefix(self) -> &'static str {
        match self {
            Self::ConnectionMp => "Connection MP",
            Self::Nomenclature => "Nomenclature",
            Self::MarketplaceProduct => "MP Product",
            Self::Organization => "Organization",
        }
    }
}

type RefLookups = HashMap<(RefKind, String), String>;

/// Сканирует строки detail-таблицы и запускает фоновые fetch'и для всех
/// уникальных ссылок известных типов. Каждое разрешение мутирует общий
/// сигнал `lookups`, чем триггерит реактивный перерендер карточек.
fn kick_off_ref_resolution(rows: &[JsonValue], lookups: RwSignal<RefLookups>) {
    let mut to_resolve: HashMap<RefKind, BTreeSet<String>> = HashMap::new();
    for row in rows {
        let JsonValue::Object(map) = row else {
            continue;
        };
        for (key, value) in map {
            let Some(kind) = RefKind::from_field_name(key) else {
                continue;
            };
            let id = match value {
                JsonValue::String(s) if !s.is_empty() => s.clone(),
                _ => continue,
            };
            to_resolve.entry(kind).or_default().insert(id);
        }
    }

    let already = lookups.get_untracked();
    for (kind, ids) in to_resolve {
        for id in ids {
            if already.contains_key(&(kind, id.clone())) {
                continue;
            }
            let lookups = lookups;
            spawn_local(async move {
                if let Some((k, id, label)) = resolve_ref(kind, id).await {
                    lookups.update(|map| {
                        map.insert((k, id), label);
                    });
                }
            });
        }
    }
}

async fn resolve_ref(kind: RefKind, id: String) -> Option<(RefKind, String, String)> {
    let url = match kind {
        RefKind::ConnectionMp => format!("{}/api/connection_mp/{}", api_base(), id),
        RefKind::Nomenclature => format!("{}/api/nomenclature/{}", api_base(), id),
        RefKind::MarketplaceProduct => format!("{}/api/marketplace_product/{}", api_base(), id),
        RefKind::Organization => format!("{}/api/organization/{}", api_base(), id),
    };

    let response = Request::get(&url).send().await.ok()?;
    if !response.ok() {
        return None;
    }
    let json: JsonValue = response.json().await.ok()?;

    let label = match kind {
        RefKind::ConnectionMp | RefKind::Organization => json
            .get("description")
            .and_then(JsonValue::as_str)
            .map(str::to_string),
        RefKind::Nomenclature | RefKind::MarketplaceProduct => {
            let desc = json
                .get("description")
                .and_then(JsonValue::as_str)
                .unwrap_or("");
            let article = json
                .get("article")
                .and_then(JsonValue::as_str)
                .unwrap_or("");
            if !article.is_empty() && !desc.is_empty() {
                Some(format!("{desc} (арт. {article})"))
            } else if !desc.is_empty() {
                Some(desc.to_string())
            } else if !article.is_empty() {
                Some(format!("арт. {article}"))
            } else {
                None
            }
        }
    };

    label.map(|l| (kind, id, l))
}

fn short_id(value: &str) -> &str {
    if value.len() >= 8 {
        &value[..8]
    } else {
        value
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

fn format_money(value: f64) -> String {
    format!("{value:.2}")
}

const MONEY_FIELDS: &[&str] = &[
    "amount",
    "sale_amount",
    "sum",
    "total",
    "debit_amount",
    "credit_amount",
    "wb_advert_sum",
    "ppvz_for_pay",
    "retail_amount",
    "delivery_amount",
    "return_amount",
];

fn json_cell_text(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "—".to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::String(s) => {
            if s.is_empty() {
                "—".to_string()
            } else {
                s.clone()
            }
        }
        JsonValue::Number(n) => n.to_string(),
        JsonValue::Array(_) | JsonValue::Object(_) => value.to_string(),
    }
}

/// Форматирует значение поля с учётом семантики:
/// для денежных полей округляет до 2 знаков, остальное — как обычно.
fn format_field_value(field: &str, value: &JsonValue) -> String {
    if MONEY_FIELDS.contains(&field) {
        if let Some(n) = value.as_f64() {
            return format_money(n);
        }
    }
    json_cell_text(value)
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

    let (active_tab, set_active_tab) = signal("general");
    let (turnover, set_turnover) = signal::<Option<GeneralLedgerTurnoverDto>>(None);
    let (turnover_loading, set_turnover_loading) = signal(false);
    let (resource_detail, set_resource_detail) = signal::<Option<GlResourceDetailResponse>>(None);
    let (resource_loading, set_resource_loading) = signal(false);
    let (resource_error, set_resource_error) = signal::<Option<String>>(None);
    let ref_lookups: RwSignal<RefLookups> = RwSignal::new(HashMap::new());
    // Представление кабинета МП (description) для блока «Глобальные измерения».
    let (connection_name, set_connection_name) = signal::<Option<String>>(None);

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

            // Резолвим представление кабинета МП (uuid → description).
            set_connection_name.set(None);
            if let Some(conn_id) = item
                .connection_mp_ref
                .clone()
                .filter(|value| !value.trim().is_empty())
            {
                spawn_local(async move {
                    if let Some((_, _, label)) =
                        resolve_ref(RefKind::ConnectionMp, conn_id).await
                    {
                        set_connection_name.set(Some(label));
                    }
                });
            }

            let code = item.turnover_code.clone();
            if !code.is_empty() {
                set_turnover_loading.set(true);
                spawn_local(async move {
                    match fetch_gl_turnover_by_code(&code).await {
                        Ok(t) => set_turnover.set(Some(t)),
                        Err(err) => {
                            leptos::logging::log!("Failed to load GL turnover '{}': {}", code, err)
                        }
                    }
                    set_turnover_loading.set(false);
                });
            }

            let entry_id = item.id.clone();
            set_resource_loading.set(true);
            set_resource_error.set(None);
            spawn_local(async move {
                match fetch_gl_resource_details(&entry_id).await {
                    Ok(d) => {
                        kick_off_ref_resolution(&d.rows, ref_lookups);
                        set_resource_detail.set(Some(d));
                    }
                    Err(err) => set_resource_error.set(Some(err)),
                }
                set_resource_loading.set(false);
            });
        }
    });

    let open_registrator = move |registrator_type: String, registrator_ref: String| {
        if let Some(key) = registrator_tab_key(&registrator_type, &registrator_ref) {
            tabs_store.open_tab(
                &key,
                &registrator_tab_label(&registrator_type, &registrator_ref),
            );
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
                            registrator_tab_key(&item.registrator_type, &item.registrator_ref)
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

            <div class="page__tabs">
                <button
                    class="page__tab"
                    class:page__tab--active=move || active_tab.get() == "general"
                    on:click=move |_| set_active_tab.set("general")
                >
                    {icon("file-text")} " Общие"
                </button>
                <button
                    class="page__tab"
                    class:page__tab--active=move || active_tab.get() == "resource"
                    on:click=move |_| set_active_tab.set("resource")
                >
                    {icon("list")} " Детализация"
                    {move || {
                        resource_detail.get().map(|d| {
                            let color = if d.totals.is_match {
                                BadgeColor::Success
                            } else {
                                BadgeColor::Danger
                            };
                            view! {
                                <Badge
                                    appearance=BadgeAppearance::Tint
                                    color=color
                                    attr:style="margin-left: 6px;"
                                >
                                    {d.totals.row_count.to_string()}
                                </Badge>
                            }
                        })
                    }}
                </button>
            </div>

            {move || {
                let Some(d) = resource_detail.get() else {
                    return view! { <></> }.into_any();
                };
                if d.totals.is_match {
                    return view! { <></> }.into_any();
                }
                let row_count = d.totals.row_count;
                let resource_field = d.resource_field.clone();
                let sign = d.resource_sign;
                let sum_signed = d.totals.sum_signed;
                let gl_amount = d.totals.gl_amount;
                let delta = d.totals.delta;
                let integrity = d.integrity.clone();
                let sum_ok = row_count >= 1 && (sum_signed - gl_amount).abs() <= 0.01;

                let (intent, title, summary) = if !d.supported {
                    (
                        MessageBarIntent::Warning,
                        "Сверка недоступна: ",
                        format!(
                            "Таблица {} не зарегистрирована в detail_links — сверка пропущена. amount проводки = {}.",
                            d.resource_table,
                            format_money(gl_amount),
                        ),
                    )
                } else if row_count == 0 {
                    (
                        MessageBarIntent::Error,
                        "Расхождение GL и детализации: ",
                        format!(
                            "Нет связанных строк в {} (ожидалось >= 1). amount проводки = {}.",
                            d.resource_table,
                            format_money(gl_amount),
                        ),
                    )
                } else if !integrity.is_ok {
                    let sample = if integrity.mismatched_refs_sample.is_empty() {
                        String::new()
                    } else {
                        format!(
                            " Примеры неверных ref: {}.",
                            integrity.mismatched_refs_sample.join(", ")
                        )
                    };
                    (
                        MessageBarIntent::Error,
                        "Нарушена целостность general_ledger_ref: ",
                        format!(
                            "из {} строк связано с этой проводкой только {}; без ref: {}; указывает на другую проводку: {}.{}",
                            row_count,
                            integrity.matched_count,
                            integrity.missing_count,
                            integrity.mismatched_count,
                            sample,
                        ),
                    )
                } else if !sum_ok {
                    (
                        MessageBarIntent::Error,
                        "Расхождение GL и детализации: ",
                        format!(
                            "Σ({}·{}) = {} ≠ amount = {} (Δ {}, строк: {}).",
                            resource_field,
                            sign,
                            format_money(sum_signed),
                            format_money(gl_amount),
                            format_money(delta),
                            row_count,
                        ),
                    )
                } else {
                    return view! { <></> }.into_any();
                };

                view! {
                    <div style="padding: var(--spacing-sm);">
                        <MessageBar intent=intent>
                            <span>
                                <strong>{title}</strong>
                                {summary}
                            </span>
                        </MessageBar>
                    </div>
                }
                .into_any()
            }}

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

                    match active_tab.get() {
                        "resource" => view! {
                            <ResourceDetailsTab
                                detail=resource_detail
                                loading=resource_loading
                                error=resource_error
                                ref_lookups=ref_lookups
                            />
                        }.into_any(),
                        _ => {
                            // Представление кабинета: resolved description → иначе сырой ref → «-».
                            let connection_display = connection_name
                                .get()
                                .filter(|value| !value.trim().is_empty())
                                .or_else(|| {
                                    item.connection_mp_ref
                                        .clone()
                                        .filter(|value| !value.trim().is_empty())
                                })
                                .unwrap_or_else(|| "-".to_string());
                            view! {
                            <GeneralTabContent
                                item=item
                                connection_display=connection_display
                                turnover=turnover
                                turnover_loading=turnover_loading
                                on_open_registrator=Callback::new(move |(t, r): (String, String)| {
                                    open_registrator(t, r)
                                })
                                on_open_resource=Callback::new(move |(t, r): (String, String)| {
                                    open_resource_target(t, r)
                                })
                            />
                        }.into_any()
                        },
                    }
                }}
            </div>
        </PageFrame>
    }
}

#[component]
fn GeneralTabContent(
    item: GeneralLedgerEntryDto,
    connection_display: String,
    turnover: ReadSignal<Option<GeneralLedgerTurnoverDto>>,
    turnover_loading: ReadSignal<bool>,
    on_open_registrator: Callback<(String, String)>,
    on_open_resource: Callback<(String, String)>,
) -> impl IntoView {
    let has_registrator_link =
        registrator_tab_key(&item.registrator_type, &item.registrator_ref).is_some();
    let registrator_type_for_click = item.registrator_type.clone();
    let registrator_ref_for_click = item.registrator_ref.clone();
    let has_resource_link = resource_tab_key(&item.resource_table, &item.registrator_ref).is_some();
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
                    on_click=move |_| on_open_registrator.run((
                        registrator_type_for_click.clone(),
                        registrator_ref_for_click.clone(),
                    ))
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
                    on_click=move |_| on_open_resource.run((
                        resource_table_for_click.clone(),
                        registrator_ref_for_resource_click.clone(),
                    ))
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

    let turnover_code = item.turnover_code.clone();
    let resource_table = item.resource_table.clone();
    let resource_field = item.resource_field.clone();
    let resource_sign = item.resource_sign.to_string();

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

                <CardAnimated delay_ms=40 nav_id="general_ledger_details_global_dims">
                    <h4 class="details-section__title">"Глобальные измерения"</h4>
                    <ReadonlyField
                        label="Order ID"
                        value=item.order_id.clone().unwrap_or_else(|| "-".to_string())
                    />
                    <ReadonlyField label="Connection MP" value=connection_display />
                    <ReadonlyField label="Layer" value=item.layer.as_str().to_string() />
                    <ReadonlyField
                        label="Субъект (entity)"
                        value=item.entity.clone().unwrap_or_else(|| "-".to_string())
                    />
                </CardAnimated>

                <CardAnimated delay_ms=80 nav_id="general_ledger_details_turnover">
                    <h4 class="details-section__title">"Данные оборота"</h4>
                    <ReadonlyField label="Turnover Code" value=turnover_code />
                    <ReadonlyField label="Resource Table" value=resource_table />
                    <ReadonlyField label="Resource Field" value=resource_field />
                    <ReadonlyField label="Resource Sign" value=resource_sign />
                    <div class="form__group">
                        <label class="form__label">"Описание оборота"</label>
                        {move || {
                            if turnover_loading.get() {
                                return view! { <span style="color: var(--color-text-secondary);">"Загрузка..."</span> }.into_any();
                            }
                            match turnover.get() {
                                Some(t) => {
                                    let description = if t.description.trim().is_empty() {
                                        if t.journal_comment.trim().is_empty() {
                                            "—".to_string()
                                        } else {
                                            t.journal_comment.clone()
                                        }
                                    } else {
                                        t.description.clone()
                                    };
                                    let formula = if t.formula_hint.trim().is_empty() {
                                        None
                                    } else {
                                        Some(t.formula_hint.clone())
                                    };
                                    view! {
                                        <Textarea
                                            value=RwSignal::new(description)
                                            attr:rows=4
                                            attr:readonly=true
                                        />
                                        {formula.map(|f| view! {
                                            <div style="margin-top: var(--spacing-xs); color: var(--color-text-secondary); font-size: var(--font-size-sm);">
                                                <strong>"Формула: "</strong>{f}
                                            </div>
                                        })}
                                    }.into_any()
                                }
                                None => view! { <span style="color: var(--color-text-secondary);">"—"</span> }.into_any(),
                            }
                        }}
                    </div>
                </CardAnimated>
            </div>

            <div class="detail-grid__col">
                <CardAnimated delay_ms=20 nav_id="general_ledger_details_accounts">
                    <h4 class="details-section__title">"Счета"</h4>
                    <ReadonlyField label="Layer" value=item.layer.as_str().to_string() />
                    <ReadonlyField label="Debit" value=item.debit_account.clone() />
                    <ReadonlyField label="Credit" value=item.credit_account.clone() />
                </CardAnimated>

                <CardAnimated delay_ms=60 nav_id="general_ledger_details_registrator">
                    <h4 class="details-section__title">"Регистратор"</h4>
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
    }
}

#[component]
fn ResourceDetailsTab(
    detail: ReadSignal<Option<GlResourceDetailResponse>>,
    loading: ReadSignal<bool>,
    error: ReadSignal<Option<String>>,
    ref_lookups: RwSignal<RefLookups>,
) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    view! {
        {move || {
            if loading.get() {
                return view! {
                    <Flex gap=FlexGap::Small style="align-items: center; justify-content: center; padding: var(--spacing-4xl);">
                        <Spinner />
                        <span>"Загрузка детализации..."</span>
                    </Flex>
                }.into_any();
            }

            if let Some(err) = error.get() {
                return view! {
                    <CardAnimated delay_ms=0 nav_id="general_ledger_details_resource_error">
                        <div class="alert alert--error">{err}</div>
                    </CardAnimated>
                }.into_any();
            }

            let Some(d) = detail.get() else {
                return view! { <div class="alert">"Нет данных."</div> }.into_any();
            };

            let resource_table = d.resource_table.clone();
            let resource_field = d.resource_field.clone();
            let resource_sign = d.resource_sign;
            let totals = d.totals.clone();
            let integrity = d.integrity.clone();
            let detail_error = d.error.clone();
            let rows = d.rows.clone();
            let gl_id = d.gl_id.clone();
            let records_title = format!("Связанные записи — {} ({})", resource_table, rows.len());
            let badge_table = format!("Таблица: {}", resource_table);
            let badge_field = format!("Поле: {} · знак: {}", resource_field, resource_sign);
            let badge_count = format!("Строк: {}", totals.row_count);
            let badge_sum = format!(
                "Σ({}·{}) = {}",
                resource_field,
                resource_sign,
                format_money(totals.sum_signed)
            );
            let badge_amount = format!("amount проводки: {}", format_money(totals.gl_amount));
            let sum_ok = totals.row_count >= 1 && (totals.sum_signed - totals.gl_amount).abs() <= 0.01;
            let badge_sum_match = if sum_ok {
                view! {
                    <Badge appearance=BadgeAppearance::Filled color=BadgeColor::Success>
                        "Σ совпадает"
                    </Badge>
                }.into_any()
            } else {
                let delta_text = format!("Σ Δ: {}", format_money(totals.delta));
                view! {
                    <Badge appearance=BadgeAppearance::Filled color=BadgeColor::Danger>
                        {delta_text}
                    </Badge>
                }.into_any()
            };
            let badge_integrity = if totals.row_count == 0 {
                view! { <span></span> }.into_any()
            } else if integrity.is_ok {
                view! {
                    <Badge appearance=BadgeAppearance::Filled color=BadgeColor::Success>
                        "general_ledger_ref OK"
                    </Badge>
                }.into_any()
            } else {
                let text = format!(
                    "general_ledger_ref: {}/{} ok, без ref: {}, чужой ref: {}",
                    integrity.matched_count,
                    totals.row_count,
                    integrity.missing_count,
                    integrity.mismatched_count,
                );
                view! {
                    <Badge appearance=BadgeAppearance::Filled color=BadgeColor::Danger>
                        {text}
                    </Badge>
                }.into_any()
            };

            view! {
                <CardAnimated delay_ms=0 nav_id="general_ledger_details_resource_summary">
                    <h4 class="details-section__title">"Сводка"</h4>
                    <Flex gap=FlexGap::Medium style="flex-wrap: wrap; align-items: center;">
                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Informative>
                            {badge_table}
                        </Badge>
                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Informative>
                            {badge_field}
                        </Badge>
                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                            {badge_count}
                        </Badge>
                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                            {badge_sum}
                        </Badge>
                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Informative>
                            {badge_amount}
                        </Badge>
                        {badge_sum_match}
                        {badge_integrity}
                    </Flex>
                    {detail_error.map(|err| view! {
                        <div style="margin-top: var(--spacing-md);">
                            <MessageBar intent=MessageBarIntent::Warning>
                                <span>{err}</span>
                            </MessageBar>
                        </div>
                    })}
                </CardAnimated>

                <CardAnimated delay_ms=60 nav_id="general_ledger_details_resource_records">
                    <h4 class="details-section__title">{records_title}</h4>
                    <ResourceRecordsList
                        rows=rows
                        resource_field=resource_field
                        gl_id=gl_id
                        ref_lookups=ref_lookups
                        tabs_store=tabs_store.clone()
                    />
                </CardAnimated>
            }.into_any()
        }}
    }
}

/// Поля, которые рендерятся отдельно (в шапке/бейджах карточки)
/// и не должны попадать в общий блок «Прочее».
const HEADER_FIELDS: &[&str] = &["turnover_code", "is_problem", "sale_amount"];

/// Малозначимые системные поля — выводятся мелким шрифтом внизу карточки.
const META_FIELDS: &[&str] = &[
    "id",
    "entry_date",
    "created_at",
    "updated_at",
    "layer",
    "value_kind",
    "agg_kind",
    "event_kind",
    "link_status",
    "registrator_type",
    "registrator_ref",
    "general_ledger_ref",
    "line_key",
    "line_event_key",
];

fn format_field_label(field: &str) -> String {
    let mut chars = field.chars();
    let first = chars.next().unwrap_or(' ').to_uppercase().to_string();
    let rest: String = chars.collect();
    let title = format!("{first}{rest}").replace('_', " ");
    title
}

#[component]
fn ResourceRecordsList(
    rows: Vec<JsonValue>,
    resource_field: String,
    gl_id: String,
    ref_lookups: RwSignal<RefLookups>,
    tabs_store: AppGlobalContext,
) -> impl IntoView {
    if rows.is_empty() {
        return view! {
            <div style="padding: var(--spacing-md); color: var(--color-text-secondary);">
                "Связанных строк не найдено."
            </div>
        }
        .into_any();
    }

    view! {
        <div style="display: flex; flex-direction: column; gap: var(--spacing-md);">
            {rows.into_iter().enumerate().map(|(idx, row)| {
                view! {
                    <ResourceRecordCard
                        index=idx
                        row=row
                        resource_field=resource_field.clone()
                        gl_id=gl_id.clone()
                        ref_lookups=ref_lookups
                        tabs_store=tabs_store.clone()
                    />
                }
            }).collect_view()}
        </div>
    }
    .into_any()
}

#[component]
fn ResourceRecordCard(
    index: usize,
    row: JsonValue,
    resource_field: String,
    gl_id: String,
    ref_lookups: RwSignal<RefLookups>,
    tabs_store: AppGlobalContext,
) -> impl IntoView {
    let JsonValue::Object(map) = row else {
        return view! {
            <div style="padding: var(--spacing-md); color: var(--color-text-secondary);">
                "Некорректный формат строки."
            </div>
        }
        .into_any();
    };

    let resource_value = map
        .get(&resource_field)
        .map(|v| format_field_value(&resource_field, v))
        .unwrap_or_else(|| "—".to_string());

    let turnover_code = map
        .get("turnover_code")
        .and_then(JsonValue::as_str)
        .unwrap_or("")
        .to_string();
    let is_problem = map
        .get("is_problem")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);
    let sale_amount = map
        .get("sale_amount")
        .map(|v| format_field_value("sale_amount", v));

    let mut ref_fields: Vec<(String, RefKind, String)> = Vec::new();
    let mut other_fields: Vec<(String, String)> = Vec::new();
    let mut meta_fields: Vec<(String, String)> = Vec::new();
    let mut order_key: Option<String> = None;

    for (key, value) in &map {
        if key == &resource_field {
            continue;
        }
        if HEADER_FIELDS.contains(&key.as_str()) {
            continue;
        }
        if key == "order_key" {
            let s = json_cell_text(value);
            if s != "—" && !s.is_empty() {
                order_key = Some(s);
            }
            continue;
        }
        if let Some(kind) = RefKind::from_field_name(key) {
            if let JsonValue::String(id) = value {
                if !id.is_empty() {
                    ref_fields.push((key.clone(), kind, id.clone()));
                    continue;
                }
            }
            other_fields.push((key.clone(), format_field_value(key, value)));
            continue;
        }
        if META_FIELDS.contains(&key.as_str()) {
            meta_fields.push((key.clone(), format_field_value(key, value)));
            continue;
        }
        other_fields.push((key.clone(), format_field_value(key, value)));
    }

    other_fields.sort_by(|a, b| a.0.cmp(&b.0));
    meta_fields.sort_by(|a, b| a.0.cmp(&b.0));
    ref_fields.sort_by(|a, b| a.0.cmp(&b.0));

    let registrator_type = map
        .get("registrator_type")
        .and_then(JsonValue::as_str)
        .unwrap_or("")
        .to_string();
    let registrator_ref = map
        .get("registrator_ref")
        .and_then(JsonValue::as_str)
        .unwrap_or("")
        .to_string();
    let general_ledger_ref = map
        .get("general_ledger_ref")
        .and_then(JsonValue::as_str)
        .map(str::to_string);
    let row_id = map
        .get("id")
        .and_then(JsonValue::as_str)
        .unwrap_or("")
        .to_string();

    let (gl_ref_class, gl_ref_text) = match general_ledger_ref.as_deref() {
        Some(r) if r == gl_id => ("badge badge--success", "general_ledger_ref ✓".to_string()),
        Some(r) if r.is_empty() => (
            "badge badge--error",
            "general_ledger_ref пустой".to_string(),
        ),
        Some(r) => (
            "badge badge--error",
            format!("general_ledger_ref → {}…", short_id(r)),
        ),
        None => ("badge badge--error", "general_ledger_ref NULL".to_string()),
    };
    let gl_ref_badge = view! {
        <span class=gl_ref_class>{gl_ref_text}</span>
    };

    let problem_badge = if is_problem {
        view! { <span class="badge badge--warning">"is_problem"</span> }.into_any()
    } else {
        view! { <></> }.into_any()
    };

    let turnover_badge = if turnover_code.is_empty() {
        view! { <></> }.into_any()
    } else {
        view! { <span class="badge badge--primary">{turnover_code}</span> }.into_any()
    };

    let sale_amount_field = sale_amount
        .filter(|v| v != "—" && v != "0.00" && v != "0")
        .map(|v| {
            view! {
                <div class="record-field">
                    <span class="record-field__label">"Sale amount:"</span>
                    <span class="record-field__value"><strong>{v}</strong></span>
                </div>
            }
        });

    let order_key_field = order_key.map(|srid| {
        let tabs = tabs_store.clone();
        let srid_for_click = srid.clone();
        let srid_for_title = srid.clone();
        view! {
            <div class="record-field">
                <span class="record-field__label">"Order key:"</span>
                <a
                    class="record-field__value record-field__link"
                    href="#"
                    on:click=move |ev: web_sys::MouseEvent| {
                        ev.prevent_default();
                        let encoded = urlencoding::encode(&srid_for_click);
                        tabs.open_tab(
                            &format!("d402_wb_order_flow_srid_{}", encoded),
                            "Вся история",
                        );
                    }
                    title=srid_for_title
                >
                    {srid}
                </a>
            </div>
        }
    });

    let has_registrator = registrator_tab_key(&registrator_type, &registrator_ref).is_some();
    let registrator_field = if !registrator_type.is_empty() {
        let tabs = tabs_store.clone();
        let regtype = registrator_type.clone();
        let regref = registrator_ref.clone();
        let title = if registrator_ref.is_empty() {
            registrator_type.clone()
        } else {
            format!("{} · {}", registrator_type, registrator_ref)
        };
        let label = if registrator_ref.is_empty() {
            registrator_type.clone()
        } else {
            format!("{} · {}", registrator_type, short_id(&registrator_ref))
        };
        let value_view = if has_registrator {
            view! {
                <a
                    class="record-field__value record-field__link"
                    href="#"
                    on:click=move |ev: web_sys::MouseEvent| {
                        ev.prevent_default();
                        if let Some(key) = registrator_tab_key(&regtype, &regref) {
                            tabs.open_tab(&key, &registrator_tab_label(&regtype, &regref));
                        }
                    }
                    title=title
                >
                    {label}
                </a>
            }
            .into_any()
        } else {
            view! { <span class="record-field__value" title=title>{label}</span> }.into_any()
        };
        view! {
            <div class="record-field">
                <span class="record-field__label">"Registrator:"</span>
                {value_view}
            </div>
        }
        .into_any()
    } else {
        view! { <></> }.into_any()
    };

    let ref_field_views = ref_fields
        .into_iter()
        .map(|(field_name, kind, id)| {
            let tabs = tabs_store.clone();
            let id_click = id.clone();
            let id_title = id.clone();
            let id_lookup = id.clone();
            let id_short = short_id(&id).to_string();
            let label_text = format!("{}:", format_field_label(&field_name));
            view! {
                <div class="record-field">
                    <span class="record-field__label">{label_text}</span>
                    <a
                        class="record-field__value record-field__link"
                        href="#"
                        on:click=move |ev: web_sys::MouseEvent| {
                            ev.prevent_default();
                            tabs.open_tab(
                                &kind.tab_key(&id_click),
                                &format!("{} {}", kind.tab_title_prefix(), short_id(&id_click)),
                            );
                        }
                        title=id_title
                    >
                        {move || {
                            ref_lookups
                                .get()
                                .get(&(kind, id_lookup.clone()))
                                .cloned()
                                .unwrap_or_else(|| format!("{}…", id_short))
                        }}
                    </a>
                </div>
            }
        })
        .collect_view();

    let other_field_views = other_fields
        .into_iter()
        .map(|(field, value)| {
            let label = format!("{}:", format_field_label(&field));
            view! {
                <div class="record-field">
                    <span class="record-field__label">{label}</span>
                    <span class="record-field__value">{value}</span>
                </div>
            }
        })
        .collect_view();

    let meta_summary = if meta_fields.is_empty() && row_id.is_empty() {
        None
    } else {
        let parts: Vec<String> = std::iter::once(if row_id.is_empty() {
            String::new()
        } else {
            format!("id: {}", short_id(&row_id))
        })
        .filter(|s| !s.is_empty())
        .chain(
            meta_fields
                .into_iter()
                .map(|(k, v)| format!("{}: {}", k, v)),
        )
        .collect();
        Some(parts.join(" · "))
    };

    let card_style = if is_problem {
        "border: 1px solid var(--color-warning-200, #f1c40f); border-left: 4px solid var(--color-warning, #f39c12); border-radius: var(--radius-sm); padding: var(--spacing-md); background: var(--color-surface);"
    } else {
        "border: 1px solid var(--color-neutral-stroke-2, #e5e5e5); border-left: 4px solid var(--color-brand, #0078d4); border-radius: var(--radius-sm); padding: var(--spacing-md); background: var(--color-surface);"
    };

    view! {
        <div style=card_style>
            <div style="display: flex; gap: var(--spacing-sm); align-items: center; flex-wrap: wrap; margin-bottom: var(--spacing-sm);">
                <span style="font-size: var(--font-size-base); color: var(--color-text-secondary); flex-shrink: 0;">
                    {format!("#{}", index + 1)}
                </span>
                <span style="font-size: var(--font-size-lg); font-weight: 700; flex-shrink: 0;">
                    {format!("{}: {}", resource_field, resource_value)}
                </span>
                {turnover_badge}
                {problem_badge}
                {gl_ref_badge}
            </div>

            <div class="record-grid">
                {sale_amount_field}
                {ref_field_views}
                {order_key_field}
                {registrator_field}
                {other_field_views}
            </div>

            {meta_summary.map(|s| view! {
                <div style="margin-top: var(--spacing-sm); padding-top: var(--spacing-xs); border-top: 1px dashed var(--color-neutral-stroke-2, #e5e5e5); color: var(--color-text-secondary); font-size: var(--font-size-sm); font-family: var(--font-family-mono, monospace); word-break: break-all;">
                    {s}
                </div>
            })}
        </div>
    }
    .into_any()
}
