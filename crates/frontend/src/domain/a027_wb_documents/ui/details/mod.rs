use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::auth_download::download_authenticated_file;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::components::ui::FieldDisplay;
use crate::shared::export::{export_to_excel, ExcelExportable};
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_DETAIL;
use crate::system::favorites::ui::FavoriteButton;
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use thaw::*;

fn fmt_dt(value: &str) -> String {
    if let Some((date, time)) = value.split_once('T') {
        let time_clean = time
            .split('Z')
            .next()
            .unwrap_or(time)
            .split('+')
            .next()
            .unwrap_or(time)
            .split('.')
            .next()
            .unwrap_or(time);
        if let Some((year, rest)) = date.split_once('-') {
            if let Some((month, day)) = rest.split_once('-') {
                return format!("{}.{}.{} {}", day, month, year, time_clean);
            }
        }
    }
    value.to_string()
}

fn format_amount(amount: f64) -> String {
    let sign = if amount < 0.0 { "-" } else { "" };
    let raw = format!("{:.2}", amount.abs());
    let (whole, fraction) = raw.split_once('.').unwrap_or((raw.as_str(), "00"));
    let mut grouped = String::new();
    for (idx, ch) in whole.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            grouped.push(' ');
        }
        grouped.push(ch);
    }
    let whole_grouped: String = grouped.chars().rev().collect();
    format!("{}{}.{}", sign, whole_grouped, fraction)
}

fn fmt_optional_amount(value: Option<f64>) -> String {
    value.map(format_amount).unwrap_or_else(|| "?".to_string())
}

fn fmt_optional_percent(value: Option<f64>) -> String {
    value
        .map(|amount| format!("{:.2}%", amount))
        .unwrap_or_else(|| "?".to_string())
}

fn fmt_reconciliation_difference(line: &ReconciliationLineDto) -> String {
    if !line.is_available {
        return "?".to_string();
    }

    match line.difference_percent {
        Some(percent) => format!(
            "{} ({})",
            fmt_optional_amount(line.difference_amount),
            fmt_optional_percent(Some(percent)),
        ),
        None => fmt_optional_amount(line.difference_amount),
    }
}

fn fmt_csv_decimal(value: Option<f64>) -> String {
    value
        .map(|amount| format!("{:.2}", amount).replace('.', ","))
        .unwrap_or_default()
}

/// Одна строка таблицы сверки для выгрузки в CSV (Excel).
struct CheckExportRow {
    indicator: String,
    formula: String,
    wb_report: String,
    database_value: String,
    difference: String,
}

impl ExcelExportable for CheckExportRow {
    fn headers() -> Vec<&'static str> {
        vec![
            "Показатель",
            "Формула оборотов",
            "Отчет WB",
            "Данные в базе",
            "Расхождение",
        ]
    }

    fn to_csv_row(&self) -> Vec<String> {
        vec![
            self.indicator.clone(),
            self.formula.clone(),
            self.wb_report.clone(),
            self.database_value.clone(),
            self.difference.clone(),
        ]
    }
}

fn check_export_row(
    label: &str,
    wb_report_value: String,
    line: &ReconciliationLineDto,
) -> CheckExportRow {
    CheckExportRow {
        indicator: label.to_string(),
        formula: line.formula.clone(),
        wb_report: fmt_csv_decimal(parse_optional_amount(wb_report_value)),
        database_value: if line.is_available {
            fmt_csv_decimal(line.database_value)
        } else {
            "?".to_string()
        },
        difference: fmt_csv_decimal(line.difference_amount),
    }
}

fn parse_optional_amount(value: String) -> Option<f64> {
    let normalized = value.trim().replace(' ', "").replace(',', ".");
    if normalized.is_empty() {
        None
    } else {
        normalized.parse::<f64>().ok()
    }
}

fn effective_document_date(period_to: Option<&String>, creation_time: &str) -> String {
    period_to.cloned().unwrap_or_else(|| fmt_dt(creation_time))
}

#[derive(Debug, Clone, Deserialize)]
struct DetailsDto {
    id: String,
    service_name: String,
    name: String,
    category: String,
    creation_time: String,
    viewed: bool,
    extensions: Vec<String>,
    connection_id: String,
    connection_name: Option<String>,
    organization_id: String,
    organization_name: Option<String>,
    marketplace_id: String,
    marketplace_name: Option<String>,
    is_weekly_report: bool,
    report_period_from: Option<String>,
    report_period_to: Option<String>,
    realized_goods_total: Option<f64>,
    wb_reward_with_vat: Option<f64>,
    seller_transfer_total: Option<f64>,
    other_deductions: Option<f64>,
    logistics: Option<f64>,
    acquiring: Option<f64>,
    #[serde(default)]
    max_deviation: Option<f64>,
    #[serde(default)]
    comment: Option<String>,
    reconciliation: ReconciliationDto,
    fact_reconciliation: ReconciliationDto,
    fetched_at: String,
    locale: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ReconciliationDto {
    realized_goods_total: ReconciliationLineDto,
    wb_reward_with_vat: ReconciliationLineDto,
    seller_transfer_total: ReconciliationLineDto,
    advert_other_deductions: ReconciliationLineDto,
    logistics: ReconciliationLineDto,
    acquiring: ReconciliationLineDto,
}

#[derive(Debug, Clone, Deserialize)]
struct ReconciliationLineDto {
    formula: String,
    database_value: Option<f64>,
    difference_amount: Option<f64>,
    difference_percent: Option<f64>,
    is_available: bool,
}

#[derive(Debug, Clone, Serialize)]
struct UpdateManualFieldsRequest {
    is_weekly_report: bool,
    report_period_from: Option<String>,
    report_period_to: Option<String>,
    realized_goods_total: Option<f64>,
    wb_reward_with_vat: Option<f64>,
    seller_transfer_total: Option<f64>,
    other_deductions: Option<f64>,
    logistics: Option<f64>,
    acquiring: Option<f64>,
    comment: Option<String>,
}

#[component]
fn ReadField(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="form__group">
            <label class="form__label">{label}</label>
            <FieldDisplay value=value />
        </div>
    }
}

