mod state;

use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::excel_importer::{ColumnDef, DataType, ExcelImporter};
use crate::shared::icons::icon;
use crate::shared::list_utils::{format_number, get_sort_class, get_sort_indicator};
use crate::shared::modal_stack::ModalStackService;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use state::{create_state, persist_state};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thaw::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

#[component]
fn P906Header(
    #[prop(into)] total_count: Signal<usize>,
    #[prop(into)] is_loading: Signal<bool>,
    on_refresh: Callback<()>,
    on_import: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="page__header">
            <div class="page__header-left">
                {icon("dollar-sign")}
                <h1 class="page__title">"Дилерские цены (УТ)"</h1>
                <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                    <span>{move || total_count.get().to_string()}</span>
                </Badge>
            </div>

            <div class="page__header-right">
                <Button
                    appearance=ButtonAppearance::Primary
                    on_click=move |_| on_import.run(())
                >
                    {icon("upload")}
                    " Импорт Excel"
                </Button>
                <Button
                    appearance=ButtonAppearance::Secondary
                    on_click=move |_| on_refresh.run(())
                    disabled=is_loading
                >
                    {icon("refresh")}
                    {move || if is_loading.get() { " Загрузка..." } else { " Обновить" }}
                </Button>
            </div>
        </div>
    }
}

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

