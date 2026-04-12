use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::auth_download::download_authenticated_file;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_DETAIL;
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

fn fmt_optional_amount(value: Option<f64>) -> String {
    value
        .map(|amount| format!("{:.2}", amount))
        .unwrap_or_else(|| "—".to_string())
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
    fetched_at: String,
    locale: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct UpdateManualFieldsRequest {
    is_weekly_report: bool,
    report_period_from: Option<String>,
    report_period_to: Option<String>,
    realized_goods_total: Option<f64>,
    wb_reward_with_vat: Option<f64>,
    seller_transfer_total: Option<f64>,
}

#[component]
fn ReadField(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="form__group" style="margin-bottom: 10px;">
            <label class="form__label">{label}</label>
            <Input value=RwSignal::new(value) attr:readonly=true />
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
        <div class="form__group" style="margin-bottom: 10px;">
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
pub fn WbDocumentsDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let tabs = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let stored_id = StoredValue::new(id.clone());
    let (loading, set_loading) = signal(true);
    let (saving, set_saving) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (success, set_success) = signal::<Option<String>>(None);
    let (doc, set_doc) = signal::<Option<DetailsDto>>(None);
    let selected_tab = RwSignal::new("main".to_string());

    let is_weekly_report = RwSignal::new(false);
    let report_period_from = RwSignal::new(String::new());
    let report_period_to = RwSignal::new(String::new());
    let realized_goods_total = RwSignal::new(String::new());
    let wb_reward_with_vat = RwSignal::new(String::new());
    let seller_transfer_total = RwSignal::new(String::new());

    let apply_doc_to_form = move |data: &DetailsDto| {
        is_weekly_report.set(data.is_weekly_report);
        report_period_from.set(data.report_period_from.clone().unwrap_or_default());
        report_period_to.set(data.report_period_to.clone().unwrap_or_default());
        realized_goods_total.set(
            data.realized_goods_total
                .map(|v| format!("{:.2}", v))
                .unwrap_or_default(),
        );
        wb_reward_with_vat.set(
            data.wb_reward_with_vat
                .map(|v| format!("{:.2}", v))
                .unwrap_or_default(),
        );
        seller_transfer_total.set(
            data.seller_transfer_total
                .map(|v| format!("{:.2}", v))
                .unwrap_or_default(),
        );
    };

    let load_doc = {
        let tabs = tabs.clone();
        let stored_id = stored_id;
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
            let set_error = set_error;
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

    let on_save = move |_| {
        let Some(current_doc) = doc.get() else {
            return;
        };

        let request = UpdateManualFieldsRequest {
            is_weekly_report: is_weekly_report.get(),
            report_period_from: {
                let value = report_period_from.get();
                if value.trim().is_empty() {
                    None
                } else {
                    Some(value)
                }
            },
            report_period_to: {
                let value = report_period_to.get();
                if value.trim().is_empty() {
                    None
                } else {
                    Some(value)
                }
            },
            realized_goods_total: parse_optional_amount(realized_goods_total.get()),
            wb_reward_with_vat: parse_optional_amount(wb_reward_with_vat.get()),
            seller_transfer_total: parse_optional_amount(seller_transfer_total.get()),
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
    };

    view! {
        <PageFrame page_id="a027_wb_documents--detail" category=PAGE_CAT_DETAIL>
            <div class="page__header">
                <div class="page__header-left">
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
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| on_close.run(())>
                        "Закрыть"
                    </Button>
                </div>
            </div>

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
                    let download_id = d.id.clone();
                    let primary_date = effective_document_date(d.report_period_to.as_ref(), &d.creation_time);
                    let period_text = match (d.report_period_from.clone(), d.report_period_to.clone()) {
                        (Some(from), Some(to)) => format!("{} - {}", from, to),
                        (Some(from), None) => from,
                        (None, Some(to)) => to,
                        (None, None) => "—".to_string(),
                    };
                    let summary_primary_date = primary_date.clone();
                    let summary_period_text = period_text.clone();
                    let summary_service_name = d.service_name.clone();
                    let summary_category = d.category.clone();

                    view! {
                        <div style="display:flex;flex-direction:column;gap:16px;">
                            {move || success.get().map(|msg| view! { <div class="alert alert--success">{msg}</div> })}

                            <Card>
                                <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(220px,1fr));gap:14px;">
                                    <div>
                                        <div style="font-size:12px;color:var(--color-text-secondary);">"Дата документа"</div>
                                        <div style="font-size:20px;font-weight:700;">{summary_primary_date}</div>
                                    </div>
                                    <div>
                                        <div style="font-size:12px;color:var(--color-text-secondary);">"Период отчета"</div>
                                        <div style="font-size:16px;font-weight:600;">{summary_period_text}</div>
                                    </div>
                                    <div>
                                        <div style="font-size:12px;color:var(--color-text-secondary);">"Service Name"</div>
                                        <div style="font-size:14px;font-weight:600;word-break:break-word;">{summary_service_name}</div>
                                    </div>
                                    <div>
                                        <div style="font-size:12px;color:var(--color-text-secondary);">"Категория"</div>
                                        <div style="font-size:14px;font-weight:600;">{summary_category}</div>
                                    </div>
                                </div>
                            </Card>

                            <TabList selected_value=selected_tab>
                                <Tab value="main".to_string()>"Основное"</Tab>
                                <Tab value="meta".to_string()>"Дополнительно"</Tab>
                            </TabList>

                            {move || {
                                let d = d.clone();
                                let primary_date = primary_date.clone();
                                let period_text = period_text.clone();
                                let download_id = download_id.clone();
                                if selected_tab.get() == "main" {
                                view! {
                                    <div style="display:flex;flex-direction:column;gap:16px;">
                                        <Card>
                                            <div style="display:flex;justify-content:space-between;align-items:center;gap:12px;flex-wrap:wrap;">
                                                <div style="display:flex;flex-direction:column;gap:4px;">
                                                    <div style="font-weight:700;">"Поля проверки еженедельного отчета"</div>
                                                    <div style="font-size:12px;color:var(--color-text-secondary);">
                                                        "Заполняются вручную для сверки итогов по weekly report"
                                                    </div>
                                                </div>
                                                <Button
                                                    appearance=ButtonAppearance::Primary
                                                    on_click=on_save
                                                    disabled=Signal::derive(move || saving.get())
                                                >
                                                    {move || if saving.get() { "Сохранение..." } else { "Сохранить" }}
                                                </Button>
                                            </div>

                                            <div style="display:flex;flex-direction:column;gap:18px;margin-top:16px;">
                                                <div style="display:flex;flex-direction:column;gap:10px;">
                                                    <div style="font-size:13px;font-weight:700;">"Тип и период"</div>
                                                    <div style="display:grid;grid-template-columns:minmax(220px,260px) 140px 140px;gap:12px;align-items:end;max-width:580px;">
                                                        <div class="form__group" style="margin-bottom:0;">
                                                            <label class="form__label">"Тип документа"</label>
                                                            <div style="min-height:38px;display:flex;align-items:center;">
                                                                <Checkbox checked=is_weekly_report label="Еженедельный отчет" />
                                                            </div>
                                                        </div>
                                                        <EditField
                                                            label="Период с"
                                                            value=report_period_from
                                                            input_type="date".to_string()
                                                        />
                                                        <EditField
                                                            label="Период по"
                                                            value=report_period_to
                                                            input_type="date".to_string()
                                                        />
                                                    </div>
                                                </div>

                                                <div style="display:flex;flex-direction:column;gap:10px;">
                                                    <div style="display:flex;justify-content:space-between;align-items:baseline;gap:12px;flex-wrap:wrap;">
                                                        <div style="font-size:13px;font-weight:700;">"Суммы для сверки"</div>
                                                        <div style="font-size:12px;color:var(--color-text-secondary);">
                                                            "Итоговые значения из подтвержденного weekly report"
                                                        </div>
                                                    </div>

                                                    <div style="display:flex;flex-direction:column;gap:10px;max-width:920px;">
                                                        <div style="display:grid;grid-template-columns:minmax(320px,1fr) 220px;gap:16px;align-items:center;">
                                                            <div style="font-size:13px;">"Итого стоимость реализованного товара"</div>
                                                            <input
                                                                class="form__input"
                                                                type="text"
                                                                inputmode="decimal"
                                                                prop:value=move || realized_goods_total.get()
                                                                placeholder="0.00"
                                                                style="text-align:right;font-variant-numeric:tabular-nums;"
                                                                on:input=move |ev| realized_goods_total.set(event_target_value(&ev))
                                                            />
                                                        </div>

                                                        <div style="display:grid;grid-template-columns:minmax(320px,1fr) 220px;gap:16px;align-items:center;">
                                                            <div style="font-size:13px;">"Сумма вознаграждения Вайлдберриз (ВВ c YLC)"</div>
                                                            <input
                                                                class="form__input"
                                                                type="text"
                                                                inputmode="decimal"
                                                                prop:value=move || wb_reward_with_vat.get()
                                                                placeholder="0.00"
                                                                style="text-align:right;font-variant-numeric:tabular-nums;"
                                                                on:input=move |ev| wb_reward_with_vat.set(event_target_value(&ev))
                                                            />
                                                        </div>

                                                        <div style="display:grid;grid-template-columns:minmax(320px,1fr) 220px;gap:16px;align-items:center;">
                                                            <div style="font-size:13px;font-weight:600;">"Итого к перечислению Продавцу"</div>
                                                            <input
                                                                class="form__input"
                                                                type="text"
                                                                inputmode="decimal"
                                                                prop:value=move || seller_transfer_total.get()
                                                                placeholder="0.00"
                                                                style="text-align:right;font-variant-numeric:tabular-nums;font-weight:600;"
                                                                on:input=move |ev| seller_transfer_total.set(event_target_value(&ev))
                                                            />
                                                        </div>
                                                    </div>
                                                </div>
                                            </div>
                                        </Card>

                                        <div class="detail-grid">
                                            <div class="detail-grid__col">
                                                <Card>
                                                    <div style="font-weight:700;margin-bottom:12px;">"Основные поля"</div>
                                                    <ReadField label="Дата документа" value=primary_date />
                                                    <ReadField label="Период отчета" value=period_text />
                                                    <ReadField label="Категория" value=d.category.clone() />
                                                    <ReadField label="Category ID" value=d.name.clone() />
                                                    <ReadField label="Кабинет" value=d.connection_name.clone().unwrap_or(d.connection_id.clone()) />
                                                    <ReadField label="Организация" value=d.organization_name.clone().unwrap_or(d.organization_id.clone()) />
                                                    <ReadField label="Маркетплейс" value=d.marketplace_name.clone().unwrap_or(d.marketplace_id.clone()) />
                                                </Card>
                                            </div>
                                            <div class="detail-grid__col">
                                                <Card>
                                                    <div style="font-weight:700;margin-bottom:12px;">"Скачать документ"</div>
                                                    <div style="display:flex;gap:8px;flex-wrap:wrap;">
                                                        <For
                                                            each=move || d.extensions.clone()
                                                            key=|ext| ext.clone()
                                                            children=move |ext| {
                                                                let download_id_value = download_id.clone();
                                                                let extension = ext.clone();
                                                                let service_name = d.service_name.clone();
                                                                view! {
                                                                    <Button
                                                                        appearance=ButtonAppearance::Primary
                                                                        on_click=move |_| {
                                                                            set_error.set(None);
                                                                            start_download.run((
                                                                                download_id_value.clone(),
                                                                                service_name.clone(),
                                                                                extension.clone(),
                                                                            ));
                                                                        }
                                                                    >
                                                                        <span>{icon("download")}</span>
                                                                        <span>{format!(" {}", ext)}</span>
                                                                    </Button>
                                                                }
                                                            }
                                                        />
                                                    </div>

                                                    <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:10px;margin-top:16px;">
                                                        <div>
                                                            <div style="font-size:12px;color:var(--color-text-secondary);">"Реализовано"</div>
                                                            <div style="font-size:16px;font-weight:600;">{fmt_optional_amount(d.realized_goods_total)}</div>
                                                        </div>
                                                        <div>
                                                            <div style="font-size:12px;color:var(--color-text-secondary);">"Вознаграждение WB"</div>
                                                            <div style="font-size:16px;font-weight:600;">{fmt_optional_amount(d.wb_reward_with_vat)}</div>
                                                        </div>
                                                        <div>
                                                            <div style="font-size:12px;color:var(--color-text-secondary);">"К перечислению"</div>
                                                            <div style="font-size:16px;font-weight:600;">{fmt_optional_amount(d.seller_transfer_total)}</div>
                                                        </div>
                                                    </div>
                                                </Card>
                                            </div>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="detail-grid">
                                        <div class="detail-grid__col">
                                            <Card>
                                                <div style="font-weight:700;margin-bottom:12px;">"Технические поля"</div>
                                                <ReadField label="ID" value=d.id.clone() />
                                                <ReadField label="Service Name" value=d.service_name.clone() />
                                                <ReadField label="Connection ID" value=d.connection_id.clone() />
                                                <ReadField label="Organization ID" value=d.organization_id.clone() />
                                                <ReadField label="Marketplace ID" value=d.marketplace_id.clone() />
                                                <ReadField label="Locale" value=d.locale.clone() />
                                            </Card>
                                        </div>
                                        <div class="detail-grid__col">
                                            <Card>
                                                <div style="font-weight:700;margin-bottom:12px;">"Системные даты"</div>
                                                <ReadField label="Создан в WB" value=fmt_dt(&d.creation_time) />
                                                <ReadField label="Загружено" value=fmt_dt(&d.fetched_at) />
                                                <ReadField label="Создано в БД" value=fmt_dt(&d.created_at) />
                                                <ReadField label="Обновлено в БД" value=fmt_dt(&d.updated_at) />
                                            </Card>
                                        </div>
                                    </div>
                                }.into_any()
                            }}}
                        </div>
                    }.into_any()
                } else {
                    view! { <div class="alert">"Документ не найден."</div> }.into_any()
                }}
            </div>
        </PageFrame>
    }
}