#[component]
fn EditField(
    label: &'static str,
    #[prop(into)] value: RwSignal<String>,
    #[prop(default = "text".to_string())] input_type: String,
    #[prop(default = "".to_string())] placeholder: String,
) -> impl IntoView {
    view! {
        <div class="form__group">
            <label class="form__label">{label}</label>
            <input
                class="form__input"
                type=input_type
                prop:value=move || value.get()
                placeholder=placeholder
                on:input=move |ev| value.set(event_target_value(&ev))
            />
        </div>
    }
}

#[component]
fn DocumentTabBar(selected_tab: RwSignal<String>) -> impl IntoView {
    view! {
        <div class="page__tabs">
            <button
                class="page__tab"
                class:page__tab--active=move || selected_tab.get() == "general"
                on:click=move |_| selected_tab.set("general".to_string())
            >
                {icon("file-text")} "Общие"
            </button>
            <button
                class="page__tab"
                class:page__tab--active=move || selected_tab.get() == "check"
                on:click=move |_| selected_tab.set("check".to_string())
            >
                {icon("check-square")} "Проверка"
            </button>
            <button
                class="page__tab"
                class:page__tab--active=move || selected_tab.get() == "meta"
                on:click=move |_| selected_tab.set("meta".to_string())
            >
                {icon("database")} "Служебное"
            </button>
        </div>
    }
}

