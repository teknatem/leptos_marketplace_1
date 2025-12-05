use crate::shared::list_utils::format_number;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NomenclaturePriceDto {
    pub id: String,
    pub period: String,
    pub nomenclature_ref: String,
    pub price: f64,
    pub created_at: String,
    pub updated_at: String,
    pub nomenclature_name: Option<String>,
    pub nomenclature_article: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse {
    pub items: Vec<NomenclaturePriceDto>,
    pub total_count: i64,
}

#[derive(Debug, Clone, PartialEq)]
enum SortColumn {
    Period,
    NomenclatureName,
    Article,
    Code1C,
    Price,
}

impl SortColumn {
    fn as_str(&self) -> String {
        match self {
            SortColumn::Period => "Period".to_string(),
            SortColumn::NomenclatureName => "NomenclatureName".to_string(),
            SortColumn::Article => "Article".to_string(),
            SortColumn::Code1C => "Code1C".to_string(),
            SortColumn::Price => "Price".to_string(),
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "Period" => Some(SortColumn::Period),
            "NomenclatureName" => Some(SortColumn::NomenclatureName),
            "Article" => Some(SortColumn::Article),
            "Code1C" => Some(SortColumn::Code1C),
            "Price" => Some(SortColumn::Price),
            _ => None,
        }
    }
}

#[component]
pub fn NomenclaturePricesList() -> impl IntoView {
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);
    let (prices, set_prices) = signal(Vec::<NomenclaturePriceDto>::new());
    let (total_count, set_total_count) = signal(0i64);

    // Фильтры
    let (period_filter, set_period_filter) = signal(String::new());
    let (article_filter, set_article_filter) = signal(String::new());
    let (available_periods, set_available_periods) = signal(Vec::<String>::new());
    let (limit, set_limit) = signal("1000".to_string());

    // Сортировка
    let (sort_column, set_sort_column) = signal(None::<String>);
    let (sort_ascending, set_sort_ascending) = signal(true);

    // Загрузить доступные периоды при монтировании
    Effect::new(move |_| {
        spawn_local(async move {
            match fetch_periods().await {
                Ok(periods) => {
                    set_available_periods.set(periods);
                    log!("Loaded periods for P906");
                }
                Err(e) => {
                    log!("Failed to fetch periods: {}", e);
                }
            }
        });
    });

    let load_prices = move || {
        set_loading.set(true);
        set_error.set(None);

        let period_val = period_filter.get_untracked();
        let limit_val = limit.get_untracked();

        let mut query_params = format!("?limit={}", limit_val);

        if !period_val.is_empty() {
            query_params.push_str(&format!("&period={}", period_val));
        }

        spawn_local(async move {
            match fetch_prices(&query_params).await {
                Ok(response) => {
                    set_prices.set(response.items);
                    set_total_count.set(response.total_count);
                    set_loading.set(false);
                }
                Err(e) => {
                    log!("Failed to fetch prices: {:?}", e);
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    };

    // Загрузить данные при монтировании
    Effect::new(move |_| {
        load_prices();
    });

    // Handle column click for sorting
    let handle_column_click = move |column: SortColumn| {
        let col_str = column.as_str();
        if sort_column.get_untracked().as_ref() == Some(&col_str) {
            set_sort_ascending.set(!sort_ascending.get_untracked());
        } else {
            set_sort_column.set(Some(col_str));
            set_sort_ascending.set(true);
        }
    };

    // Filtered and sorted prices data
    let filtered_sorted_prices = move || {
        let mut data = prices.get();
        let sort_col_opt = sort_column.get();
        let sort_asc = sort_ascending.get();
        let article_search = article_filter.get().to_lowercase();

        // Фильтрация по артикулу (подстрока)
        if !article_search.is_empty() {
            data.retain(|item| {
                item.nomenclature_article
                    .as_ref()
                    .map(|a| a.to_lowercase().contains(&article_search))
                    .unwrap_or(false)
            });
        }

        // Сортировка
        if let Some(col_str) = sort_col_opt {
            if let Some(col) = SortColumn::from_str(&col_str) {
                data.sort_by(|a, b| {
                    let cmp = match col {
                        SortColumn::Period => a.period.cmp(&b.period),
                        SortColumn::NomenclatureName => {
                            let a_name = a.nomenclature_name.as_deref().unwrap_or("");
                            let b_name = b.nomenclature_name.as_deref().unwrap_or("");
                            a_name.cmp(b_name)
                        }
                        SortColumn::Article => {
                            let a_art = a.nomenclature_article.as_deref().unwrap_or("");
                            let b_art = b.nomenclature_article.as_deref().unwrap_or("");
                            a_art.cmp(b_art)
                        }
                        SortColumn::Code1C => a.nomenclature_ref.cmp(&b.nomenclature_ref),
                        SortColumn::Price => a
                            .price
                            .partial_cmp(&b.price)
                            .unwrap_or(std::cmp::Ordering::Equal),
                    };
                    if sort_asc {
                        cmp
                    } else {
                        cmp.reverse()
                    }
                });
            }
        }
        data
    };

    // Helper for sort indicators
    let get_sort_indicator = move |column: SortColumn| {
        let col_str = column.as_str();
        let current_col = sort_column.get();
        let current_asc = sort_ascending.get();

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
        <div class="nomenclature-prices-list" style="background: #f8f9fa; padding: 12px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
            // Header - Row 1: Title
            <div style="background: linear-gradient(135deg, #2e7d32 0%, #1b5e20 100%); padding: 8px 12px; border-radius: 6px 6px 0 0; margin: -12px -12px 0 -12px; display: flex; align-items: center; justify-content: space-between;">
                <h2 style="margin: 0; font-size: 1.1rem; font-weight: 600; color: white; letter-spacing: 0.5px;">"💰 Плановые цены номенклатуры (P906)"</h2>
            </div>

            // Header - Row 2: Filters and Actions - All in one row
            <div style="background: white; padding: 8px 12px; margin: 0 -12px 10px -12px; border-bottom: 1px solid #e9ecef; display: flex; align-items: center; gap: 12px; flex-wrap: wrap;">
                // Period filter
                <div style="display: flex; align-items: center; gap: 8px;">
                    <label style="margin: 0; font-size: 0.875rem; font-weight: 500; color: #495057; white-space: nowrap;">"Период:"</label>
                    <select
                        prop:value=move || period_filter.get()
                        on:change=move |ev| {
                            set_period_filter.set(event_target_value(&ev));
                        }
                        style="padding: 6px 10px; border: 1px solid #ced4da; border-radius: 4px; font-size: 0.875rem; min-width: 150px; background: #fff;"
                    >
                        <option value="">"Все периоды"</option>
                        {move || available_periods.get().into_iter().map(|p| {
                            let period_clone = p.clone();
                            view! {
                                <option value=p>{period_clone}</option>
                            }
                        }).collect_view()}
                    </select>
                </div>

                // Article search filter
                <div style="display: flex; align-items: center; gap: 8px;">
                    <label style="margin: 0; font-size: 0.875rem; font-weight: 500; color: #495057; white-space: nowrap;">"Артикул:"</label>
                    <input
                        type="text"
                        placeholder="Поиск..."
                        prop:value=move || article_filter.get()
                        on:input=move |ev| {
                            set_article_filter.set(event_target_value(&ev));
                        }
                        style="padding: 6px 10px; border: 1px solid #ced4da; border-radius: 4px; font-size: 0.875rem; width: 120px; background: #fff;"
                    />
                </div>

                // Limit selector
                <div style="display: flex; align-items: center; gap: 8px;">
                    <label style="margin: 0; font-size: 0.875rem; font-weight: 500; color: #495057; white-space: nowrap;">"Лимит:"</label>
                    <select
                        prop:value=move || limit.get()
                        on:change=move |ev| {
                            set_limit.set(event_target_value(&ev));
                        }
                        style="padding: 6px 10px; border: 1px solid #ced4da; border-radius: 4px; font-size: 0.875rem; min-width: 80px; background: #fff;"
                    >
                        <option value="100">"100"</option>
                        <option value="500">"500"</option>
                        <option value="1000">"1000"</option>
                        <option value="5000">"5000"</option>
                        <option value="10000">"10000"</option>
                    </select>
                </div>

                // Action buttons
                <div style="margin-left: auto; display: flex; gap: 8px; align-items: center;">
                    <button
                        on:click=move |_| {
                            load_prices();
                        }
                        class="action-button action-button-success"
                        style="height: 32px; padding: 0 16px; background: #48bb78; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.875rem; font-weight: 500; transition: all 0.2s ease; display: flex; align-items: center; gap: 4px;"
                    >
                        "↻ Обновить"
                    </button>
                </div>
            </div>

            {move || {
                if loading.get() {
                    view! { <div style="padding: 20px; text-align: center;">"Загрузка..."</div> }.into_any()
                } else if let Some(err) = error.get() {
                    view! { <div style="color: red; padding: 20px;">{err}</div> }.into_any()
                } else {
                    let filtered_data = filtered_sorted_prices();
                    let count = filtered_data.len();

                    view! {
                        <div style="overflow-y: auto; max-height: calc(100vh - 180px); border: 1px solid #e0e0e0;">
                            <table class="data-table table-striped" style="width: 100%; border-collapse: collapse; margin: 0; font-size: 0.85em;">
                                <thead style="position: sticky; top: 0; z-index: 10; background: var(--color-table-header-bg);">
                                    <tr>
                                        <th style="min-width: 90px; width: 90px; border: 1px solid #e0e0e0; padding: 4px 6px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::Period)>
                                            "Период " {get_sort_indicator(SortColumn::Period)}
                                        </th>
                                        <th style="width: 80px; min-width: 80px; border: 1px solid #e0e0e0; padding: 4px 6px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::Article)>
                                            "Артикул " {get_sort_indicator(SortColumn::Article)}
                                        </th>
                                        <th style="width: 40px; min-width: 40px; border: 1px solid #e0e0e0; padding: 4px 6px; cursor: pointer; user-select: none; font-weight: 600; font-size: 0.75em;"
                                            on:click=move |_| handle_column_click(SortColumn::Code1C)>
                                            "Код 1С " {get_sort_indicator(SortColumn::Code1C)}
                                        </th>
                                        <th style="border: 1px solid #e0e0e0; padding: 4px 6px; cursor: pointer; user-select: none; font-weight: 600;"
                                            on:click=move |_| handle_column_click(SortColumn::NomenclatureName)>
                                            "Номенклатура " {get_sort_indicator(SortColumn::NomenclatureName)}
                                        </th>
                                        <th style="min-width: 90px; width: 100px; border: 1px solid #e0e0e0; padding: 4px 6px; cursor: pointer; user-select: none; font-weight: 600; text-align: right;"
                                            on:click=move |_| handle_column_click(SortColumn::Price)>
                                            "Цена " {get_sort_indicator(SortColumn::Price)}
                                        </th>
                                    </tr>
                                    // Totals row (without price sum)
                                    <tr>
                                        <td style="border: 1px solid #e0e0e0; padding: 2px 4px; font-size: 0.8em; font-weight: 600; color: #2d3748;" colspan="5">
                                            {format!("📋 Итого: {} записей (из {})", count, total_count.get())}
                                        </td>
                                    </tr>
                                </thead>
                                <tbody>
                                    {filtered_data.into_iter().map(|item| {
                                        // Сокращаем UUID до первых 8 символов для отображения
                                        let code_1c_short = if item.nomenclature_ref.len() > 8 {
                                            format!("{}…", &item.nomenclature_ref[..8])
                                        } else {
                                            item.nomenclature_ref.clone()
                                        };
                                        let code_1c_full = item.nomenclature_ref.clone();

                                        view! {
                                            <tr>
                                                <td style="border: 1px solid #e0e0e0; padding: 4px 6px;">{item.period.clone()}</td>
                                                <td style="border: 1px solid #e0e0e0; padding: 4px 6px; font-size: 0.9em;">{item.nomenclature_article.clone().unwrap_or_default()}</td>
                                                <td style="border: 1px solid #e0e0e0; padding: 4px 6px; font-size: 0.7em; color: #666;" title=code_1c_full>{code_1c_short}</td>
                                                <td style="border: 1px solid #e0e0e0; padding: 4px 6px;">{item.nomenclature_name.clone().unwrap_or_default()}</td>
                                                <td style="border: 1px solid #e0e0e0; padding: 4px 6px; text-align: right; font-weight: 500;">{format_number(item.price)}</td>
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

async fn fetch_prices(query_params: &str) -> Result<ListResponse, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("/api/p906/nomenclature-prices{}", query_params);
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
    let data: ListResponse = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

async fn fetch_periods() -> Result<Vec<String>, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = "/api/p906/periods";
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
    let data: Vec<String> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}
