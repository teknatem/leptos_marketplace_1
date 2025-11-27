use crate::projections::p904_sales_data::state::create_state;
use crate::shared::components::date_input::DateInput;
use crate::shared::components::month_selector::MonthSelector;
use crate::shared::list_utils::format_number;
use chrono::Datelike;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use serde_json::json;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesDataDto {
    pub id: String,
    pub registrator_ref: String,
    pub registrator_type: String,
    pub date: String,
    pub connection_mp_ref: String,
    pub nomenclature_ref: String,
    pub marketplace_product_ref: String,
    pub customer_in: f64,
    pub customer_out: f64,
    pub coinvest_in: f64,
    pub commission_out: f64,
    pub acquiring_out: f64,
    pub penalty_out: f64,
    pub logistics_out: f64,
    pub seller_out: f64,
    pub price_full: f64,
    pub price_list: f64,
    pub price_return: f64,
    pub commission_percent: f64,
    pub coinvest_persent: f64,
    pub total: f64,
    pub document_no: String,
    pub article: String,
    pub posted_at: String,
    pub connection_mp_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum SortColumn {
    Date,
    DocumentNo,
    Article,
    Cabinet,
    CustomerIn,
    CustomerOut,
    CoinvestIn,
    CommissionOut,
    AcquiringOut,
    PenaltyOut,
    LogisticsOut,
    SellerOut,
    PriceFull,
    PriceList,
    PriceReturn,
    CommissionPercent,
    CoinvestPersent,
    Total,
}

impl SortColumn {
    fn as_str(&self) -> String {
        match self {
            SortColumn::Date => "Date".to_string(),
            SortColumn::DocumentNo => "DocumentNo".to_string(),
            SortColumn::Article => "Article".to_string(),
            SortColumn::Cabinet => "Cabinet".to_string(),
            SortColumn::CustomerIn => "CustomerIn".to_string(),
            SortColumn::CustomerOut => "CustomerOut".to_string(),
            SortColumn::CoinvestIn => "CoinvestIn".to_string(),
            SortColumn::CommissionOut => "CommissionOut".to_string(),
            SortColumn::AcquiringOut => "AcquiringOut".to_string(),
            SortColumn::PenaltyOut => "PenaltyOut".to_string(),
            SortColumn::LogisticsOut => "LogisticsOut".to_string(),
            SortColumn::SellerOut => "SellerOut".to_string(),
            SortColumn::PriceFull => "PriceFull".to_string(),
            SortColumn::PriceList => "PriceList".to_string(),
            SortColumn::PriceReturn => "PriceReturn".to_string(),
            SortColumn::CommissionPercent => "CommissionPercent".to_string(),
            SortColumn::CoinvestPersent => "CoinvestPersent".to_string(),
            SortColumn::Total => "Total".to_string(),
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "Date" => Some(SortColumn::Date),
            "DocumentNo" => Some(SortColumn::DocumentNo),
            "Article" => Some(SortColumn::Article),
            "Cabinet" => Some(SortColumn::Cabinet),
            "CustomerIn" => Some(SortColumn::CustomerIn),
            "CustomerOut" => Some(SortColumn::CustomerOut),
            "CoinvestIn" => Some(SortColumn::CoinvestIn),
            "CommissionOut" => Some(SortColumn::CommissionOut),
            "AcquiringOut" => Some(SortColumn::AcquiringOut),
            "PenaltyOut" => Some(SortColumn::PenaltyOut),
            "LogisticsOut" => Some(SortColumn::LogisticsOut),
            "SellerOut" => Some(SortColumn::SellerOut),
            "PriceFull" => Some(SortColumn::PriceFull),
            "PriceList" => Some(SortColumn::PriceList),
            "PriceReturn" => Some(SortColumn::PriceReturn),
            "CommissionPercent" => Some(SortColumn::CommissionPercent),
            "CoinvestPersent" => Some(SortColumn::CoinvestPersent),
            "Total" => Some(SortColumn::Total),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum SortDirection {
    Asc,
    Desc,
}

// Helper function to fetch connections from API
async fn fetch_connections_mp() -> Result<Vec<ConnectionMP>, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = "/api/connection_mp";
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
    let data: Vec<ConnectionMP> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

#[component]
pub fn SalesDataList() -> impl IntoView {
    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);

    let tabs_store =
        leptos::context::use_context::<crate::layout::global_context::AppGlobalContext>()
            .expect("AppGlobalContext context not found");

    const FORM_KEY: &str = "p904_sales_data";

    // Create local RwSignals for form fields to avoid disposed signal issues
    // Use try_with_untracked to safely handle potentially disposed signals
    let date_from = RwSignal::new(
        state.try_with_untracked(|s| s.date_from.clone())
            .unwrap_or_else(|| {
                let now = chrono::Utc::now().date_naive();
                let year = now.year();
                let month = now.month();
                chrono::NaiveDate::from_ymd_opt(year, month, 1)
                    .expect("Invalid date")
                    .format("%Y-%m-%d")
                    .to_string()
            })
    );
    let date_to = RwSignal::new(
        state.try_with_untracked(|s| s.date_to.clone())
            .unwrap_or_else(|| {
                let now = chrono::Utc::now().date_naive();
                let year = now.year();
                let month = now.month();
                if month == 12 {
                    chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
                        .map(|d| d - chrono::Duration::days(1))
                } else {
                    chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
                        .map(|d| d - chrono::Duration::days(1))
                }
                .expect("Invalid date")
                .format("%Y-%m-%d")
                .to_string()
            })
    );
    let limit = RwSignal::new(
        state.try_with_untracked(|s| s.limit.clone())
            .unwrap_or_else(|| "1000".to_string())
    );
    let cabinet_filter = RwSignal::new(
        state.try_with_untracked(|s| s.cabinet_filter.clone())
            .unwrap_or_else(|| "".to_string())
    );

    // Sync local signals with global state (safely handle disposed state)
    Effect::new(move |_| {
        let from = date_from.get();
        let _ = state.try_update(|s| s.date_from = from);
    });
    Effect::new(move |_| {
        let to = date_to.get();
        let _ = state.try_update(|s| s.date_to = to);
    });
    Effect::new(move |_| {
        let lim = limit.get();
        let _ = state.try_update(|s| s.limit = lim);
    });
    Effect::new(move |_| {
        let cab = cabinet_filter.get();
        let _ = state.try_update(|s| s.cabinet_filter = cab);
    });

    // Note: We no longer read from tabs_store/localStorage on init for filter defaults
    // because the state is now persistent in memory (state.rs).
    // However, we still support saving/loading settings to DB which overwrites state.

    let load_sales = move || {
        set_loading.set(true);
        set_error.set(None);

        let date_from_val = date_from.get_untracked();
        let date_to_val = date_to.get_untracked();
        let cabinet_val = cabinet_filter.get_untracked();
        let limit_val = limit.get_untracked();

        let mut query_params = format!(
            "?limit={}&date_from={}&date_to={}",
            limit_val, date_from_val, date_to_val
        );

        if !cabinet_val.is_empty() {
            query_params.push_str(&format!("&connection_mp_ref={}", cabinet_val));
        }

        spawn_local(async move {
            match fetch_sales(&query_params).await {
                Ok(data) => {
                    let _ = state.try_update(|s| {
                        s.sales = data;
                        s.is_loaded = true;
                    });
                    set_loading.set(false);
                }
                Err(e) => {
                    log!("Failed to fetch sales data: {:?}", e);
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    };

    // State for save settings notification
    let (save_notification, set_save_notification) = signal(None::<String>);

    // Load all cabinets from API
    let (cabinets, set_cabinets) = signal(Vec::<(String, String)>::new());
    let (cabinets_loaded, set_cabinets_loaded) = signal(false);

    // Load cabinets on mount - first priority
    Effect::new(move |_| {
        spawn_local(async move {
            match fetch_connections_mp().await {
                Ok(connections) => {
                    let mut cabinet_list: Vec<(String, String)> = connections
                        .into_iter()
                        .map(|c| {
                            use contracts::domain::common::AggregateId;
                            (c.base.id.as_string(), c.base.description)
                        })
                        .collect();
                    cabinet_list.sort_by(|a, b| a.1.cmp(&b.1));
                    let count = cabinet_list.len();
                    set_cabinets.set(cabinet_list);
                    set_cabinets_loaded.set(true);
                    log!("Loaded {} cabinets", count);
                }
                Err(e) => {
                    log!("Failed to fetch cabinets: {}", e);
                    set_cabinets_loaded.set(true); // Mark as loaded even on error
                }
            }
        });
    });

    // Load saved settings from database on mount IF not already loaded in memory
    // and AFTER cabinets are loaded
    Effect::new(move |_| {
        // Wait for cabinets to be loaded first
        if !cabinets_loaded.get() {
            return;
        }

        if !state.try_with_untracked(|s| s.is_loaded).unwrap_or(true) {
            spawn_local(async move {
                match load_saved_settings(FORM_KEY).await {
                    Ok(Some(settings)) => {
                        if let Some(date_from_val) =
                            settings.get("date_from").and_then(|v| v.as_str())
                        {
                            date_from.set(date_from_val.to_string());
                        }
                        if let Some(date_to_val) =
                            settings.get("date_to").and_then(|v| v.as_str())
                        {
                            date_to.set(date_to_val.to_string());
                        }
                        if let Some(cabinet_val) =
                            settings.get("cabinet_filter").and_then(|v| v.as_str())
                        {
                            cabinet_filter.set(cabinet_val.to_string());
                            log!("Restored cabinet filter: {}", cabinet_val);
                        }
                        if let Some(limit_val) = settings.get("limit").and_then(|v| v.as_str())
                        {
                            // Валидация лимита: минимум 100, по умолчанию 1000
                            let limit_num = limit_val.parse::<u32>().unwrap_or(1000);
                            if limit_num < 100 {
                                log!("WARNING: Invalid limit {} from saved settings, using default 1000", limit_val);
                                limit.set("1000".to_string());
                            } else {
                                limit.set(limit_val.to_string());
                            }
                        }
                        log!("Loaded saved settings for P904");
                        load_sales();
                    }
                    Ok(None) => {
                        log!("No saved settings found for P904");
                        load_sales();
                    }
                    Err(e) => {
                        log!("Failed to load saved settings: {}", e);
                        load_sales();
                    }
                }
            });
        } else {
            log!("Used cached data for P904");
        }
    });

    // Handle column click for sorting
    let handle_column_click = move |column: SortColumn| {
        let col_str = column.as_str();
        let _ = state.try_update(|s| {
            if s.sort_column.as_ref() == Some(&col_str) {
                s.sort_ascending = !s.sort_ascending;
            } else {
                s.sort_column = Some(col_str);
                s.sort_ascending = true;
            }
        });
    };

    // Sorted sales data
    let sorted_sales = move || {
        let mut data = state.with(|s| s.sales.clone());
        let sort_col_opt = state.with(|s| s.sort_column.clone());
        let sort_asc = state.with(|s| s.sort_ascending);

        if let Some(col_str) = sort_col_opt {
            if let Some(col) = SortColumn::from_str(&col_str) {
                let direction = if sort_asc {
                    SortDirection::Asc
                } else {
                    SortDirection::Desc
                };
                data.sort_by(|a, b| {
                    let cmp = match col {
                        SortColumn::Date => a.date.cmp(&b.date),
                        SortColumn::DocumentNo => a.document_no.cmp(&b.document_no),
                        SortColumn::Article => a.article.cmp(&b.article),
                        SortColumn::Cabinet => {
                            let a_cab = a.connection_mp_name.as_deref().unwrap_or("");
                            let b_cab = b.connection_mp_name.as_deref().unwrap_or("");
                            a_cab.cmp(b_cab)
                        }
                        SortColumn::CustomerIn => a
                            .customer_in
                            .partial_cmp(&b.customer_in)
                            .unwrap_or(std::cmp::Ordering::Equal),
                        SortColumn::CustomerOut => a
                            .customer_out
                            .partial_cmp(&b.customer_out)
                            .unwrap_or(std::cmp::Ordering::Equal),
                        SortColumn::CoinvestIn => a
                            .coinvest_in
                            .partial_cmp(&b.coinvest_in)
                            .unwrap_or(std::cmp::Ordering::Equal),
                        SortColumn::CommissionOut => a
                            .commission_out
                            .partial_cmp(&b.commission_out)
                            .unwrap_or(std::cmp::Ordering::Equal),
                        SortColumn::AcquiringOut => a
                            .acquiring_out
                            .partial_cmp(&b.acquiring_out)
                            .unwrap_or(std::cmp::Ordering::Equal),
                        SortColumn::PenaltyOut => a
                            .penalty_out
                            .partial_cmp(&b.penalty_out)
                            .unwrap_or(std::cmp::Ordering::Equal),
                        SortColumn::LogisticsOut => a
                            .logistics_out
                            .partial_cmp(&b.logistics_out)
                            .unwrap_or(std::cmp::Ordering::Equal),
                        SortColumn::SellerOut => a
                            .seller_out
                            .partial_cmp(&b.seller_out)
                            .unwrap_or(std::cmp::Ordering::Equal),
                        SortColumn::PriceFull => a
                            .price_full
                            .partial_cmp(&b.price_full)
                            .unwrap_or(std::cmp::Ordering::Equal),
                        SortColumn::PriceList => a
                            .price_list
                            .partial_cmp(&b.price_list)
                            .unwrap_or(std::cmp::Ordering::Equal),
                        SortColumn::PriceReturn => a
                            .price_return
                            .partial_cmp(&b.price_return)
                            .unwrap_or(std::cmp::Ordering::Equal),
                        SortColumn::CommissionPercent => a
                            .commission_percent
                            .partial_cmp(&b.commission_percent)
                            .unwrap_or(std::cmp::Ordering::Equal),
                        SortColumn::CoinvestPersent => a
                            .coinvest_persent
                            .partial_cmp(&b.coinvest_persent)
                            .unwrap_or(std::cmp::Ordering::Equal),
                        SortColumn::Total => a
                            .total
                            .partial_cmp(&b.total)
                            .unwrap_or(std::cmp::Ordering::Equal),
                    };
                    match direction {
                        SortDirection::Asc => cmp,
                        SortDirection::Desc => cmp.reverse(),
                    }
                });
            }
        }
        data
    };

    // Calculate totals
    let totals = move || {
        let data = sorted_sales();
        let count = data.len();
        let customer_in: f64 = data.iter().map(|s| s.customer_in).sum();
        let customer_out: f64 = data.iter().map(|s| s.customer_out).sum();
        let coinvest_in: f64 = data.iter().map(|s| s.coinvest_in).sum();
        let commission_out: f64 = data.iter().map(|s| s.commission_out).sum();
        let acquiring_out: f64 = data.iter().map(|s| s.acquiring_out).sum();
        let penalty_out: f64 = data.iter().map(|s| s.penalty_out).sum();
        let logistics_out: f64 = data.iter().map(|s| s.logistics_out).sum();
        let seller_out: f64 = data.iter().map(|s| s.seller_out).sum();
        let price_full: f64 = data.iter().map(|s| s.price_full).sum();
        let price_list: f64 = data.iter().map(|s| s.price_list).sum();
        let price_return: f64 = data.iter().map(|s| s.price_return).sum();
        let commission_percent: f64 = data.iter().map(|s| s.commission_percent).sum();
        let coinvest_persent: f64 = data.iter().map(|s| s.coinvest_persent).sum();
        let total: f64 = data.iter().map(|s| s.total).sum();

        (
            count,
            customer_in,
            customer_out,
            coinvest_in,
            commission_out,
            acquiring_out,
            penalty_out,
            logistics_out,
            seller_out,
            price_full,
            price_list,
            price_return,
            commission_percent,
            coinvest_persent,
            total,
        )
    };

    // Save current settings to database
    let save_settings_to_db = move |_| {
        let settings = json!({
            "date_from": date_from.get_untracked(),
            "date_to": date_to.get_untracked(),
            "cabinet_filter": cabinet_filter.get_untracked(),
            "limit": limit.get_untracked(),
        });

        spawn_local(async move {
            match save_settings_to_database(FORM_KEY, settings).await {
                Ok(_) => {
                    set_save_notification.set(Some("✓ Настройки сохранены".to_string()));
                    // Clear notification after 3 seconds
                    spawn_local(async move {
                        gloo_timers::future::TimeoutFuture::new(3000).await;
                        set_save_notification.set(None);
                    });
                }
                Err(e) => {
                    set_save_notification.set(Some(format!("✗ Ошибка: {}", e)));
                    log!("Failed to save settings: {}", e);
                }
            }
        });
    };

    let open_document =
        move |registrator_type: String, registrator_ref: String, document_no: String| {
            match registrator_type.as_str() {
                "WB_Sales" => {
                    tabs_store.open_tab(
                        &format!("a012_wb_sales_detail_{}", registrator_ref),
                        &format!("WB Sales {}", document_no),
                    );
                }
                "OZON_Transactions" => {
                    tabs_store.open_tab(
                        &format!("a014_ozon_transactions_detail_{}", registrator_ref),
                        &format!("OZON Txn {}", document_no),
                    );
                }
                _ => {
                    log!(
                        "Unknown registrator type: {}, registrator_ref: {}, document_no: {}",
                        registrator_type,
                        registrator_ref,
                        document_no
                    );
                }
            }
        };

    // Load and restore settings from database
    let restore_settings = move |_| {
        spawn_local(async move {
            match load_saved_settings(FORM_KEY).await {
                Ok(Some(settings)) => {
                    if let Some(date_from_val) =
                        settings.get("date_from").and_then(|v| v.as_str())
                    {
                        date_from.set(date_from_val.to_string());
                    }
                    if let Some(date_to_val) = settings.get("date_to").and_then(|v| v.as_str())
                    {
                        date_to.set(date_to_val.to_string());
                    }
                    if let Some(cabinet_val) =
                        settings.get("cabinet_filter").and_then(|v| v.as_str())
                    {
                        cabinet_filter.set(cabinet_val.to_string());
                    }
                    if let Some(limit_val) = settings.get("limit").and_then(|v| v.as_str()) {
                        // Валидация лимита: минимум 100, по умолчанию 1000
                        let limit_num = limit_val.parse::<u32>().unwrap_or(1000);
                        if limit_num < 100 {
                            log!("WARNING: Invalid limit {} from saved settings, using default 1000", limit_val);
                            limit.set("1000".to_string());
                        } else {
                            limit.set(limit_val.to_string());
                        }
                    }
                    set_save_notification.set(Some("✓ Настройки восстановлены".to_string()));
                    // Clear notification after 3 seconds
                    spawn_local(async move {
                        gloo_timers::future::TimeoutFuture::new(3000).await;
                        set_save_notification.set(None);
                    });
                    log!("Restored saved settings for P904");
                    load_sales(); // Reload data with new settings
                }
                Ok(None) => {
                    set_save_notification.set(Some("ℹ Нет сохраненных настроек".to_string()));
                    spawn_local(async move {
                        gloo_timers::future::TimeoutFuture::new(3000).await;
                        set_save_notification.set(None);
                    });
                    log!("No saved settings found for P904");
                }
                Err(e) => {
                    set_save_notification.set(Some(format!("✗ Ошибка: {}", e)));
                    log!("Failed to load saved settings: {}", e);
                }
            }
        });
    };

    // Helper for sort indicators
    let get_sort_indicator = move |column: SortColumn| {
        let col_str = column.as_str();
        let current_col = state.with(|s| s.sort_column.clone());
        let current_asc = state.with(|s| s.sort_ascending);

        if current_col == Some(col_str) {
            if current_asc {
                "↑"
            } else {
                "↓"
            }
        } else {
            ""
        }
    };

    view! {
        <div class="sales-data-list" style="background: #f8f9fa; padding: 12px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
            // Header - Row 1: Title with Settings Buttons
            <div style="background: linear-gradient(135deg, #4a5568 0%, #2d3748 100%); padding: 8px 12px; border-radius: 6px 6px 0 0; margin: -12px -12px 0 -12px; display: flex; align-items: center; justify-content: space-between;">
                <h2 style="margin: 0; font-size: 1.1rem; font-weight: 600; color: white; letter-spacing: 0.5px;">"📊 Sales Data (P904)"</h2>
                <div style="display: flex; gap: 8px; align-items: center;">
                    {move || {
                        if let Some(msg) = save_notification.get() {
                            view! {
                                <span style="font-size: 0.75rem; color: white; font-weight: 500; margin-right: 8px;">{msg}</span>
                            }.into_any()
                        } else {
                            view! { <></> }.into_any()
                        }
                    }}
                    <button
                        on:click=restore_settings
                        style="width: 32px; height: 32px; background: rgba(255,255,255,0.2); color: white; border: 1px solid rgba(255,255,255,0.3); border-radius: 4px; cursor: pointer; font-size: 1rem; transition: all 0.2s ease; display: flex; align-items: center; justify-content: center; padding: 0;"
                        title="Восстановить настройки из базы данных"
                    >
                        "🔄"
                    </button>
                    <button
                        on:click=save_settings_to_db
                        style="width: 32px; height: 32px; background: rgba(255,255,255,0.2); color: white; border: 1px solid rgba(255,255,255,0.3); border-radius: 4px; cursor: pointer; font-size: 1rem; transition: all 0.2s ease; display: flex; align-items: center; justify-content: center; padding: 0;"
                        title="Сохранить настройки в базу данных"
                    >
                        "💾"
                    </button>
                </div>
            </div>

            // Header - Row 2: Filters and Actions - All in one row
            <div style="background: white; padding: 8px 12px; margin: 0 -12px 10px -12px; border-bottom: 1px solid #e9ecef; display: flex; align-items: center; gap: 12px; flex-wrap: wrap;">
                // Period section
                <div style="display: flex; align-items: center; gap: 8px;">
                    <label style="margin: 0; font-size: 0.875rem; font-weight: 500; color: #495057; white-space: nowrap;">"Период:"</label>
                    <DateInput
                        value=date_from
                        on_change=move |val| date_from.set(val)
                    />
                    <span style="color: #6c757d;">"—"</span>
                    <DateInput
                        value=date_to
                        on_change=move |val| date_to.set(val)
                    />
                    <MonthSelector
                        on_select=Callback::new(move |(from, to)| {
                            date_from.set(from);
                            date_to.set(to);
                        })
                    />
                </div>

                // Cabinet filter
                <div style="display: flex; align-items: center; gap: 8px;">
                    <label style="margin: 0; font-size: 0.875rem; font-weight: 500; color: #495057; white-space: nowrap;">"Кабинет:"</label>
                    <select
                        prop:value=move || cabinet_filter.get()
                        on:change=move |ev| {
                            cabinet_filter.set(event_target_value(&ev));
                        }
                        style="padding: 6px 10px; border: 1px solid #ced4da; border-radius: 4px; font-size: 0.875rem; min-width: 150px; background: #fff;"
                    >
                        <option value="">"Все"</option>
                        {move || cabinets.get().into_iter().map(|(ref_id, name)| {
                            view! {
                                <option value=ref_id>{name}</option>
                            }
                        }).collect_view()}
                    </select>
                </div>

                // Limit selector
                <div style="display: flex; align-items: center; gap: 8px;">
                    <label style="margin: 0; font-size: 0.875rem; font-weight: 500; color: #495057; white-space: nowrap;">"Лимит:"</label>
                    <select
                        prop:value=move || limit.get()
                        on:change=move |ev| {
                            limit.set(event_target_value(&ev));
                            load_sales();
                        }
                        style="padding: 6px 10px; border: 1px solid #ced4da; border-radius: 4px; font-size: 0.875rem; min-width: 80px; background: #fff;"
                    >
                        <option value="1000">"1000"</option>
                        <option value="5000">"5000"</option>
                        <option value="10000">"10000"</option>
                        <option value="50000">"50000"</option>
                    </select>
                </div>

                // Action buttons
                <div style="margin-left: auto; display: flex; gap: 8px; align-items: center;">
                    <button
                        on:click=move |_| {
                            load_sales();
                        }
                        class="action-button action-button-success"
                        style="height: 32px; padding: 0 16px; background: #48bb78; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.875rem; font-weight: 500; transition: all 0.2s ease; display: flex; align-items: center; gap: 4px;"
                    >
                        "↻ Обновить"
                    </button>

                    <button
                        on:click=move |_| {
                            let data = sorted_sales();
                            if let Err(e) = export_to_csv(&data) {
                                log!("Failed to export: {}", e);
                            }
                        }
                        class="action-button action-button-primary"
                        style="height: 32px; padding: 0 16px; background: #217346; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.875rem; font-weight: 500; transition: all 0.2s ease; display: flex; align-items: center; gap: 4px;"
                        disabled=move || loading.get() || state.with(|s| s.sales.is_empty())
                    >
                        "📑 Excel"
                    </button>
                </div>
            </div>

            {move || {
                if loading.get() {
                    view! { <div>"Loading..."</div> }.into_any()
                } else if let Some(err) = error.get() {
                    view! { <div style="color: red;">{err}</div> }.into_any()
                } else {
                    let (count, t_customer_in, t_customer_out, t_coinvest_in, t_commission_out,
                         t_acquiring_out, t_penalty_out, t_logistics_out, t_seller_out,
                         t_price_full, t_price_list, t_price_return, t_commission_percent,
                         t_coinvest_persent, t_total) = totals();

                    view! {
                        <div style="overflow-y: auto; max-height: calc(100vh - 180px); border: 1px solid #e0e0e0;">
                            <table class="data-table table-striped" style="width: 100%; border-collapse: collapse; margin: 0; font-size: 0.8em;">
                                <thead style="position: sticky; top: 0; z-index: 10; background: var(--color-table-header-bg);">
                                    <tr>
                                        <th style="min-width: 85px; width: 85px; border-left: 1px solid #e0e0e0; border-right: 1px solid #e0e0e0; border-top: none; border-bottom: 1px solid #ddd; padding: 2px 4px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::Date)>
                                            "Date " {get_sort_indicator(SortColumn::Date)}
                                        </th>
                                        <th style="border-left: 1px solid #e0e0e0; border-right: 1px solid #e0e0e0; border-top: none; border-bottom: 1px solid #ddd; padding: 2px 4px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::DocumentNo)>
                                            "Doc No " {get_sort_indicator(SortColumn::DocumentNo)}
                                        </th>
                                        <th style="border-left: 1px solid #e0e0e0; border-right: 1px solid #e0e0e0; border-top: none; border-bottom: 1px solid #ddd; padding: 2px 4px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::Article)>
                                            "Article " {get_sort_indicator(SortColumn::Article)}
                                        </th>
                                        <th style="border-left: 1px solid #e0e0e0; border-right: 1px solid #e0e0e0; border-top: none; border-bottom: 1px solid #ddd; padding: 2px 4px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::Cabinet)>
                                            "Cabinet " {get_sort_indicator(SortColumn::Cabinet)}
                                        </th>
                                        <th style="border-left: 1px solid #e0e0e0; border-right: 1px solid #e0e0e0; border-top: none; border-bottom: 1px solid #ddd; padding: 2px 4px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::CustomerIn)>
                                            "Cust In " {get_sort_indicator(SortColumn::CustomerIn)}
                                        </th>
                                        <th style="border-left: 1px solid #e0e0e0; border-right: 1px solid #e0e0e0; border-top: none; border-bottom: 1px solid #ddd; padding: 2px 4px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::CustomerOut)>
                                            "Cust Out " {get_sort_indicator(SortColumn::CustomerOut)}
                                        </th>
                                        <th style="border-left: 1px solid #e0e0e0; border-right: 1px solid #e0e0e0; border-top: none; border-bottom: 1px solid #ddd; padding: 2px 4px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::CoinvestIn)>
                                            "Coinv In " {get_sort_indicator(SortColumn::CoinvestIn)}
                                        </th>
                                        <th style="border-left: 1px solid #e0e0e0; border-right: 1px solid #e0e0e0; border-top: none; border-bottom: 1px solid #ddd; padding: 2px 4px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::CommissionOut)>
                                            "Comm Out " {get_sort_indicator(SortColumn::CommissionOut)}
                                        </th>
                                        <th style="border-left: 1px solid #e0e0e0; border-right: 1px solid #e0e0e0; border-top: none; border-bottom: 1px solid #ddd; padding: 2px 4px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::AcquiringOut)>
                                            "Acq Out " {get_sort_indicator(SortColumn::AcquiringOut)}
                                        </th>
                                        <th style="border-left: 1px solid #e0e0e0; border-right: 1px solid #e0e0e0; border-top: none; border-bottom: 1px solid #ddd; padding: 2px 4px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::PenaltyOut)>
                                            "Pen Out " {get_sort_indicator(SortColumn::PenaltyOut)}
                                        </th>
                                        <th style="border-left: 1px solid #e0e0e0; border-right: 1px solid #e0e0e0; border-top: none; border-bottom: 1px solid #ddd; padding: 2px 4px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::LogisticsOut)>
                                            "Log Out " {get_sort_indicator(SortColumn::LogisticsOut)}
                                        </th>
                                        <th style="border-left: 1px solid #e0e0e0; border-right: 1px solid #e0e0e0; border-top: none; border-bottom: 1px solid #ddd; padding: 2px 4px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::SellerOut)>
                                            "Sell Out " {get_sort_indicator(SortColumn::SellerOut)}
                                        </th>
                                        <th style="border-left: 1px solid #e0e0e0; border-right: 1px solid #e0e0e0; border-top: none; border-bottom: 1px solid #ddd; padding: 2px 4px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::PriceFull)>
                                            "Price Full " {get_sort_indicator(SortColumn::PriceFull)}
                                        </th>
                                        <th style="border-left: 1px solid #e0e0e0; border-right: 1px solid #e0e0e0; border-top: none; border-bottom: 1px solid #ddd; padding: 2px 4px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::PriceList)>
                                            "Price List " {get_sort_indicator(SortColumn::PriceList)}
                                        </th>
                                        <th style="border-left: 1px solid #e0e0e0; border-right: 1px solid #e0e0e0; border-top: none; border-bottom: 1px solid #ddd; padding: 2px 4px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::PriceReturn)>
                                            "Price Ret " {get_sort_indicator(SortColumn::PriceReturn)}
                                        </th>
                                        <th style="border-left: 1px solid #e0e0e0; border-right: 1px solid #e0e0e0; border-top: none; border-bottom: 1px solid #ddd; padding: 2px 4px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::CommissionPercent)>
                                            "Comm %" {get_sort_indicator(SortColumn::CommissionPercent)}
                                        </th>
                                        <th style="border-left: 1px solid #e0e0e0; border-right: 1px solid #e0e0e0; border-top: none; border-bottom: 1px solid #ddd; padding: 2px 4px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::CoinvestPersent)>
                                            "Coinv %" {get_sort_indicator(SortColumn::CoinvestPersent)}
                                        </th>
                                        <th style="border-left: 1px solid #e0e0e0; border-right: 1px solid #e0e0e0; border-top: none; border-bottom: 1px solid #ddd; padding: 2px 4px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::Total)>
                                            "Total " {get_sort_indicator(SortColumn::Total)}
                                        </th>
                                    </tr>
                                    // Totals row - compact design
                                    <tr>
                                        <td style="border: 1px solid #e0e0e0; padding: 1px 2px; font-size: 0.75em; font-weight: 600; color: #2d3748;" colspan="2">
                                            {format!("📋 Итого: {} строк", count)}
                                        </td>
                                        <td style="border: 1px solid #e0e0e0; padding: 1px 2px; font-size: 0.75em;" colspan="2"></td>
                                        <td style="border: 1px solid #e0e0e0; padding: 1px 2px; text-align: right; font-size: 0.75em; font-weight: 500;">{format_number(t_customer_in)}</td>
                                        <td style="border: 1px solid #e0e0e0; padding: 1px 2px; text-align: right; font-size: 0.75em; font-weight: 500;">{format_number(t_customer_out)}</td>
                                        <td style="border: 1px solid #e0e0e0; padding: 1px 2px; text-align: right; font-size: 0.75em; font-weight: 500;">{format_number(t_coinvest_in)}</td>
                                        <td style="border: 1px solid #e0e0e0; padding: 1px 2px; text-align: right; font-size: 0.75em; font-weight: 500;">{format_number(t_commission_out)}</td>
                                        <td style="border: 1px solid #e0e0e0; padding: 1px 2px; text-align: right; font-size: 0.75em; font-weight: 500;">{format_number(t_acquiring_out)}</td>
                                        <td style="border: 1px solid #e0e0e0; padding: 1px 2px; text-align: right; font-size: 0.75em; font-weight: 500;">{format_number(t_penalty_out)}</td>
                                        <td style="border: 1px solid #e0e0e0; padding: 1px 2px; text-align: right; font-size: 0.75em; font-weight: 500;">{format_number(t_logistics_out)}</td>
                                        <td style="border: 1px solid #e0e0e0; padding: 1px 2px; text-align: right; font-size: 0.75em; font-weight: 500;">{format_number(t_seller_out)}</td>
                                        <td style="border: 1px solid #e0e0e0; padding: 1px 2px; text-align: right; font-size: 0.75em; font-weight: 500;">{format_number(t_price_full)}</td>
                                        <td style="border: 1px solid #e0e0e0; padding: 1px 2px; text-align: right; font-size: 0.75em; font-weight: 500;">{format_number(t_price_list)}</td>
                                        <td style="border: 1px solid #e0e0e0; padding: 1px 2px; text-align: right; font-size: 0.75em; font-weight: 500;">{format_number(t_price_return)}</td>
                                        <td style="border: 1px solid #e0e0e0; padding: 1px 2px; text-align: right; font-size: 0.75em; font-weight: 500;">{format_number(t_commission_percent)}</td>
                                        <td style="border: 1px solid #e0e0e0; padding: 1px 2px; text-align: right; font-size: 0.75em; font-weight: 500;">{format_number(t_coinvest_persent)}</td>
                                        <td style="border: 1px solid #e0e0e0; padding: 1px 2px; text-align: right; font-size: 0.8em; font-weight: 700; color: #2d3748;">{format_number(t_total)}</td>
                                    </tr>
                                </thead>
                                <tbody>
                                    {sorted_sales().into_iter().map(|item| {
                                        let item_clone = item.clone();
                                        let open_doc = open_document.clone();
                                        // Format date to show only date part
                                        let date_only = if item.date.len() >= 10 {
                                            item.date[0..10].to_string()
                                        } else {
                                            item.date.clone()
                                        };
                                        view! {
                                            <tr>
                                                <td style="min-width: 85px; width: 85px; border: 1px solid #e0e0e0; padding: 2px 3px;">{date_only}</td>
                                                <td style="border: 1px solid #e0e0e0; padding: 2px 3px;">
                                                    <a
                                                        href="#"
                                                        on:click=move |ev| {
                                                            ev.prevent_default();
                                                            open_doc(item_clone.registrator_type.clone(), item_clone.registrator_ref.clone(), item_clone.document_no.clone());
                                                        }
                                                        style="color: #2196F3; text-decoration: underline; cursor: pointer;"
                                                    >
                                                        {item.document_no.clone()}
                                                    </a>
                                                </td>
                                                <td style="border: 1px solid #e0e0e0; padding: 2px 3px;">{item.article.clone()}</td>
                                                <td style="border: 1px solid #e0e0e0; padding: 2px 3px;">{item.connection_mp_name.clone().unwrap_or_default()}</td>
                                                <td style="border: 1px solid #e0e0e0; padding: 2px 3px; text-align: right;">{format_number(item.customer_in)}</td>
                                                <td style="border: 1px solid #e0e0e0; padding: 2px 3px; text-align: right;">{format_number(item.customer_out)}</td>
                                                <td style="border: 1px solid #e0e0e0; padding: 2px 3px; text-align: right;">{format_number(item.coinvest_in)}</td>
                                                <td style="border: 1px solid #e0e0e0; padding: 2px 3px; text-align: right;">{format_number(item.commission_out)}</td>
                                                <td style="border: 1px solid #e0e0e0; padding: 2px 3px; text-align: right;">{format_number(item.acquiring_out)}</td>
                                                <td style="border: 1px solid #e0e0e0; padding: 2px 3px; text-align: right;">{format_number(item.penalty_out)}</td>
                                                <td style="border: 1px solid #e0e0e0; padding: 2px 3px; text-align: right;">{format_number(item.logistics_out)}</td>
                                                <td style="border: 1px solid #e0e0e0; padding: 2px 3px; text-align: right;">{format_number(item.seller_out)}</td>
                                                <td style="border: 1px solid #e0e0e0; padding: 2px 3px; text-align: right;">{format_number(item.price_full)}</td>
                                                <td style="border: 1px solid #e0e0e0; padding: 2px 3px; text-align: right;">{format_number(item.price_list)}</td>
                                                <td style="border: 1px solid #e0e0e0; padding: 2px 3px; text-align: right;">{format_number(item.price_return)}</td>
                                                <td style="border: 1px solid #e0e0e0; padding: 2px 3px; text-align: right;">{format_number(item.commission_percent)}</td>
                                                <td style="border: 1px solid #e0e0e0; padding: 2px 3px; text-align: right;">{format_number(item.coinvest_persent)}</td>
                                                <td style="border: 1px solid #e0e0e0; padding: 2px 3px; text-align: right; font-weight: bold;">{format_number(item.total)}</td>
                                            </tr>
                                        }
                                    }).collect_view()}
                                </tbody>
                            </table>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

async fn load_saved_settings(form_key: &str) -> Result<Option<serde_json::Value>, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("/api/form-settings/{}", form_key);
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

    // Response is Option<FormSettings>
    let response: Option<serde_json::Value> =
        serde_json::from_str(&text).map_err(|e| format!("{e}"))?;

    if let Some(form_settings) = response {
        if let Some(settings_json) = form_settings.get("settings_json").and_then(|v| v.as_str()) {
            let settings: serde_json::Value =
                serde_json::from_str(settings_json).map_err(|e| format!("{e}"))?;
            return Ok(Some(settings));
        }
    }

    Ok(None)
}

async fn save_settings_to_database(
    form_key: &str,
    settings: serde_json::Value,
) -> Result<(), String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let request_body = json!({
        "form_key": form_key,
        "settings": settings,
    });

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let body_str = serde_json::to_string(&request_body).map_err(|e| format!("{e}"))?;
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body_str));

    let url = "/api/form-settings";
    let request = Request::new_with_str_and_init(url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{e:?}"))?;
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

    Ok(())
}

fn export_to_csv(data: &[SalesDataDto]) -> Result<(), String> {
    use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

    // UTF-8 BOM for proper Cyrillic display in Excel
    let mut csv = String::from("\u{FEFF}");

    // Header with semicolon as delimiter
    csv.push_str("Date;Doc No;Article;Cabinet;Customer In;Customer Out;Coinvest In;Commission Out;Acquiring Out;Penalty Out;Logistics Out;Seller Out;Price Full;Price List;Price Return;Commission %;Coinvest %;Total\n");

    for sale in data {
        let document_no = sale.document_no.replace("\"", "\"\"");
        let article = sale.article.replace("\"", "\"\"");
        let cabinet = sale
            .connection_mp_name
            .as_deref()
            .unwrap_or("")
            .replace("\"", "\"\"");

        // Format date to show only date part
        let date_only = if sale.date.len() >= 10 {
            &sale.date[0..10]
        } else {
            &sale.date
        };

        // Format numbers with comma as decimal separator for Excel
        let customer_in = format!("{:.2}", sale.customer_in).replace(".", ",");
        let customer_out = format!("{:.2}", sale.customer_out).replace(".", ",");
        let coinvest_in = format!("{:.2}", sale.coinvest_in).replace(".", ",");
        let commission_out = format!("{:.2}", sale.commission_out).replace(".", ",");
        let acquiring_out = format!("{:.2}", sale.acquiring_out).replace(".", ",");
        let penalty_out = format!("{:.2}", sale.penalty_out).replace(".", ",");
        let logistics_out = format!("{:.2}", sale.logistics_out).replace(".", ",");
        let seller_out = format!("{:.2}", sale.seller_out).replace(".", ",");
        let price_full = format!("{:.2}", sale.price_full).replace(".", ",");
        let price_list = format!("{:.2}", sale.price_list).replace(".", ",");
        let price_return = format!("{:.2}", sale.price_return).replace(".", ",");
        let commission_percent = format!("{:.2}", sale.commission_percent).replace(".", ",");
        let coinvest_persent = format!("{:.2}", sale.coinvest_persent).replace(".", ",");
        let total = format!("{:.2}", sale.total).replace(".", ",");

        csv.push_str(&format!(
            "\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\n",
            date_only, document_no, article, cabinet,
            customer_in, customer_out, coinvest_in, commission_out,
            acquiring_out, penalty_out, logistics_out, seller_out,
            price_full, price_list, price_return, commission_percent,
            coinvest_persent, total
        ));
    }

    // Create Blob with CSV data
    let blob_parts = js_sys::Array::new();
    blob_parts.push(&wasm_bindgen::JsValue::from_str(&csv));

    let blob_props = BlobPropertyBag::new();
    blob_props.set_type("text/csv;charset=utf-8;");

    let blob = Blob::new_with_str_sequence_and_options(&blob_parts, &blob_props)
        .map_err(|e| format!("Failed to create blob: {:?}", e))?;

    // Create URL for blob
    let url = Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("Failed to create URL: {:?}", e))?;

    // Create temporary link for download
    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let document = window.document().ok_or_else(|| "no document".to_string())?;

    let a = document
        .create_element("a")
        .map_err(|e| format!("Failed to create element: {:?}", e))?
        .dyn_into::<HtmlAnchorElement>()
        .map_err(|e| format!("Failed to cast to anchor: {:?}", e))?;

    a.set_href(&url);
    let filename = format!(
        "sales_data_{}.csv",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    a.set_download(&filename);
    a.click();

    // Revoke URL
    Url::revoke_object_url(&url).map_err(|e| format!("Failed to revoke URL: {:?}", e))?;

    Ok(())
}

async fn fetch_sales(query_params: &str) -> Result<Vec<SalesDataDto>, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("/api/p904/sales-data{}", query_params);
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
    let data: Vec<SalesDataDto> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}