#[component]
pub fn WbDocumentsDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let tabs = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let stored_id = StoredValue::new(id.clone());
    let (loading, set_loading) = signal(true);
    let (saving, set_saving) = signal(false);
    let (posting, set_posting) = signal(false);
    let (extracting, set_extracting) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (success, set_success) = signal::<Option<String>>(None);
    let (doc, set_doc) = signal::<Option<DetailsDto>>(None);
    let selected_tab = RwSignal::new("general".to_string());

    let is_weekly_report = RwSignal::new(false);
    let report_period_from = RwSignal::new(String::new());
    let report_period_to = RwSignal::new(String::new());
    let realized_goods_total = RwSignal::new(String::new());
    let wb_reward_with_vat = RwSignal::new(String::new());
    let seller_transfer_total = RwSignal::new(String::new());
    let other_deductions = RwSignal::new(String::new());
    let logistics = RwSignal::new(String::new());
    let acquiring = RwSignal::new(String::new());
    let comment = RwSignal::new(String::new());

    let apply_doc_to_form = move |data: &DetailsDto| {
        is_weekly_report.set(data.is_weekly_report);
        report_period_from.set(data.report_period_from.clone().unwrap_or_default());
        report_period_to.set(data.report_period_to.clone().unwrap_or_default());
        realized_goods_total.set(
            data.realized_goods_total
                .map(format_amount)
                .unwrap_or_default(),
        );
        wb_reward_with_vat.set(
            data.wb_reward_with_vat
                .map(format_amount)
                .unwrap_or_default(),
        );
        seller_transfer_total.set(
            data.seller_transfer_total
                .map(format_amount)
                .unwrap_or_default(),
        );
        other_deductions.set(data.other_deductions.map(format_amount).unwrap_or_default());
        logistics.set(data.logistics.map(format_amount).unwrap_or_default());
        acquiring.set(data.acquiring.map(format_amount).unwrap_or_default());
        comment.set(data.comment.clone().unwrap_or_default());
    };

    let load_doc = {
        let tabs = tabs.clone();
        Callback::new(move |()| {
            let current_id = stored_id.get_value();
            let tab_id = stored_id.get_value();
            let tabs = tabs.clone();
            spawn_local(async move {
                set_loading.set(true);
                set_error.set(None);
                set_success.set(None);
                match Request::get(&format!(
                    "{}/api/a027/wb-documents/{}",
                    api_base(),
                    current_id
                ))
                .send()
                .await
                {
                    Ok(resp) if resp.ok() => match resp.json::<DetailsDto>().await {
                        Ok(data) => {
                            tabs.update_tab_title(
                                &format!("a027_wb_documents_details_{}", tab_id),
                                &format!(
                                    "WB Doc {}",
                                    effective_document_date(
                                        data.report_period_to.as_ref(),
                                        &data.creation_time,
                                    )
                                ),
                            );
                            apply_doc_to_form(&data);
                            set_doc.set(Some(data));
                        }
                        Err(err) => set_error.set(Some(format!("Ошибка парсинга: {}", err))),
                    },
                    Ok(resp) => {
                        set_error.set(Some(format!("Ошибка сервера: HTTP {}", resp.status())))
                    }
                    Err(err) => set_error.set(Some(format!("Ошибка сети: {}", err))),
                }
                set_loading.set(false);
            });
        })
    };

    Effect::new({
        let load_doc = load_doc.clone();
        move |_| load_doc.run(())
    });

    let start_download = Callback::new(
        move |(document_id, service_name, extension): (String, String, String)| {
            spawn_local(async move {
                let url = format!(
                    "{}/api/a027/wb-documents/{}/download/{}",
                    api_base(),
                    document_id,
                    urlencoding::encode(&extension)
                );
                let fallback_filename = format!("{}_document.{}", service_name, extension);
                if let Err(err) = download_authenticated_file(&url, &fallback_filename).await {
                    set_error.set(Some(format!("Ошибка скачивания: {}", err)));
                }
            });
        },
    );

    let on_save = Callback::new(move |()| {
        let Some(current_doc) = doc.get() else {
            return;
        };

        let request = UpdateManualFieldsRequest {
            is_weekly_report: is_weekly_report.get(),
            report_period_from: {
                let value = report_period_from.get();
                (!value.trim().is_empty()).then_some(value)
            },
            report_period_to: {
                let value = report_period_to.get();
                (!value.trim().is_empty()).then_some(value)
            },
            realized_goods_total: parse_optional_amount(realized_goods_total.get()),
            wb_reward_with_vat: parse_optional_amount(wb_reward_with_vat.get()),
            seller_transfer_total: parse_optional_amount(seller_transfer_total.get()),
            other_deductions: parse_optional_amount(other_deductions.get()),
            logistics: parse_optional_amount(logistics.get()),
            acquiring: parse_optional_amount(acquiring.get()),
            comment: {
                let value = comment.get();
                (!value.trim().is_empty()).then_some(value)
            },
        };

        let document_id = current_doc.id.clone();
        set_saving.set(true);
        set_error.set(None);
        set_success.set(None);

        spawn_local(async move {
            match Request::put(&format!(
                "{}/api/a027/wb-documents/{}/manual",
                api_base(),
                document_id
            ))
            .json(&request)
            {
                Ok(req) => match req.send().await {
                    Ok(resp) if resp.ok() => match resp.json::<DetailsDto>().await {
                        Ok(updated) => {
                            apply_doc_to_form(&updated);
                            set_doc.set(Some(updated));
                            set_success.set(Some("Поля сохранены".to_string()));
                        }
                        Err(err) => {
                            set_error.set(Some(format!("Ошибка парсинга ответа: {}", err)));
                        }
                    },
                    Ok(resp) => {
                        set_error.set(Some(format!("Ошибка сохранения: HTTP {}", resp.status())));
                    }
                    Err(err) => {
                        set_error.set(Some(format!("Ошибка сети: {}", err)));
                    }
                },
                Err(err) => {
                    set_error.set(Some(format!("Ошибка подготовки запроса: {}", err)));
                }
            }

            set_saving.set(false);
        });
    });

    let on_post = Callback::new(move |()| {
        let Some(current_doc) = doc.get() else {
            return;
        };

        let request = UpdateManualFieldsRequest {
            is_weekly_report: is_weekly_report.get(),
            report_period_from: {
                let value = report_period_from.get();
                (!value.trim().is_empty()).then_some(value)
            },
            report_period_to: {
                let value = report_period_to.get();
                (!value.trim().is_empty()).then_some(value)
            },
            realized_goods_total: parse_optional_amount(realized_goods_total.get()),
            wb_reward_with_vat: parse_optional_amount(wb_reward_with_vat.get()),
            seller_transfer_total: parse_optional_amount(seller_transfer_total.get()),
            other_deductions: parse_optional_amount(other_deductions.get()),
            logistics: parse_optional_amount(logistics.get()),
            acquiring: parse_optional_amount(acquiring.get()),
            comment: {
                let value = comment.get();
                (!value.trim().is_empty()).then_some(value)
            },
        };

        let document_id = current_doc.id.clone();
        set_posting.set(true);
        set_error.set(None);
        set_success.set(None);

        spawn_local(async move {
            let manual_result = match Request::put(&format!(
                "{}/api/a027/wb-documents/{}/manual",
                api_base(),
                document_id
            ))
            .json(&request)
            {
                Ok(req) => req.send().await,
                Err(err) => {
                    set_error.set(Some(format!("Ошибка подготовки запроса: {}", err)));
                    set_posting.set(false);
                    return;
                }
            };

            match manual_result {
                Ok(resp) if resp.ok() => {}
                Ok(resp) => {
                    set_error.set(Some(format!("Ошибка сохранения: HTTP {}", resp.status())));
                    set_posting.set(false);
                    return;
                }
                Err(err) => {
                    set_error.set(Some(format!("Ошибка сети: {}", err)));
                    set_posting.set(false);
                    return;
                }
            }

            match Request::post(&format!(
                "{}/api/a027/wb-documents/{}/post",
                api_base(),
                document_id
            ))
            .send()
            .await
            {
                Ok(resp) if resp.ok() => match resp.json::<DetailsDto>().await {
                    Ok(updated) => {
                        apply_doc_to_form(&updated);
                        set_doc.set(Some(updated));
                        set_success.set(Some(
                            "Документ проведен, данные сверки обновлены".to_string(),
                        ));
                    }
                    Err(err) => {
                        set_error.set(Some(format!("Ошибка парсинга ответа: {}", err)));
                    }
                },
                Ok(resp) => {
                    set_error.set(Some(format!("Ошибка проведения: HTTP {}", resp.status())));
                }
                Err(err) => {
                    set_error.set(Some(format!("Ошибка сети: {}", err)));
                }
            }

            set_posting.set(false);
        });
    });

    let on_extract_weekly_report = Callback::new(move |()| {
        let Some(current_doc) = doc.get() else {
            return;
        };

        let document_id = current_doc.id.clone();
        set_extracting.set(true);
        set_error.set(None);
        set_success.set(None);

        spawn_local(async move {
            match Request::post(&format!(
                "{}/api/a027/wb-documents/{}/extract-weekly-report",
                api_base(),
                document_id
            ))
            .send()
            .await
            {
                Ok(resp) if resp.ok() => match resp.json::<DetailsDto>().await {
                    Ok(updated) => {
                        apply_doc_to_form(&updated);
                        set_doc.set(Some(updated));
                        set_success.set(Some(
                            "Данные отчета извлечены из PDF и заполнены".to_string(),
                        ));
                    }
                    Err(err) => {
                        set_error.set(Some(format!("Ошибка парсинга ответа: {}", err)));
                    }
                },
                Ok(resp) => {
                    set_error.set(Some(format!(
                        "Ошибка извлечения отчета: HTTP {}",
                        resp.status()
                    )));
                }
                Err(err) => {
                    set_error.set(Some(format!("Ошибка сети: {}", err)));
                }
            }

            set_extracting.set(false);
        });
    });

    let favorite_target_id = stored_id.get_value();
    let favorite_tab_key = format!("a027_wb_documents_details_{}", stored_id.get_value());
    let favorite_title = Signal::derive(move || {
        doc.get()
            .map(|d| {
                format!(
                    "WB Document {}",
                    effective_document_date(d.report_period_to.as_ref(), &d.creation_time)
                )
            })
            .unwrap_or_else(|| "WB Document".to_string())
    });

    view! {
        <PageFrame page_id="a027_wb_documents--detail" category=PAGE_CAT_DETAIL>
            <div class="page__header">
                <div class="page__header-left">
                    <FavoriteButton
                        target_kind="a027_wb_documents_details".to_string()
                        target_id=favorite_target_id
                        target_title=favorite_title
                        tab_key=favorite_tab_key
                    />
                    <h1 class="page__title">
                        {move || {
                            doc.get()
                                .map(|d| {
                                    let primary_date =
                                        effective_document_date(d.report_period_to.as_ref(), &d.creation_time);
                                    format!("WB Document {}", primary_date)
                                })
                                .unwrap_or_else(|| "WB Document".to_string())
                        }}
                    </h1>
                    <Show when=move || doc.get().is_some()>
                        {move || {
                            if let Some(d) = doc.get() {
                                let viewed = d.viewed;
                                view! {
                                    <Flex gap=FlexGap::Small>
                                        <Badge appearance=BadgeAppearance::Tint color=if viewed { BadgeColor::Success } else { BadgeColor::Warning }>
                                            {if viewed { "Просмотрен" } else { "Не просмотрен" }}
                                        </Badge>
                                        <Badge appearance=BadgeAppearance::Outline>
                                            {if d.is_weekly_report { "Еженедельный отчет" } else { "Прочий документ" }}
                                        </Badge>
                                    </Flex>
                                }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }
                        }}
                    </Show>
                </div>
                <div class="page__header-right">
                    <Button appearance=ButtonAppearance::Secondary size=ButtonSize::Medium on_click=move |_| on_close.run(())>
                        "Закрыть"
                    </Button>
                </div>
            </div>

            <DocumentTabBar selected_tab=selected_tab />

            <div class="page__content">
                {move || if loading.get() {
                    view! {
                        <Flex gap=FlexGap::Small style="align-items:center;justify-content:center;padding:var(--spacing-4xl);">
                            <Spinner />
                            <span>"Загрузка..."</span>
                        </Flex>
                    }.into_any()
                } else if let Some(err) = error.get() {
                    view! { <div class="alert alert--error">{err}</div> }.into_any()
                } else if let Some(d) = doc.get() {
                    let primary_date = effective_document_date(d.report_period_to.as_ref(), &d.creation_time);
                    let period_text = match (d.report_period_from.clone(), d.report_period_to.clone()) {
                        (Some(from), Some(to)) => format!("{} - {}", from, to),
                        (Some(from), None) => from,
                        (None, Some(to)) => to,
                        (None, None) => "—".to_string(),
                    };

                    view! {
                        <div style="display:flex;flex-direction:column;gap:var(--spacing-md);">
                            {move || success.get().map(|msg| view! { <div class="alert alert--success">{msg}</div> })}
                            {move || match selected_tab.get().as_str() {
                                "check" => view! {
                                    <CheckTab
                                        saving=saving
                                        posting=posting
                                        extracting=extracting
                                        on_save=on_save
                                        on_post=on_post
                                        on_extract_weekly_report=on_extract_weekly_report
                                        is_weekly_report=is_weekly_report
                                        report_period_from=report_period_from
                                        report_period_to=report_period_to
                                        realized_goods_total=realized_goods_total
                                        wb_reward_with_vat=wb_reward_with_vat
                                        seller_transfer_total=seller_transfer_total
                                        other_deductions=other_deductions
                                        logistics=logistics
                                        acquiring=acquiring
                                        comment=comment
                                        reconciliation=d.reconciliation.clone()
                                        fact_reconciliation=d.fact_reconciliation.clone()
                                        max_deviation=d.max_deviation
                                        document_label=primary_date.clone()
                                    />
                                }.into_any(),
                                "meta" => view! { <MetaTab doc=d.clone() /> }.into_any(),
                                _ => view! {
                                    <GeneralTab
                                        doc=d.clone()
                                        primary_date=primary_date.clone()
                                        period_text=period_text.clone()
                                        start_download=start_download
                                        set_error=set_error
                                    />
                                }.into_any(),
                            }}
                        </div>
                    }.into_any()
                } else {
                    view! { <div class="alert">"Документ не найден."</div> }.into_any()
                }}
            </div>
        </PageFrame>
    }
}

