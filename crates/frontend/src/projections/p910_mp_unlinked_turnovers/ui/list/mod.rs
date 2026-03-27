mod state;

use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::icons::icon;
use crate::shared::list_utils::{format_number, get_sort_class, get_sort_indicator};
use crate::shared::page_frame::PageFrame;
use chrono::Datelike;
use contracts::projections::p910_mp_unlinked_turnovers::dto::{
    MpUnlinkedTurnoverDto, MpUnlinkedTurnoverListResponse,
};
use leptos::prelude::*;
use state::{create_state, persist_state};
use thaw::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

fn current_month_bounds() -> (String, String) {
    let now = chrono::Utc::now().date_naive();
    let year = now.year();
    let month = now.month();
    let start = chrono::NaiveDate::from_ymd_opt(year, month, 1)
        .expect("invalid month start")
        .format("%Y-%m-%d")
        .to_string();
    let end = if month == 12 {
        chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .map(|value| value - chrono::Duration::days(1))
            .expect("invalid month end")
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
            .map(|value| value - chrono::Duration::days(1))
            .expect("invalid month end")
    }
    .format("%Y-%m-%d")
    .to_string();

    (start, end)
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_string()
    } else {
        let short: String = value.chars().take(max_chars).collect();
        format!("{short}...")
    }
}

fn open_registrator_tab(tabs: &AppGlobalContext, registrator_ref: &str) {
    let Some(rest) = registrator_ref.strip_prefix("p903:") else {
        return;
    };
    let Some((rr_dt, rrd_id)) = rest.rsplit_once(':') else {
        return;
    };
    tabs.open_tab(
        &format!(
            "p903_wb_finance_report_details_{}__{}",
            urlencoding::encode(rr_dt),
            rrd_id
        ),
        &format!("WB Finance #{rrd_id}"),
    );
}