#[component]
pub fn NomenclaturePricesList() -> impl IntoView {
    let modal_stack =
        use_context::<ModalStackService>().expect("ModalStackService should be provided");

    let state = create_state();

    let (items, set_items) = signal(Vec::<NomenclaturePriceDto>::new());
    let (is_loading, set_is_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);

    let (available_periods, set_available_periods) = signal(Vec::<String>::new());

    // inputs bound to Thaw controls
    let period = RwSignal::new(state.get_untracked().period.clone());
    let q = RwSignal::new(state.get_untracked().q.clone());

    // Excel column definitions
    let excel_columns = vec![
        ColumnDef {
            field_name: "date".to_string(),
            title: "Дата".to_string(),
            data_type: DataType::Date,
        },
        ColumnDef {
            field_name: "article".to_string(),
            title: "Артикул".to_string(),
            data_type: DataType::String,
        },
        ColumnDef {
            field_name: "price".to_string(),
            title: "Себестоимость".to_string(),
            data_type: DataType::Number,
        },
    ];

    // Load available periods (once)
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            if let Ok(periods) = fetch_periods().await {
                set_available_periods.set(periods);
            }
        });
    });

    let load = move || {
        set_is_loading.set(true);
        set_error.set(None);

        let st = state.get_untracked();
        let limit = st.page_size;
        let offset = st.page * st.page_size;
        let period_val = st.period.clone();
        let q_val = st.q.clone();
        let sort_by = st.sort_by.clone();
        let sort_desc = !st.sort_ascending;

        leptos::task::spawn_local(async move {
            match fetch_prices(limit, offset, &period_val, &q_val, &sort_by, sort_desc).await {
                Ok(resp) => {
                    let total = resp.total_count.max(0) as usize;
                    let total_pages = if limit == 0 {
                        0
                    } else {
                        (total + limit - 1) / limit
                    };

                    set_items.set(resp.items);
                    state.update(|s| {
                        s.total_count = total;
                        s.total_pages = total_pages;
                        s.is_loaded = true;
                    });
                    persist_state(state);
                    set_is_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_is_loading.set(false);
                }
            }
        });
    };

    // Initial load (once)
    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            load();
        }
    });

    // period -> state.period and reload
    let period_first_run = StoredValue::new(true);
    Effect::new(move |_| {
        let v = period.get();
        if period_first_run.get_value() {
            period_first_run.set_value(false);
            return;
        }
        state.update(|s| {
            s.period = v;
            s.page = 0;
        });
        persist_state(state);
        load();
    });

    // q (with debounce) -> state.q and reload (server-side search)
    let debounce_timeout = StoredValue::new(None::<i32>);
    let q_first_run = StoredValue::new(true);
    Effect::new(move |_| {
        let q_now = q.get();

        if q_first_run.get_value() {
            q_first_run.set_value(false);
            return;
        }

        // Cancel previous timer
        if let Some(timeout_id) = debounce_timeout.get_value() {
            web_sys::window().and_then(|w| Some(w.clear_timeout_with_handle(timeout_id)));
        }

        // Only apply filter for len>=3, or clear if empty
        if !(q_now.trim().is_empty() || q_now.trim().len() >= 3) {
            return;
        }

        let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
            state.update(|s| {
                s.q = q_now.clone();
                s.page = 0;
            });
            persist_state(state);
            load();
        }) as Box<dyn Fn()>);

        let window = web_sys::window().expect("no window");
        let timeout_id = window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref::<js_sys::Function>(),
                300,
            )
            .expect("setTimeout failed");

        closure.forget();
        debounce_timeout.set_value(Some(timeout_id));
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
                // default to ascending when choosing a new field
                s.sort_ascending = true;
            }
            s.page = 0;
        });
        persist_state(state);
        load();
    };

    // Open Excel Importer via centralized modal stack
    let open_excel_importer = {
        let load_on_success = load.clone();
        Callback::new(move |_| {
            let columns = excel_columns.clone();
            let close_lock = Arc::new(AtomicBool::new(false));
            let close_guard = {
                let close_lock = close_lock.clone();
                Arc::new(move || !close_lock.load(Ordering::Relaxed))
            };

            modal_stack.push_with_frame_guard(
                Some("max-width: min(1400px, 95vw); width: min(1400px, 95vw);".to_string()),
                Some("excel-importer-modal".to_string()),
                Some(close_guard),
                move |handle| {
                    view! {
                        <ExcelImporter
                            columns=columns.clone()
                            import_endpoint="/api/p906/import-excel".to_string()
                            on_success=Callback::new(move |_| load_on_success())
                            close_lock=close_lock.clone()
                            on_cancel=Callback::new({
                                let handle = handle.clone();
                                move |_| handle.close()
                            })
                        />
                    }
                    .into_any()
                },
            );
        })
    };

    view! {
        <div class="page page--wide">
            <P906Header
                total_count=Signal::derive(move || state.get().total_count)
                is_loading=Signal::derive(move || is_loading.get())
                on_refresh=Callback::new(move |_| load())
                on_import=open_excel_importer
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

                <div class="filter-panel-content">
                        <Flex gap=FlexGap::Small align=FlexAlign::End>
                            <div style="min-width: 240px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Период:"</Label>
                                    <Select value=period>
                                        <option value="">"Все периоды"</option>
                                        <For
                                            each=move || available_periods.get()
                                            key=|p| p.clone()
                                            children=move |p| view! { <option value=p.clone()>{p.clone()}</option> }
                                        />
                                    </Select>
                                </Flex>
                            </div>

                            <div style="min-width: 360px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Поиск:"</Label>
                                    <Input value=q placeholder="Артикул или наименование… (мин. 3 символа)" />
                                </Flex>
                            </div>
                        </Flex>
                    </div>
            </div>

            <div class="page-content">
                <div style="width: 100%; overflow-x: auto;">
                    <Table attr:style="width: 100%;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell resizable=true min_width=120.0>
                                    "Период"
                                    <span
                                        class={move || get_sort_class("period", &state.get().sort_by)}
                                        on:click=move |_| toggle_sort("period")
                                    >
                                        {move || get_sort_indicator("period", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=120.0>
                                    "Артикул"
                                    <span
                                        class={move || get_sort_class("article", &state.get().sort_by)}
                                        on:click=move |_| toggle_sort("article")
                                    >
                                        {move || get_sort_indicator("article", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=120.0>
                                    "Код 1С"
                                    <span
                                        class={move || get_sort_class("code1c", &state.get().sort_by)}
                                        on:click=move |_| toggle_sort("code1c")
                                    >
                                        {move || get_sort_indicator("code1c", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=280.0>
                                    "Номенклатура"
                                    <span
                                        class={move || get_sort_class("nomenclature_name", &state.get().sort_by)}
                                        on:click=move |_| toggle_sort("nomenclature_name")
                                    >
                                        {move || get_sort_indicator("nomenclature_name", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=120.0>
                                    "Цена"
                                    <span
                                        class={move || get_sort_class("price", &state.get().sort_by)}
                                        on:click=move |_| toggle_sort("price")
                                    >
                                        {move || get_sort_indicator("price", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            {move || {
                                if is_loading.get() && items.get().is_empty() {
                                    return vec![view! {
                                        <TableRow>
                                            <TableCell attr:colspan="5">
                                                <TableCellLayout>
                                                    <span class="text-muted">"Загрузка…"</span>
                                                </TableCellLayout>
                                            </TableCell>
                                        </TableRow>
                                    }.into_view()];
                                }

                                let data = items.get();
                                if data.is_empty() {
                                    return vec![view! {
                                        <TableRow>
                                            <TableCell attr:colspan="5">
                                                <TableCellLayout>
                                                    <span class="text-muted">"Нет данных"</span>
                                                </TableCellLayout>
                                            </TableCell>
                                        </TableRow>
                                    }.into_view()];
                                }

                                data.into_iter()
                                    .map(|row| {
                                        let code_full = row.nomenclature_ref.clone();
                                        let code_short = if code_full.len() > 8 {
                                            format!("{}…", &code_full[..8])
                                        } else {
                                            code_full.clone()
                                        };

                                        view! {
                                            <TableRow>
                                                <TableCell><TableCellLayout>{row.period}</TableCellLayout></TableCell>
                                                <TableCell><TableCellLayout>{row.nomenclature_article.unwrap_or_default()}</TableCellLayout></TableCell>
                                                <TableCell attr:title=code_full><TableCellLayout>{code_short}</TableCellLayout></TableCell>
                                                <TableCell><TableCellLayout truncate=true>{row.nomenclature_name.unwrap_or_default()}</TableCellLayout></TableCell>
                                                <TableCell class="table__cell--right"><TableCellLayout>{format_number(row.price)}</TableCellLayout></TableCell>
                                            </TableRow>
                                        }
                                        .into_view()
                                    })
                                    .collect::<Vec<_>>()
                            }}
                        </TableBody>
                    </Table>
                </div>
            </div>
        </div>
    }
}

fn encode_q(s: &str) -> String {
    js_sys::encode_uri_component(s)
        .as_string()
        .unwrap_or_default()
}

async fn fetch_prices(
    limit: usize,
    offset: usize,
    period: &str,
    q: &str,
    sort_by: &str,
    sort_desc: bool,
) -> Result<ListResponse, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let mut url = format!(
        "/api/p906/nomenclature-prices?limit={}&offset={}&sort_by={}&sort_desc={}",
        limit,
        offset,
        encode_q(sort_by),
        if sort_desc { "true" } else { "false" }
    );

    if !period.trim().is_empty() {
        url.push_str(&format!("&period={}", encode_q(period.trim())));
    }

    if !q.trim().is_empty() {
        url.push_str(&format!("&q={}", encode_q(q.trim())));
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