#[component]
fn GeneralTab(
    doc: DetailsDto,
    primary_date: String,
    period_text: String,
    start_download: Callback<(String, String, String)>,
    set_error: WriteSignal<Option<String>>,
) -> impl IntoView {
    let doc_for_files = doc.clone();
    let document_id = doc.id.clone();

    view! {
        <div class="detail-grid">
            <div class="detail-grid__col">
                <CardAnimated delay_ms=0 nav_id="a027_wb_documents_details_general_main">
                    <h4 class="details-section__title">"Основные поля"</h4>
                    <div style="display:grid;grid-template-columns:1fr 1fr;gap:var(--spacing-sm);">
                        <ReadField label="Дата документа" value=primary_date />
                        <ReadField label="Период отчета" value=period_text />
                    </div>
                    <div style="display:grid;grid-template-columns:1fr 1fr;gap:var(--spacing-sm);">
                        <ReadField label="Категория" value=doc.category.clone() />
                        <ReadField label="Category ID" value=doc.name.clone() />
                    </div>
                    <ReadField label="Service Name" value=doc.service_name.clone() />
                </CardAnimated>

                <CardAnimated delay_ms=80 nav_id="a027_wb_documents_details_general_refs">
                    <h4 class="details-section__title">"Связи"</h4>
                    <ReadField label="Кабинет" value=doc.connection_name.clone().unwrap_or(doc.connection_id.clone()) />
                    <ReadField label="Организация" value=doc.organization_name.clone().unwrap_or(doc.organization_id.clone()) />
                    <ReadField label="Маркетплейс" value=doc.marketplace_name.clone().unwrap_or(doc.marketplace_id.clone()) />
                </CardAnimated>
            </div>

            <div class="detail-grid__col">
                <CardAnimated delay_ms=40 nav_id="a027_wb_documents_details_general_files">
                    <h4 class="details-section__title">"Файлы"</h4>
                    <div style="display:flex;gap:8px;flex-wrap:wrap;">
                        <For
                            each=move || doc_for_files.extensions.clone()
                            key=|ext| ext.clone()
                            children=move |ext| {
                                let document_id = document_id.clone();
                                let service_name = doc_for_files.service_name.clone();
                                let extension = ext.clone();
                                view! {
                                    <Button
                                        appearance=ButtonAppearance::Primary
                                        size=ButtonSize::Small
                                        on_click=move |_| {
                                            set_error.set(None);
                                            start_download.run((
                                                document_id.clone(),
                                                service_name.clone(),
                                                extension.clone(),
                                            ));
                                        }
                                    >
                                        {icon("download")}
                                        <span style="margin-left:6px;">{ext}</span>
                                    </Button>
                                }
                            }
                        />
                    </div>
                </CardAnimated>

                <CardAnimated delay_ms=120 nav_id="a027_wb_documents_details_general_totals">
                    <h4 class="details-section__title">"Итоги проверки"</h4>
                    <div style="display:grid;grid-template-columns:1fr 1fr;gap:var(--spacing-sm);">
                        <ReadField label="Реализовано" value=fmt_optional_amount(doc.realized_goods_total) />
                        <ReadField label="Вознаграждение WB" value=fmt_optional_amount(doc.wb_reward_with_vat) />
                    </div>
                    <ReadField label="К перечислению" value=fmt_optional_amount(doc.seller_transfer_total) />
                </CardAnimated>
            </div>
        </div>
    }
}