#[component]
pub fn MpUnlinkedTurnoversList() -> impl IntoView {
    let tabs_store = use_context::<AppGlobalContext>().expect("AppGlobalContext context not found");
    let (default_from, default_to) = current_month_bounds();
    let state = create_state();
    if state.with_untracked(|state| state.date_from.is_empty()) {
        state.update(|state| state.date_from = default_from.clone());
    }
    if state.with_untracked(|state| state.date_to.is_empty()) {
        state.update(|state| state.date_to = default_to.clone());
    }

    let (items, set_items) = signal(Vec::<MpUnlinkedTurnoverDto>::new());
    let (is_loading, set_is_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);

    let date_from = RwSignal::new(state.get_untracked().date_from.clone());
    let date_to = RwSignal::new(state.get_untracked().date_to.clone());
    let connection_mp_ref = RwSignal::new(state.get_untracked().connection_mp_ref.clone());
    let layer = RwSignal::new(state.get_untracked().layer.clone());
    let turnover_code = RwSignal::new(state.get_untracked().turnover_code.clone());
    let registrator_type = RwSignal::new(state.get_untracked().registrator_type.clone());

    let active_filters_count = Signal::derive(move || {
        let state = state.get();
        let mut count = 0usize;
        for value in [
            state.date_from.as_str(),
            state.date_to.as_str(),
            state.connection_mp_ref.as_str(),
            state.layer.as_str(),
            state.turnover_code.as_str(),
            state.registrator_type.as_str(),
        ] {
            if !value.is_empty() {
                count += 1;
            }
        }
        count
    });

    let load = move || {
        let current = state.get_untracked();
        let limit = current.page_size;
        let offset = current.page * current.page_size;
        let sort_by = current.sort_by.clone();
        let sort_desc = !current.sort_ascending;

        set_is_loading.set(true);
        set_error.set(None);

        leptos::task::spawn_local(async move {
            match fetch_items(
                &current.date_from,
                &current.date_to,
                &current.connection_mp_ref,
                &current.layer,
                &current.turnover_code,
                &current.registrator_type,
                limit,
                offset,
                &sort_by,
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
                    state.update(|state| {
                        state.total_count = total;
                        state.total_pages = total_pages;
                        state.is_loaded = true;
                    });
                    persist_state(state);
                    set_is_loading.set(false);
                }
                Err(message) => {
                    set_error.set(Some(message));
                    set_is_loading.set(false);
                }
            }
        });
    };

    Effect::new(move |_| {
        if !state.with_untracked(|state| state.is_loaded) {
            load();
        }
    });

    let apply_filters = move |_| {
        state.update(|state| {
            state.date_from = date_from.get_untracked();
            state.date_to = date_to.get_untracked();
            state.connection_mp_ref = connection_mp_ref.get_untracked();
            state.layer = layer.get_untracked();
            state.turnover_code = turnover_code.get_untracked();
            state.registrator_type = registrator_type.get_untracked();
            state.page = 0;
        });
        persist_state(state);
        load();
    };

    let reset_filters = move |_| {
        date_from.set(default_from.clone());
        date_to.set(default_to.clone());
        connection_mp_ref.set(String::new());
        layer.set(String::new());
        turnover_code.set(String::new());
        registrator_type.set(String::new());
        state.update(|state| {
            state.date_from = default_from.clone();
            state.date_to = default_to.clone();
            state.connection_mp_ref.clear();
            state.layer.clear();
            state.turnover_code.clear();
            state.registrator_type.clear();
            state.page = 0;
        });
        persist_state(state);
        load();
    };

    let go_to_page = move |page: usize| {
        state.update(|state| state.page = page);
        persist_state(state);
        load();
    };

    let change_page_size = move |size: usize| {
        state.update(|state| {
            state.page_size = size;
            state.page = 0;
        });
        persist_state(state);
        load();
    };

    let toggle_sort = move |field: &'static str| {
        state.update(|state| {
            if state.sort_by == field {
                state.sort_ascending = !state.sort_ascending;
            } else {
                state.sort_by = field.to_string();
                state.sort_ascending = true;
            }
            state.page = 0;
        });
        persist_state(state);
        load();
    };

    let open_detail = move |id: String, turnover_code: String| {
        tabs_store.open_tab(
            &format!(
                "p910_mp_unlinked_turnovers_details_{}",
                urlencoding::encode(&id)
            ),
            &format!("P910 {turnover_code}"),
        );
    };

    view! {
        <PageFrame page_id="p910_mp_unlinked_turnovers--list" category="list" class="page--wide">
            <div class="page__header">
                <div class="page__header-left">
                    {icon("database")}
                    <h1 class="page__title">"P910 Unlinked Turnovers"</h1>
                    <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                        <span>{move || state.get().total_count.to_string()}</span>
                    </Badge>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| load()
                        disabled=is_loading
                    >
                        {icon("refresh")}
                        {move || if is_loading.get() { " Loading..." } else { " Reload" }}
                    </Button>
                </div>
            </div>

            {move || {
                error.get().map(|message| view! {
                    <div class="warning-box warning-box--error">
                        <span class="warning-box__icon">"!"</span>
                        <span class="warning-box__text">{message}</span>
                    </div>
                })
            }}

            <div class="page__content">
                <div class="filter-panel">
                    <div class="filter-panel-header">
                        <div class="filter-panel-header__left">
                            <span>{vec![icon("filter").into_view()]}</span>
                            <span class="filter-panel__title">"Filters"</span>
                            <Show when=move || active_filters_count.get() != 0>
                                <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Informative>
                                    <span>{move || active_filters_count.get().to_string()}</span>
                                </Badge>
                            </Show>
                        </div>

                        <div class="filter-panel-header__center">
                            <PaginationControls
                                current_page=Signal::derive(move || state.get().page)
                                total_pages=Signal::derive(move || state.get().total_pages)
                                total_count=Signal::derive(move || state.get().total_count)
                                page_size=Signal::derive(move || state.get().page_size)
                                on_page_change=Callback::new(go_to_page)
                                on_page_size_change=Callback::new(change_page_size)
                                page_size_options=vec![50, 100, 250, 500, 1000]
                            />
                        </div>
                    </div>

                    <div class="filter-panel-content">
                        <Flex vertical=true gap=FlexGap::Medium>
                            <div style="display: flex; flex-wrap: wrap; gap: 8px;">
                                <div style="min-width: 180px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Date from"</Label>
                                        <input
                                            type="date"
                                            prop:value=move || date_from.get()
                                            on:input=move |ev| date_from.set(event_target_value(&ev))
                                        />
                                    </Flex>
                                </div>
                                <div style="min-width: 180px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Date to"</Label>
                                        <input
                                            type="date"
                                            prop:value=move || date_to.get()
                                            on:input=move |ev| date_to.set(event_target_value(&ev))
                                        />
                                    </Flex>
                                </div>
                                <div style="min-width: 260px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Connection"</Label>
                                        <Input value=connection_mp_ref placeholder="connection_mp_ref" />
                                    </Flex>
                                </div>
                            </div>

                            <div style="display: flex; flex-wrap: wrap; gap: 8px; align-items: end;">
                                <div style="min-width: 120px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Layer"</Label>
                                        <Input value=layer placeholder="fact|oper|plan" />
                                    </Flex>
                                </div>
                                <div style="min-width: 220px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Turnover code"</Label>
                                        <Input value=turnover_code placeholder="turnover_code" />
                                    </Flex>
                                </div>
                                <div style="min-width: 220px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Registrator type"</Label>
                                        <Input value=registrator_type placeholder="p903_wb_finance_report" />
                                    </Flex>
                                </div>
                                <Button appearance=ButtonAppearance::Primary on_click=apply_filters>
                                    "Apply"
                                </Button>
                                <Button appearance=ButtonAppearance::Secondary on_click=reset_filters>
                                    "Reset"
                                </Button>
                            </div>
                        </Flex>
                    </div>
                </div>

                <div style="width: 100%; overflow-x: auto;" class="table-wrapper">
                    <Table attr:style="width: 100%;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell resizable=true min_width=120.0>
                                    "Date"
                                    <span
                                        class={move || get_sort_class("entry_date", &state.get().sort_by)}
                                        on:click=move |_| toggle_sort("entry_date")
                                    >
                                        {move || get_sort_indicator("entry_date", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=90.0>
                                    "Layer"
                                    <span
                                        class={move || get_sort_class("layer", &state.get().sort_by)}
                                        on:click=move |_| toggle_sort("layer")
                                    >
                                        {move || get_sort_indicator("layer", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=220.0>
                                    "Turnover"
                                    <span
                                        class={move || get_sort_class("turnover_code", &state.get().sort_by)}
                                        on:click=move |_| toggle_sort("turnover_code")
                                    >
                                        {move || get_sort_indicator("turnover_code", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=120.0>
                                    "Amount"
                                    <span
                                        class={move || get_sort_class("amount", &state.get().sort_by)}
                                        on:click=move |_| toggle_sort("amount")
                                    >
                                        {move || get_sort_indicator("amount", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=260.0>
                                    "Registrator"
                                </TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=280.0>
                                    "Comment"
                                </TableHeaderCell>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            {move || {
                                if is_loading.get() && items.get().is_empty() {
                                    return vec![view! {
                                        <TableRow>
                                            <TableCell attr:colspan="6">
                                                <TableCellLayout>
                                                    <span class="text-muted">"Loading..."</span>
                                                </TableCellLayout>
                                            </TableCell>
                                        </TableRow>
                                    }.into_view()];
                                }

                                let data = items.get();
                                if data.is_empty() {
                                    return vec![view! {
                                        <TableRow>
                                            <TableCell attr:colspan="6">
                                                <TableCellLayout>
                                                    <span class="text-muted">"No data"</span>
                                                </TableCellLayout>
                                            </TableCell>
                                        </TableRow>
                                    }.into_view()];
                                }

                                data.into_iter()
                                    .map(|row| {
                                        let turnover_title = row.turnover_description.clone();
                                        let row_id = row.id.clone();
                                        let turnover_code_for_click = row.turnover_code.clone();
                                        let row_id_for_click = row_id.clone();
                                        let turnover_code_for_link = turnover_code_for_click.clone();
                                        let registrator_ref = row.registrator_ref.clone();
                                        let registrator_ref_title = registrator_ref.clone();
                                        let registrator_ref_for_link = registrator_ref.clone();
                                        let tabs_for_registrator = tabs_store.clone();
                                        let comment_value = row.comment.clone().unwrap_or_default();
                                        let comment_title = comment_value.clone();

                                        view! {
                                            <TableRow>
                                                <TableCell><TableCellLayout>{row.entry_date}</TableCellLayout></TableCell>
                                                <TableCell><TableCellLayout>{row.layer.as_str().to_string()}</TableCellLayout></TableCell>
                                                <TableCell attr:title=turnover_title>
                                                    <TableCellLayout truncate=true>
                                                        <div>
                                                            <a
                                                                href="#"
                                                                class="table__link"
                                                                on:click=move |ev: web_sys::MouseEvent| {
                                                                    ev.prevent_default();
                                                                    open_detail(row_id_for_click.clone(), turnover_code_for_link.clone());
                                                                }
                                                            >
                                                                {row.turnover_name}
                                                            </a>
                                                        </div>
                                                        <div class="text-muted">{row.turnover_code}</div>
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell class="table__cell--right">
                                                    <TableCellLayout>{format_number(row.amount)}</TableCellLayout>
                                                </TableCell>
                                                <TableCell attr:title=registrator_ref_title>
                                                    <TableCellLayout>
                                                        <div>{row.registrator_type.clone()}</div>
                                                        <a
                                                            href="#"
                                                            class="table__link"
                                                            on:click=move |ev: web_sys::MouseEvent| {
                                                                ev.prevent_default();
                                                                open_registrator_tab(&tabs_for_registrator, &registrator_ref_for_link);
                                                            }
                                                        >
                                                            {truncate(&registrator_ref, 36)}
                                                        </a>
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell attr:title=comment_title>
                                                    <TableCellLayout truncate=true>{comment_value}</TableCellLayout>
                                                </TableCell>
                                            </TableRow>
                                        }.into_view()
                                    })
                                    .collect::<Vec<_>>()
                            }}
                        </TableBody>
                    </Table>
                </div>
            </div>
        </PageFrame>
    }
}

#[allow(clippy::too_many_arguments)]
async fn fetch_items(
    date_from: &str,
    date_to: &str,
    connection_mp_ref: &str,
    layer: &str,
    turnover_code: &str,
    registrator_type: &str,
    limit: usize,
    offset: usize,
    sort_by: &str,
    sort_desc: bool,
) -> Result<MpUnlinkedTurnoverListResponse, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let mut params = vec![
        format!("date_from={}", urlencoding::encode(date_from)),
        format!("date_to={}", urlencoding::encode(date_to)),
        format!("limit={limit}"),
        format!("offset={offset}"),
        format!("sort_by={}", urlencoding::encode(sort_by)),
        format!("sort_desc={sort_desc}"),
    ];
    if !connection_mp_ref.is_empty() {
        params.push(format!(
            "connection_mp_ref={}",
            urlencoding::encode(connection_mp_ref)
        ));
    }
    if !layer.is_empty() {
        params.push(format!("layer={}", urlencoding::encode(layer)));
    }
    if !turnover_code.is_empty() {
        params.push(format!(
            "turnover_code={}",
            urlencoding::encode(turnover_code)
        ));
    }
    if !registrator_type.is_empty() {
        params.push(format!(
            "registrator_type={}",
            urlencoding::encode(registrator_type)
        ));
    }

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("/api/p910/unlinked-turnovers?{}", params.join("&"));
    let request =
        Request::new_with_str_and_init(&url, &opts).map_err(|error| format!("{error:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|error| format!("{error:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let response_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|error| format!("{error:?}"))?;
    let response: Response = response_value
        .dyn_into()
        .map_err(|error| format!("{error:?}"))?;
    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }
    let text = JsFuture::from(response.text().map_err(|error| format!("{error:?}"))?)
        .await
        .map_err(|error| format!("{error:?}"))?;
    let text = text.as_string().ok_or_else(|| "bad text".to_string())?;
    serde_json::from_str(&text).map_err(|error| format!("{error}"))
}
