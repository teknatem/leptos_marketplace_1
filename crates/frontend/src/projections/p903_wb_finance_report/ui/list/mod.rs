mod state;

use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::icons::icon;
use crate::shared::list_utils::{format_number, get_sort_class, get_sort_indicator};
use leptos::logging::log;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use state::{create_state, persist_state};
use thaw::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

#[component]
fn P903Header(
    #[prop(into)] total_count: Signal<usize>,
    #[prop(into)] is_loading: Signal<bool>,
    on_refresh: Callback<()>,
    on_export: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="page-header">
            <div class="page-header__content">
                <div class="page-header__icon">{vec![icon("dollar-sign").into_view()]}</div>
                <div class="page-header__text">
                    <h1 class="page-header__title">"WB Finance Report (P903)"</h1>
                    <div class="page-header__badge">
                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                            <span>{move || total_count.get().to_string()}</span>
                        </Badge>
                    </div>
                </div>
            </div>

            <div class="page-header__actions">
                <Button
                    appearance=ButtonAppearance::Primary
                    on_click=move |_| on_export.run(())
                    disabled=move || total_count.get() == 0
                >
                    <span>{vec![icon("download").into_view()]}</span>
                    <span>" Export Excel"</span>
                </Button>
                <Button
                    appearance=ButtonAppearance::Secondary
                    on_click=move |_| on_refresh.run(())
                    disabled=is_loading
                >
                    <span>{vec![icon("refresh").into_view()]}</span>
                    <span>{move || if is_loading.get() { " Загрузка..." } else { " Обновить" }}</span>
                </Button>
            </div>
        </div>
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbFinanceReportDto {
    pub rr_dt: String,
    pub rrd_id: i64,
    pub connection_mp_ref: String,
    pub organization_ref: String,
    pub acquiring_fee: Option<f64>,
    pub acquiring_percent: Option<f64>,
    pub additional_payment: Option<f64>,
    pub bonus_type_name: Option<String>,
    pub commission_percent: Option<f64>,
    pub delivery_amount: Option<f64>,
    pub delivery_rub: Option<f64>,
    pub nm_id: Option<i64>,
    pub penalty: Option<f64>,
    pub ppvz_vw: Option<f64>,
    pub ppvz_vw_nds: Option<f64>,
    pub ppvz_sales_commission: Option<f64>,
    pub quantity: Option<i32>,
    pub rebill_logistic_cost: Option<f64>,
    pub retail_amount: Option<f64>,
    pub retail_price: Option<f64>,
    pub retail_price_withdisc_rub: Option<f64>,
    pub return_amount: Option<f64>,
    pub sa_name: Option<String>,
    pub storage_fee: Option<f64>,
    pub subject_name: Option<String>,
    pub supplier_oper_name: Option<String>,
    pub cashback_amount: Option<f64>,
    pub ppvz_for_pay: Option<f64>,
    pub ppvz_kvw_prc: Option<f64>,
    pub ppvz_kvw_prc_base: Option<f64>,
    pub srv_dbs: Option<i32>,
    pub srid: Option<String>,
    pub loaded_at_utc: String,
    pub payload_version: i32,
    pub extra: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbFinanceReportListResponse {
    pub items: Vec<WbFinanceReportDto>,
    pub total_count: i32,
    pub has_more: bool,
}

#[derive(Debug, Clone)]
struct SelectedReport {
    rr_dt: String,
    rrd_id: i64,
}

async fn fetch_connections() -> Result<Vec<(String, String)>, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = "/api/connection_mp";
    let request = Request::new_with_str_and_init(url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    
    let connections: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;

    let mut result = Vec::new();
    if let Some(items) = connections.as_array() {
        for item in items {
            if let (Some(id), Some(description)) = (
                item.get("id").and_then(|v| v.as_str()),
                item.get("description").and_then(|v| v.as_str()),
            ) {
                result.push((id.to_string(), description.to_string()));
            }
        }
    }

    log!("fetch_connections: loaded {} connections", result.len());
    Ok(result)
}

#[component]
pub fn WbFinanceReportList() -> impl IntoView {
    let state = create_state();

    let (items, set_items) = signal(Vec::<WbFinanceReportDto>::new());
    let (is_loading, set_is_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (selected_report, set_selected_report) = signal::<Option<SelectedReport>>(None);

    // Загрузка списка подключений для отображения названий
    let (connections, set_connections) = signal(Vec::<(String, String)>::new());

    // RwSignals bound to Thaw controls
    let date_from = RwSignal::new(state.get_untracked().date_from.clone());
    let date_to = RwSignal::new(state.get_untracked().date_to.clone());
    let nm_id_filter = RwSignal::new(state.get_untracked().nm_id_filter.clone());
    let sa_name_filter = RwSignal::new(state.get_untracked().sa_name_filter.clone());
    let connection_filter = RwSignal::new(state.get_untracked().connection_filter.clone());
    let operation_filter = RwSignal::new(state.get_untracked().operation_filter.clone());
    let srid_filter = RwSignal::new(state.get_untracked().srid_filter.clone());

    // Load connections on mount
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            if let Ok(conns) = fetch_connections().await {
                set_connections.set(conns);
            }
        });
    });

    // Sync local RwSignals with global state
    Effect::new(move |_| {
        let from = date_from.get();
        state.update(|s| s.date_from = from);
    });
    Effect::new(move |_| {
        let to = date_to.get();
        state.update(|s| s.date_to = to);
    });
    Effect::new(move |_| {
        let nm = nm_id_filter.get();
        state.update(|s| s.nm_id_filter = nm);
    });
    Effect::new(move |_| {
        let sa = sa_name_filter.get();
        state.update(|s| s.sa_name_filter = sa);
    });
    Effect::new(move |_| {
        let conn = connection_filter.get();
        state.update(|s| s.connection_filter = conn);
    });
    Effect::new(move |_| {
        let op = operation_filter.get();
        state.update(|s| s.operation_filter = op);
    });
    Effect::new(move |_| {
        let srid = srid_filter.get();
        state.update(|s| s.srid_filter = srid);
    });

    let load = move || {
        set_is_loading.set(true);
        set_error.set(None);

        let st = state.get_untracked();
        let limit = st.page_size;
        let offset = st.page * st.page_size;
        let date_from_val = st.date_from;
        let date_to_val = st.date_to;
        let nm_id_val = st.nm_id_filter;
        let sa_name_val = st.sa_name_filter;
        let connection_val = st.connection_filter;
        let operation_val = st.operation_filter;
        let srid_val = st.srid_filter;
        let sort_by_val = st.sort_by;
        let sort_desc = !st.sort_ascending;

        leptos::task::spawn_local(async move {
            match fetch_finance_report(
                limit,
                offset,
                &date_from_val,
                &date_to_val,
                &nm_id_val,
                &sa_name_val,
                &connection_val,
                &operation_val,
                &srid_val,
                &sort_by_val,
                sort_desc,
            )
            .await
            {
                Ok(response) => {
                    let total = response.total_count.max(0) as usize;
                    let total_pages = if limit == 0 {
                        0
                    } else {
                        (total + limit - 1) / limit
                    };

                    set_items.set(response.items);
                    state.update(|s| {
                        s.total_count = total;
                        s.total_pages = total_pages;
                        s.is_loaded = true;
                    });
                    persist_state(state);
                    set_is_loading.set(false);
                }
                Err(e) => {
                    log!("Failed to fetch finance report: {:?}", e);
                    set_error.set(Some(e));
                    set_is_loading.set(false);
                }
            }
        });
    };

    // Initial load
    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            load();
        }
    });

    let go_to_page = move |page: usize| {
        state.update(|s| s.page = page);
        persist_state(state);
        load();
    };

    let change_page_size = move |size: usize| {
        state.update(|s| {
            s.page_size = size;
            s.page = 0;
        });
        persist_state(state);
        load();
    };

    let toggle_sort = move |field: &'static str| {
        state.update(|s| {
            if s.sort_by == field {
                s.sort_ascending = !s.sort_ascending;
            } else {
                s.sort_by = field.to_string();
                s.sort_ascending = true;
            }
            s.page = 0;
        });
        persist_state(state);
        load();
    };

    let handle_row_click = move |rr_dt: String, rrd_id: i64| {
        set_selected_report.set(Some(SelectedReport { rr_dt, rrd_id }));
    };

    let close_details = move || {
        set_selected_report.set(None);
    };

    // Helper для получения имени подключения
    let get_connection_name = move |connection_id: &str| -> String {
        connections
            .get()
            .iter()
            .find(|(id, _)| id == connection_id)
            .map(|(_, name)| name.clone())
            .unwrap_or_else(|| connection_id.to_string())
    };

    // Экспорт в Excel
    let export_to_excel = move || {
        let data = items.get();
        if data.is_empty() {
            log!("No data to export");
            return;
        }

        // UTF-8 BOM для правильного отображения кириллицы в Excel
        let mut csv = String::from("\u{FEFF}");

        // Заголовок с точкой с запятой как разделитель
        csv.push_str("Date;RRD_ID;NM_ID;SA_Name;Subject;Operation;Qty;Retail_Amount;Price_withDisc;Commission%;Sales_Commission;Acquiring_Fee;Penalty;Storage_Fee;SRID;Loaded_At\n");

        for item in data {
            let nm_id_str = item
                .nm_id
                .map(|n| n.to_string())
                .unwrap_or_else(|| "-".to_string());
            let sa_name_str = item.sa_name.as_ref().map(|s| s.as_str()).unwrap_or("-");
            let subject_str = item
                .subject_name
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("-");
            let operation_str = item
                .supplier_oper_name
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("-");

            let qty_str = item
                .quantity
                .map(|q| format!("{}", q))
                .unwrap_or_else(|| "-".to_string());
            let retail_amount_str = item
                .retail_amount
                .map(|r| format!("{:.2}", r).replace(".", ","))
                .unwrap_or_else(|| "-".to_string());
            let price_withdisc_str = item
                .retail_price_withdisc_rub
                .map(|p| format!("{:.2}", p).replace(".", ","))
                .unwrap_or_else(|| "-".to_string());
            let commission_str = item
                .commission_percent
                .map(|c| format!("{:.2}", c).replace(".", ","))
                .unwrap_or_else(|| "-".to_string());
            let sales_commission_str = item
                .ppvz_sales_commission
                .map(|sc| format!("{:.2}", sc).replace(".", ","))
                .unwrap_or_else(|| "-".to_string());
            let acquiring_str = item
                .acquiring_fee
                .map(|a| format!("{:.2}", a).replace(".", ","))
                .unwrap_or_else(|| "-".to_string());
            let penalty_str = item
                .penalty
                .map(|p| format!("{:.2}", p).replace(".", ","))
                .unwrap_or_else(|| "-".to_string());
            let storage_str = item
                .storage_fee
                .map(|s| format!("{:.2}", s).replace(".", ","))
                .unwrap_or_else(|| "-".to_string());
            let srid_str = item.srid.as_ref().map(|s| s.as_str()).unwrap_or("-");

            csv.push_str(&format!(
                "\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";{};{};{};{};{};{};{};{};\"{}\";\"{}\"\n",
                item.rr_dt,
                item.rrd_id,
                nm_id_str,
                sa_name_str.replace('\"', "\"\""),
                subject_str.replace('\"', "\"\""),
                operation_str.replace('\"', "\"\""),
                qty_str,
                retail_amount_str,
                price_withdisc_str,
                commission_str,
                sales_commission_str,
                acquiring_str,
                penalty_str,
                storage_str,
                srid_str.replace('\"', "\"\""),
                item.loaded_at_utc.replace('\"', "\"\"")
            ));
        }

        // Создаем Blob с CSV данными
        use js_sys::Array;
        use wasm_bindgen::JsValue;

        let array = Array::new();
        array.push(&JsValue::from_str(&csv));

        let blob_props = web_sys::BlobPropertyBag::new();
        blob_props.set_type("text/csv;charset=utf-8;");

        if let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&array, &blob_props) {
            if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        if let Ok(a) = document.create_element("a") {
                            let a: web_sys::HtmlAnchorElement = a.unchecked_into();
                            a.set_href(&url);
                            let filename = format!(
                                "wb_finance_report_{}.csv",
                                chrono::Utc::now().format("%Y%m%d_%H%M%S")
                            );
                            a.set_download(&filename);
                            a.click();
                            let _ = web_sys::Url::revoke_object_url(&url);
                        }
                    }
                }
            }
        }
    };

    view! {
        <div class="page page--wide">
            <P903Header
                total_count=Signal::derive(move || state.get().total_count)
                is_loading=Signal::derive(move || is_loading.get())
                on_refresh=Callback::new(move |_| load())
                on_export=Callback::new(move |_| export_to_excel())
            />

            {move || {
                if let Some(e) = error.get() {
                    view! {
                        <div class="warning-box warning-box--error">
                            <span class="warning-box__icon">"⚠"</span>
                            <span class="warning-box__text">{e}</span>
                        </div>
                    }
                        .into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            <div class="filter-panel">
                <div class="filter-panel-header">
                    <div class="filter-panel-header__left">
                        <span>{vec![icon("filter").into_view()]}</span>
                        <span class="filter-panel__title">"Фильтры"</span>
                    </div>

                    <div class="filter-panel-header__center">
                        <PaginationControls
                            current_page=Signal::derive(move || state.get().page)
                            total_pages=Signal::derive(move || state.get().total_pages)
                            total_count=Signal::derive(move || state.get().total_count)
                            page_size=Signal::derive(move || state.get().page_size)
                            on_page_change=Callback::new(go_to_page)
                            on_page_size_change=Callback::new(change_page_size)
                            page_size_options=vec![100, 500, 1000, 5000, 10000]
                        />
                    </div>

                    <div class="filter-panel-header__right">
                        <span class="text-muted">
                            {move || if is_loading.get() { "Загрузка…" } else { "" }}
                        </span>
                    </div>
                </div>

                <div class="filter-panel__collapsible filter-panel__collapsible--expanded">
                    <div class="filter-panel-content">
                        <Flex gap=FlexGap::Small align=FlexAlign::End>
                            <div style="min-width: 160px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"From:"</Label>
                                    <input
                                        type="date"
                                        class="form__input"
                                        prop:value=move || date_from.get()
                                        on:input=move |ev| {
                                            date_from.set(event_target_value(&ev));
                                            load();
                                        }
                                    />
                                </Flex>
                            </div>

                            <div style="min-width: 160px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"To:"</Label>
                                    <input
                                        type="date"
                                        class="form__input"
                                        prop:value=move || date_to.get()
                                        on:input=move |ev| {
                                            date_to.set(event_target_value(&ev));
                                            load();
                                        }
                                    />
                                </Flex>
                            </div>

                            <div style="min-width: 140px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"NM ID:"</Label>
                                    <Input
                                        value=nm_id_filter
                                        placeholder="NM ID..."
                                        on:input=move |_| load()
                                    />
                                </Flex>
                            </div>

                            <div style="min-width: 200px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"SA Name:"</Label>
                                    <Input
                                        value=sa_name_filter
                                        placeholder="Артикул продавца..."
                                        on:input=move |_| load()
                                    />
                                </Flex>
                            </div>

                            <div style="min-width: 180px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Кабинет:"</Label>
                                    <Select
                                        value=connection_filter
                                        on:change=move |_| load()
                                    >
                                        <option value="">"Все"</option>
                                        <For
                                            each=move || connections.get()
                                            key=|conn| conn.0.clone()
                                            children=move |conn: (String, String)| {
                                                view! {
                                                    <option value={conn.0.clone()}>{conn.1.clone()}</option>
                                                }
                                            }
                                        />
                                    </Select>
                                </Flex>
                            </div>

                            <div style="min-width: 180px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Операция:"</Label>
                                    <Select
                                        value=operation_filter
                                        on:change=move |_| load()
                                    >
                                        <option value="">"Все"</option>
                                        <option value="Продажа">"Продажа"</option>
                                        <option value="Возврат">"Возврат"</option>
                                        <option value="Логистика">"Логистика"</option>
                                        <option value="Хранение">"Хранение"</option>
                                        <option value="Платная приемка">"Платная приемка"</option>
                                        <option value="Корректировка продаж">"Корректировка продаж"</option>
                                        <option value="Корректировка возвратов">"Корректировка возвратов"</option>
                                        <option value="Прочее">"Прочее"</option>
                                    </Select>
                                </Flex>
                            </div>

                            <div style="min-width: 140px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"SRID:"</Label>
                                    <Input
                                        value=srid_filter
                                        placeholder="SRID..."
                                        on:input=move |_| load()
                                    />
                                </Flex>
                            </div>
                        </Flex>
                    </div>
                </div>
            </div>

            <div class="page-content">
                {move || {
                    if is_loading.get() && items.get().is_empty() {
                        view! { <p class="text-muted">"Загрузка..."</p> }.into_any()
                    } else {
                        let data = items.get();
                        if data.is_empty() {
                            view! { <p class="text-muted">"Нет данных"</p> }.into_any()
                        } else {
                            // Расчет итогов
                            let items_count = data.len();
                            let total_qty: i32 = data.iter().map(|item| item.quantity.unwrap_or(0)).sum();
                            let total_retail: f64 = data.iter().map(|item| item.retail_amount.unwrap_or(0.0)).sum();
                            let total_price_withdisc: f64 = data.iter().map(|item| item.retail_price_withdisc_rub.unwrap_or(0.0)).sum();
                            let total_sales_comm: f64 = data.iter().map(|item| item.ppvz_sales_commission.unwrap_or(0.0)).sum();
                            let total_acquiring: f64 = data.iter().map(|item| item.acquiring_fee.unwrap_or(0.0)).sum();
                            let total_logistics: f64 = data.iter().map(|item| item.rebill_logistic_cost.unwrap_or(0.0)).sum();
                            let total_penalty: f64 = data.iter().map(|item| item.penalty.unwrap_or(0.0)).sum();
                            let total_storage: f64 = data.iter().map(|item| item.storage_fee.unwrap_or(0.0)).sum();

                            view! {
                                <div style="padding: 10px; margin-bottom: 10px; background: var(--color-bg-secondary); border: 1px solid var(--color-border); border-radius: var(--radius-md); display: flex; gap: 20px; flex-wrap: wrap; font-size: var(--font-size-sm);">
                                    <span style="font-weight: 600;">"ИТОГО:"</span>
                                    <span style="color: var(--color-primary);">"Строк: " {items_count}</span>
                                    <span>"Qty: " {total_qty}</span>
                                    <span>"Retail: " {format_number(total_retail)}</span>
                                    <span>"Price w/Disc: " {format_number(total_price_withdisc)}</span>
                                    <span>"Sales Comm: " {format_number(total_sales_comm)}</span>
                                    <span>"Acquiring: " {format_number(total_acquiring)}</span>
                                    <span>"Logistics: " {format_number(total_logistics)}</span>
                                    <span>"Penalty: " {format_number(total_penalty)}</span>
                                    <span>"Storage: " {format_number(total_storage)}</span>
                                </div>

                                <div style="width: 100%; overflow-x: auto;">
                                    <Table attr:style="width: 100%;">
                                        <TableHeader>
                                            <TableRow>
                                                <TableHeaderCell resizable=true min_width=100.0>
                                                    "Date"
                                                    <span
                                                        class={move || get_sort_class("rr_dt", &state.get().sort_by)}
                                                        on:click=move |_| toggle_sort("rr_dt")
                                                    >
                                                        {move || get_sort_indicator("rr_dt", &state.get().sort_by, state.get().sort_ascending)}
                                                    </span>
                                                </TableHeaderCell>
                                                <TableHeaderCell resizable=true min_width=120.0>"Кабинет"</TableHeaderCell>
                                                <TableHeaderCell resizable=true min_width=80.0>
                                                    "RRD ID"
                                                    <span
                                                        class={move || get_sort_class("rrd_id", &state.get().sort_by)}
                                                        on:click=move |_| toggle_sort("rrd_id")
                                                    >
                                                        {move || get_sort_indicator("rrd_id", &state.get().sort_by, state.get().sort_ascending)}
                                                    </span>
                                                </TableHeaderCell>
                                                <TableHeaderCell resizable=true min_width=80.0>
                                                    "NM ID"
                                                    <span
                                                        class={move || get_sort_class("nm_id", &state.get().sort_by)}
                                                        on:click=move |_| toggle_sort("nm_id")
                                                    >
                                                        {move || get_sort_indicator("nm_id", &state.get().sort_by, state.get().sort_ascending)}
                                                    </span>
                                                </TableHeaderCell>
                                                <TableHeaderCell resizable=true min_width=120.0>
                                                    "SA Name"
                                                    <span
                                                        class={move || get_sort_class("sa_name", &state.get().sort_by)}
                                                        on:click=move |_| toggle_sort("sa_name")
                                                    >
                                                        {move || get_sort_indicator("sa_name", &state.get().sort_by, state.get().sort_ascending)}
                                                    </span>
                                                </TableHeaderCell>
                                                <TableHeaderCell resizable=true min_width=120.0>
                                                    "Subject"
                                                    <span
                                                        class={move || get_sort_class("subject_name", &state.get().sort_by)}
                                                        on:click=move |_| toggle_sort("subject_name")
                                                    >
                                                        {move || get_sort_indicator("subject_name", &state.get().sort_by, state.get().sort_ascending)}
                                                    </span>
                                                </TableHeaderCell>
                                                <TableHeaderCell resizable=true min_width=140.0>
                                                    "Operation"
                                                    <span
                                                        class={move || get_sort_class("supplier_oper_name", &state.get().sort_by)}
                                                        on:click=move |_| toggle_sort("supplier_oper_name")
                                                    >
                                                        {move || get_sort_indicator("supplier_oper_name", &state.get().sort_by, state.get().sort_ascending)}
                                                    </span>
                                                </TableHeaderCell>
                                                <TableHeaderCell resizable=true min_width=70.0>
                                                    "Qty"
                                                    <span
                                                        class={move || get_sort_class("quantity", &state.get().sort_by)}
                                                        on:click=move |_| toggle_sort("quantity")
                                                    >
                                                        {move || get_sort_indicator("quantity", &state.get().sort_by, state.get().sort_ascending)}
                                                    </span>
                                                </TableHeaderCell>
                                                <TableHeaderCell resizable=true min_width=90.0>
                                                    "Retail"
                                                    <span
                                                        class={move || get_sort_class("retail_amount", &state.get().sort_by)}
                                                        on:click=move |_| toggle_sort("retail_amount")
                                                    >
                                                        {move || get_sort_indicator("retail_amount", &state.get().sort_by, state.get().sort_ascending)}
                                                    </span>
                                                </TableHeaderCell>
                                                <TableHeaderCell resizable=true min_width=100.0>
                                                    "Price w/Disc"
                                                    <span
                                                        class={move || get_sort_class("retail_price_withdisc_rub", &state.get().sort_by)}
                                                        on:click=move |_| toggle_sort("retail_price_withdisc_rub")
                                                    >
                                                        {move || get_sort_indicator("retail_price_withdisc_rub", &state.get().sort_by, state.get().sort_ascending)}
                                                    </span>
                                                </TableHeaderCell>
                                                <TableHeaderCell resizable=true min_width=80.0>
                                                    "Comm%"
                                                    <span
                                                        class={move || get_sort_class("commission_percent", &state.get().sort_by)}
                                                        on:click=move |_| toggle_sort("commission_percent")
                                                    >
                                                        {move || get_sort_indicator("commission_percent", &state.get().sort_by, state.get().sort_ascending)}
                                                    </span>
                                                </TableHeaderCell>
                                                <TableHeaderCell resizable=true min_width=100.0>
                                                    "Sales Comm"
                                                    <span
                                                        class={move || get_sort_class("ppvz_sales_commission", &state.get().sort_by)}
                                                        on:click=move |_| toggle_sort("ppvz_sales_commission")
                                                    >
                                                        {move || get_sort_indicator("ppvz_sales_commission", &state.get().sort_by, state.get().sort_ascending)}
                                                    </span>
                                                </TableHeaderCell>
                                                <TableHeaderCell resizable=true min_width=90.0>
                                                    "Acquiring"
                                                    <span
                                                        class={move || get_sort_class("acquiring_fee", &state.get().sort_by)}
                                                        on:click=move |_| toggle_sort("acquiring_fee")
                                                    >
                                                        {move || get_sort_indicator("acquiring_fee", &state.get().sort_by, state.get().sort_ascending)}
                                                    </span>
                                                </TableHeaderCell>
                                                <TableHeaderCell resizable=true min_width=80.0>
                                                    "Penalty"
                                                    <span
                                                        class={move || get_sort_class("penalty", &state.get().sort_by)}
                                                        on:click=move |_| toggle_sort("penalty")
                                                    >
                                                        {move || get_sort_indicator("penalty", &state.get().sort_by, state.get().sort_ascending)}
                                                    </span>
                                                </TableHeaderCell>
                                                <TableHeaderCell resizable=true min_width=90.0>
                                                    "Logistics"
                                                    <span
                                                        class={move || get_sort_class("rebill_logistic_cost", &state.get().sort_by)}
                                                        on:click=move |_| toggle_sort("rebill_logistic_cost")
                                                    >
                                                        {move || get_sort_indicator("rebill_logistic_cost", &state.get().sort_by, state.get().sort_ascending)}
                                                    </span>
                                                </TableHeaderCell>
                                                <TableHeaderCell resizable=true min_width=80.0>
                                                    "Storage"
                                                    <span
                                                        class={move || get_sort_class("storage_fee", &state.get().sort_by)}
                                                        on:click=move |_| toggle_sort("storage_fee")
                                                    >
                                                        {move || get_sort_indicator("storage_fee", &state.get().sort_by, state.get().sort_ascending)}
                                                    </span>
                                                </TableHeaderCell>
                                                <TableHeaderCell resizable=true min_width=140.0>
                                                    "SRID"
                                                    <span
                                                        class={move || get_sort_class("srid", &state.get().sort_by)}
                                                        on:click=move |_| toggle_sort("srid")
                                                    >
                                                        {move || get_sort_indicator("srid", &state.get().sort_by, state.get().sort_ascending)}
                                                    </span>
                                                </TableHeaderCell>
                                            </TableRow>
                                        </TableHeader>

                                        <TableBody>
                                            {data
                                                .into_iter()
                                                .map(|item| {
                                                    let rr_dt_clone = item.rr_dt.clone();
                                                    let rrd_id_clone = item.rrd_id;
                                                    let connection_id = item.connection_mp_ref.clone();
                                                    view! {
                                                        <TableRow
                                                            on:click=move |_| {
                                                                handle_row_click(rr_dt_clone.clone(), rrd_id_clone)
                                                            }
                                                        >
                                                            <TableCell><TableCellLayout>{item.rr_dt}</TableCellLayout></TableCell>
                                                            <TableCell><TableCellLayout truncate=true>{get_connection_name(&connection_id)}</TableCellLayout></TableCell>
                                                            <TableCell><TableCellLayout>{item.rrd_id}</TableCellLayout></TableCell>
                                                            <TableCell><TableCellLayout>{item.nm_id.map(|n| n.to_string()).unwrap_or_else(|| "-".to_string())}</TableCellLayout></TableCell>
                                                            <TableCell><TableCellLayout truncate=true>{item.sa_name.unwrap_or_else(|| "-".to_string())}</TableCellLayout></TableCell>
                                                            <TableCell><TableCellLayout truncate=true>{item.subject_name.unwrap_or_else(|| "-".to_string())}</TableCellLayout></TableCell>
                                                            <TableCell><TableCellLayout truncate=true>{item.supplier_oper_name.unwrap_or_else(|| "-".to_string())}</TableCellLayout></TableCell>
                                                            <TableCell class="table__cell--right"><TableCellLayout>{item.quantity.map(|q| q.to_string()).unwrap_or_else(|| "-".to_string())}</TableCellLayout></TableCell>
                                                            <TableCell class="table__cell--right"><TableCellLayout>{item.retail_amount.map(|r| format_number(r)).unwrap_or_else(|| "-".to_string())}</TableCellLayout></TableCell>
                                                            <TableCell class="table__cell--right"><TableCellLayout>{item.retail_price_withdisc_rub.map(|p| format_number(p)).unwrap_or_else(|| "-".to_string())}</TableCellLayout></TableCell>
                                                            <TableCell class="table__cell--right"><TableCellLayout>{item.commission_percent.map(|c| format_number(c)).unwrap_or_else(|| "-".to_string())}</TableCellLayout></TableCell>
                                                            <TableCell class="table__cell--right"><TableCellLayout>{item.ppvz_sales_commission.map(|sc| format_number(sc)).unwrap_or_else(|| "-".to_string())}</TableCellLayout></TableCell>
                                                            <TableCell class="table__cell--right"><TableCellLayout>{item.acquiring_fee.map(|a| format_number(a)).unwrap_or_else(|| "-".to_string())}</TableCellLayout></TableCell>
                                                            <TableCell class="table__cell--right"><TableCellLayout>{item.penalty.map(|p| format_number(p)).unwrap_or_else(|| "-".to_string())}</TableCellLayout></TableCell>
                                                            <TableCell class="table__cell--right"><TableCellLayout>{item.rebill_logistic_cost.map(|l| format_number(l)).unwrap_or_else(|| "-".to_string())}</TableCellLayout></TableCell>
                                                            <TableCell class="table__cell--right"><TableCellLayout>{item.storage_fee.map(|s| format_number(s)).unwrap_or_else(|| "-".to_string())}</TableCellLayout></TableCell>
                                                            <TableCell><TableCellLayout truncate=true>{item.srid.unwrap_or_else(|| "-".to_string())}</TableCellLayout></TableCell>
                                                        </TableRow>
                                                    }
                                                    .into_view()
                                                })
                                                .collect_view()}
                                        </TableBody>
                                    </Table>
                                </div>
                            }
                                .into_any()
                        }
                    }
                }}
            </div>

            // Details panel - Modal
            {move || {
                if let Some(selected) = selected_report.get() {
                    view! {
                        <div class="modal-overlay">
                            <div class="modal modal-content-wide">
                                <crate::projections::p903_wb_finance_report::ui::details::WbFinanceReportDetail
                                    rr_dt=selected.rr_dt.clone()
                                    rrd_id=selected.rrd_id
                                    on_close=move || close_details()
                                />
                            </div>
                        </div>
                    }
                        .into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}

        </div>
    }
}

fn encode_q(s: &str) -> String {
    js_sys::encode_uri_component(s).as_string().unwrap_or_default()
}

async fn fetch_finance_report(
    limit: usize,
    offset: usize,
    date_from: &str,
    date_to: &str,
    nm_id: &str,
    sa_name: &str,
    connection: &str,
    operation: &str,
    srid: &str,
    sort_by: &str,
    sort_desc: bool,
) -> Result<WbFinanceReportListResponse, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let mut url = format!(
        "/api/p903/finance-report?limit={}&offset={}&date_from={}&date_to={}&sort_by={}&sort_desc={}",
        limit,
        offset,
        encode_q(date_from),
        encode_q(date_to),
        encode_q(sort_by),
        if sort_desc { "true" } else { "false" }
    );

    if !nm_id.trim().is_empty() {
        if let Ok(nm) = nm_id.parse::<i64>() {
            url.push_str(&format!("&nm_id={}", nm));
        }
    }
    if !sa_name.trim().is_empty() {
        url.push_str(&format!("&sa_name={}", encode_q(sa_name.trim())));
    }
    if !connection.trim().is_empty() {
        url.push_str(&format!("&connection_mp_ref={}", encode_q(connection.trim())));
    }
    if !operation.trim().is_empty() {
        url.push_str(&format!("&supplier_oper_name={}", encode_q(operation.trim())));
    }
    if !srid.trim().is_empty() {
        url.push_str(&format!("&srid={}", encode_q(srid.trim())));
    }

    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let data: WbFinanceReportListResponse = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}