#[component]
fn CheckTab(
    saving: ReadSignal<bool>,
    posting: ReadSignal<bool>,
    extracting: ReadSignal<bool>,
    on_save: Callback<()>,
    on_post: Callback<()>,
    on_extract_weekly_report: Callback<()>,
    is_weekly_report: RwSignal<bool>,
    report_period_from: RwSignal<String>,
    report_period_to: RwSignal<String>,
    realized_goods_total: RwSignal<String>,
    wb_reward_with_vat: RwSignal<String>,
    seller_transfer_total: RwSignal<String>,
    other_deductions: RwSignal<String>,
    logistics: RwSignal<String>,
    acquiring: RwSignal<String>,
    comment: RwSignal<String>,
    reconciliation: ReconciliationDto,
    fact_reconciliation: ReconciliationDto,
    max_deviation: Option<f64>,
    document_label: String,
) -> impl IntoView {
    let max_deviation_display = max_deviation
        .map(format_amount)
        .unwrap_or_else(|| "—".to_string());

    let export_filename = format!(
        "wb_reconciliation_{}.csv",
        document_label.replace([' ', '.', ':'], "_")
    );
    let export_reconciliation = {
        let reconciliation = reconciliation.clone();
        move || {
            let rows = vec![
                check_export_row(
                    "Итого стоимость реализованного товара (1.1)",
                    realized_goods_total.get_untracked(),
                    &reconciliation.realized_goods_total,
                ),
                check_export_row(
                    "Сумма вознаграждения WB (2.1 + 2.2)",
                    wb_reward_with_vat.get_untracked(),
                    &reconciliation.wb_reward_with_vat,
                ),
                check_export_row(
                    "Прочие удержания / реклама (2.10)",
                    other_deductions.get_untracked(),
                    &reconciliation.advert_other_deductions,
                ),
                check_export_row(
                    "Логистика (2.7 + 2.8)",
                    logistics.get_untracked(),
                    &reconciliation.logistics,
                ),
                check_export_row(
                    "Эквайринг (2.6)",
                    acquiring.get_untracked(),
                    &reconciliation.acquiring,
                ),
                check_export_row(
                    "Итого к перечислению продавцу (8)",
                    seller_transfer_total.get_untracked(),
                    &reconciliation.seller_transfer_total,
                ),
            ];
            let _ = export_to_excel(&rows, &export_filename);
        }
    };
    view! {
        <CardAnimated delay_ms=0 nav_id="a027_wb_documents_details_check">
            <div style="display:flex;justify-content:space-between;align-items:center;gap:12px;flex-wrap:wrap;margin-bottom:var(--spacing-md);">
                <h4 class="details-section__title" style="margin:0;">"Проверка еженедельного отчета"</h4>
                <Flex gap=FlexGap::Small align=FlexAlign::Center>
                    <span style="font-size:13px;color:var(--color-text-secondary);">"Макс. отклонение:"</span>
                    <span style="font-variant-numeric:tabular-nums;font-weight:600;">
                        {max_deviation_display}
                    </span>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        size=ButtonSize::Small
                        on_click=move |_| export_reconciliation()
                    >
                        {icon("download")}
                        "Excel (csv)"
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        size=ButtonSize::Small
                        on_click=move |_| on_extract_weekly_report.run(())
                        disabled=Signal::derive(move || saving.get() || posting.get() || extracting.get())
                    >
                        {move || if extracting.get() { "Извлечение..." } else { "Заполнить из PDF" }}
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        size=ButtonSize::Small
                        on_click=move |_| on_save.run(())
                        disabled=Signal::derive(move || saving.get() || posting.get() || extracting.get())
                    >
                        {move || if saving.get() { "Сохранение..." } else { "Сохранить" }}
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Primary
                        size=ButtonSize::Small
                        on_click=move |_| on_post.run(())
                        disabled=Signal::derive(move || saving.get() || posting.get() || extracting.get())
                    >
                        {move || if posting.get() { "Проведение..." } else { "Провести" }}
                    </Button>
                </Flex>
            </div>

            <div style="display:grid;grid-template-columns:minmax(220px,260px) 160px 160px;gap:var(--spacing-sm);align-items:end;max-width:640px;">
                <div class="form__group">
                    <label class="form__label">"Тип документа"</label>
                    <div style="min-height:32px;display:flex;align-items:center;">
                        <Checkbox checked=is_weekly_report label="Еженедельный отчет" />
                    </div>
                </div>
                <EditField label="Период с" value=report_period_from input_type="date".to_string() />
                <EditField label="Период по" value=report_period_to input_type="date".to_string() />
            </div>

            <div class="table-wrapper" style="margin-top:var(--spacing-lg);">
                <Table attr:style="width:100%;min-width:1120px;">
                    <TableHeader>
                        <TableRow>
                            <TableHeaderCell>"Показатель"</TableHeaderCell>
                            <TableHeaderCell attr:style="width:500px;">"Формула оборотов"</TableHeaderCell>
                            <TableHeaderCell attr:style="width:150px;text-align:right;">"Отчет WB"</TableHeaderCell>
                            <TableHeaderCell attr:style="width:150px;text-align:right;">"Данные в базе"</TableHeaderCell>
                            <TableHeaderCell attr:style="width:180px;text-align:right;">"Расхождение"</TableHeaderCell>
                        </TableRow>
                    </TableHeader>
                    <TableBody>
                        <CheckInputRow
                            label="Итого стоимость реализованного товара (1.1)"
                            value=realized_goods_total
                            line=reconciliation.realized_goods_total.clone()
                        />
                        <CheckInputRow
                            label="Сумма вознаграждения WB (2.1 + 2.2)"
                            value=wb_reward_with_vat
                            line=reconciliation.wb_reward_with_vat.clone()
                        />
                        <CheckInputRow
                            label="Прочие удержания / реклама (2.10)"
                            value=other_deductions
                            line=reconciliation.advert_other_deductions.clone()
                        />
                        <CheckInputRow
                            label="Логистика (2.7 + 2.8)"
                            value=logistics
                            line=reconciliation.logistics.clone()
                        />
                        <CheckInputRow
                            label="Эквайринг (2.6)"
                            value=acquiring
                            line=reconciliation.acquiring.clone()
                        />
                        <CheckInputRow
                            label="Итого к перечислению продавцу (8)"
                            value=seller_transfer_total
                            line=reconciliation.seller_transfer_total.clone()
                            emphasis=true
                        />
                    </TableBody>
                </Table>
            </div>

            <FactCheckTable
                realized_goods_total=realized_goods_total
                wb_reward_with_vat=wb_reward_with_vat
                seller_transfer_total=seller_transfer_total
                other_deductions=other_deductions
                logistics=logistics
                acquiring=acquiring
                fact_reconciliation=fact_reconciliation
            />

            <div class="form__group" style="margin-top:var(--spacing-lg);max-width:640px;">
                <label class="form__label">"Комментарий"</label>
                <textarea
                    class="form__input"
                    rows="3"
                    style="resize:vertical;min-height:64px;"
                    placeholder="Комментарий к проверке (сохраняется кнопками «Сохранить» / «Провести»)"
                    prop:value=move || comment.get()
                    on:input=move |ev| comment.set(event_target_value(&ev))
                ></textarea>
            </div>
        </CardAnimated>
    }
}

#[component]
fn AmountTableInput(
    #[prop(into)] value: RwSignal<String>,
    #[prop(optional)] emphasis: bool,
) -> impl IntoView {
    let font_weight = if emphasis { "600" } else { "400" };
    view! {
        <input
            class="form__input"
            type="text"
            inputmode="decimal"
            prop:value=move || value.get()
            placeholder="0.00"
            style=format!("height:28px;min-height:28px;width:110px;max-width:110px;padding:2px 6px;text-align:right;font-variant-numeric:tabular-nums;font-weight:{};", font_weight)
            on:input=move |ev| value.set(event_target_value(&ev))
        />
    }
}

#[component]
fn CheckInputRow(
    label: &'static str,
    #[prop(into)] value: RwSignal<String>,
    line: ReconciliationLineDto,
    #[prop(optional)] emphasis: bool,
) -> impl IntoView {
    let database_value = if line.is_available {
        fmt_optional_amount(line.database_value)
    } else {
        "?".to_string()
    };
    let difference = fmt_reconciliation_difference(&line);
    view! {
        <TableRow>
            <TableCell><TableCellLayout>{label}</TableCellLayout></TableCell>
            <TableCell>
                <TableCellLayout attr:style="display:block;width:100%;font-size:12px;color:var(--color-text-secondary);white-space:normal;line-height:1.3;">
                    {line.formula}
                </TableCellLayout>
            </TableCell>
            <TableCell>
                <TableCellLayout attr:style="display:block;width:100%;text-align:right;">
                    <AmountTableInput value=value emphasis=emphasis />
                </TableCellLayout>
            </TableCell>
            <TableCell><TableCellLayout attr:style="display:block;width:100%;text-align:right;font-variant-numeric:tabular-nums;">{database_value}</TableCellLayout></TableCell>
            <TableCell><TableCellLayout attr:style="display:block;width:100%;text-align:right;font-variant-numeric:tabular-nums;">{difference}</TableCellLayout></TableCell>
        </TableRow>
    }
}

/// Вторая таблица сверки — по слою FACT. Полностью независимый код от старой
/// таблицы. Значения «Отчет WB» переиспользуют те же сигналы (только чтение),
/// «Данные в базе» и «Расхождение» приходят из fact-сверки бэкенда.
#[component]
fn FactCheckTable(
    realized_goods_total: RwSignal<String>,
    wb_reward_with_vat: RwSignal<String>,
    seller_transfer_total: RwSignal<String>,
    other_deductions: RwSignal<String>,
    logistics: RwSignal<String>,
    acquiring: RwSignal<String>,
    fact_reconciliation: ReconciliationDto,
) -> impl IntoView {
    view! {
        <CardAnimated delay_ms=40 nav_id="a027_wb_documents_details_check_fact">
            <div style="display:flex;justify-content:space-between;align-items:center;gap:12px;flex-wrap:wrap;margin-bottom:var(--spacing-md);margin-top:var(--spacing-lg);">
                <h4 class="details-section__title" style="margin:0;">"Проверка по слою fact"</h4>
                <span style="font-size:12px;color:var(--color-text-secondary);max-width:560px;text-align:right;">
                    "Слой fact формируется из p903 (исходные данные отчёта WB). Логистика и эквайринг повторяют состав основной таблицы; рекламы на fact нет."
                </span>
            </div>

            <div class="table-wrapper">
                <Table attr:style="width:100%;min-width:1120px;">
                    <TableHeader>
                        <TableRow>
                            <TableHeaderCell>"Показатель"</TableHeaderCell>
                            <TableHeaderCell attr:style="width:500px;">"Формула оборотов (fact)"</TableHeaderCell>
                            <TableHeaderCell attr:style="width:150px;text-align:right;">"Отчет WB"</TableHeaderCell>
                            <TableHeaderCell attr:style="width:150px;text-align:right;">"Данные в базе (fact)"</TableHeaderCell>
                            <TableHeaderCell attr:style="width:180px;text-align:right;">"Расхождение"</TableHeaderCell>
                        </TableRow>
                    </TableHeader>
                    <TableBody>
                        <FactCheckRow
                            label="Итого стоимость реализованного товара (1.1)"
                            value=realized_goods_total
                            line=fact_reconciliation.realized_goods_total.clone()
                        />
                        <FactCheckRow
                            label="Сумма вознаграждения WB (2.1 + 2.2)"
                            value=wb_reward_with_vat
                            line=fact_reconciliation.wb_reward_with_vat.clone()
                        />
                        <FactCheckRow
                            label="Прочие удержания / реклама (2.10)"
                            value=other_deductions
                            line=fact_reconciliation.advert_other_deductions.clone()
                        />
                        <FactCheckRow
                            label="Логистика (2.7 + 2.8)"
                            value=logistics
                            line=fact_reconciliation.logistics.clone()
                        />
                        <FactCheckRow
                            label="Эквайринг (2.6)"
                            value=acquiring
                            line=fact_reconciliation.acquiring.clone()
                        />
                        <FactCheckRow
                            label="Итого к перечислению продавцу (8)"
                            value=seller_transfer_total
                            line=fact_reconciliation.seller_transfer_total.clone()
                            emphasis=true
                        />
                    </TableBody>
                </Table>
            </div>
        </CardAnimated>
    }
}

#[component]
fn FactCheckRow(
    label: &'static str,
    #[prop(into)] value: RwSignal<String>,
    line: ReconciliationLineDto,
    #[prop(optional)] emphasis: bool,
) -> impl IntoView {
    let database_value = if line.is_available {
        fmt_optional_amount(line.database_value)
    } else {
        "?".to_string()
    };
    let difference = fmt_reconciliation_difference(&line);
    let report_weight = if emphasis { "600" } else { "400" };
    view! {
        <TableRow>
            <TableCell><TableCellLayout>{label}</TableCellLayout></TableCell>
            <TableCell>
                <TableCellLayout attr:style="display:block;width:100%;font-size:12px;color:var(--color-text-secondary);white-space:normal;line-height:1.3;">
                    {line.formula}
                </TableCellLayout>
            </TableCell>
            <TableCell>
                <TableCellLayout attr:style=format!("display:block;width:100%;text-align:right;font-variant-numeric:tabular-nums;font-weight:{};", report_weight)>
                    {move || value.get()}
                </TableCellLayout>
            </TableCell>
            <TableCell><TableCellLayout attr:style="display:block;width:100%;text-align:right;font-variant-numeric:tabular-nums;">{database_value}</TableCellLayout></TableCell>
            <TableCell><TableCellLayout attr:style="display:block;width:100%;text-align:right;font-variant-numeric:tabular-nums;">{difference}</TableCellLayout></TableCell>
        </TableRow>
    }
}

#[component]
fn MetaTab(doc: DetailsDto) -> impl IntoView {
    view! {
        <div class="detail-grid">
            <div class="detail-grid__col">
                <CardAnimated delay_ms=0 nav_id="a027_wb_documents_details_meta_ids">
                    <h4 class="details-section__title">"Технические поля"</h4>
                    <ReadField label="ID" value=doc.id.clone() />
                    <ReadField label="Service Name" value=doc.service_name.clone() />
                    <ReadField label="Connection ID" value=doc.connection_id.clone() />
                    <ReadField label="Organization ID" value=doc.organization_id.clone() />
                    <ReadField label="Marketplace ID" value=doc.marketplace_id.clone() />
                    <ReadField label="Locale" value=doc.locale.clone() />
                </CardAnimated>
            </div>
            <div class="detail-grid__col">
                <CardAnimated delay_ms=40 nav_id="a027_wb_documents_details_meta_dates">
                    <h4 class="details-section__title">"Системные даты"</h4>
                    <ReadField label="Создан в WB" value=fmt_dt(&doc.creation_time) />
                    <ReadField label="Загружено" value=fmt_dt(&doc.fetched_at) />
                    <ReadField label="Создано в БД" value=fmt_dt(&doc.created_at) />
                    <ReadField label="Обновлено в БД" value=fmt_dt(&doc.updated_at) />
                </CardAnimated>
            </div>
        </div>
    }
}
