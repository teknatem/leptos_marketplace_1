use super::api;
use crate::shared::filters::ConnectionMpMultiSelect;
use crate::shared::page_frame::PageFrame;
use chrono::{Duration, Utc};
use contracts::projections::p916_mp_sales_funnel_turnovers::dto::{
    FunnelPeriodSummary, FunnelRebuildRequest,
};
use contracts::usecases::u508_repost_documents::{
    aggregate::AggregateOption,
    aggregate_request::AggregateRepostRequest,
    progress::{RepostProgress, RepostStatus},
    projection::ProjectionOption,
    request::RepostRequest,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

fn storage() -> Option<web_sys::Storage> {
    web_sys::window().and_then(|window| window.local_storage().ok().flatten())
}

fn session_key() -> &'static str {
    "u508_session_id"
}

fn progress_key() -> &'static str {
    "u508_progress"
}

fn save_session_id(id: &str) {
    if let Some(storage) = storage() {
        let _ = storage.set_item(session_key(), id);
    }
}

fn load_session_id() -> Option<String> {
    storage().and_then(|storage| storage.get_item(session_key()).ok().flatten())
}

fn save_progress(progress: &RepostProgress) {
    if let Ok(json) = serde_json::to_string(progress) {
        if let Some(storage) = storage() {
            let _ = storage.set_item(progress_key(), &json);
        }
    }
}

fn load_progress() -> Option<RepostProgress> {
    storage()
        .and_then(|storage| storage.get_item(progress_key()).ok().flatten())
        .and_then(|json| serde_json::from_str(&json).ok())
}

fn clear_storage() {
    if let Some(storage) = storage() {
        let _ = storage.remove_item(session_key());
        let _ = storage.remove_item(progress_key());
    }
}

fn is_finished(progress: &RepostProgress) -> bool {
    matches!(
        progress.status,
        RepostStatus::Completed | RepostStatus::CompletedWithErrors | RepostStatus::Failed
    )
}

#[component]
pub fn RepostDocumentsWidget() -> impl IntoView {
    let default_date_from = (Utc::now() - Duration::days(7))
        .format("%Y-%m-%d")
        .to_string();
    let default_date_to = Utc::now().format("%Y-%m-%d").to_string();
    let default_date_from_funnel = default_date_from.clone();
    let default_date_to_funnel = default_date_to.clone();

    let (projections, set_projections) = signal(Vec::<ProjectionOption>::new());
    let (selected_projection, set_selected_projection) = signal(String::new());
    let (projection_date_from, set_projection_date_from) = signal(default_date_from.clone());
    let (projection_date_to, set_projection_date_to) = signal(default_date_to.clone());

    let (aggregates, set_aggregates) = signal(Vec::<AggregateOption>::new());
    let (selected_aggregate, set_selected_aggregate) = signal(String::new());
    let (aggregate_date_from, set_aggregate_date_from) = signal(default_date_from);
    let (aggregate_date_to, set_aggregate_date_to) = signal(default_date_to);
    let aggregate_only_posted = RwSignal::new(false);
    let aggregate_connection_mp_refs = RwSignal::new(Vec::<String>::new());

    let funnel_date_from = RwSignal::new(default_date_from_funnel);
    let funnel_date_to = RwSignal::new(default_date_to_funnel);
    let funnel_connection_mp_refs = RwSignal::new(Vec::<String>::new());
    let (is_starting_funnel, set_is_starting_funnel) = signal(false);
    // Помечает, что текущая сессия — пересбор воронки: по завершении тянем диагностику.
    let funnel_run_active = RwSignal::new(false);
    let (funnel_diagnostics, set_funnel_diagnostics) = signal(None::<FunnelPeriodSummary>);

    let (session_id, set_session_id) = signal(None::<String>);
    let (progress, set_progress) = signal(None::<RepostProgress>);
    let (error_msg, set_error_msg) = signal(String::new());
    let (is_starting_projection, set_is_starting_projection) = signal(false);
    let (is_starting_aggregate, set_is_starting_aggregate) = signal(false);

    Effect::new(move || {
        spawn_local(async move {
            match api::get_projections().await {
                Ok(items) => {
                    if let Some(first) = items.first() {
                        set_selected_projection.set(first.key.clone());
                    }
                    set_projections.set(items);
                }
                Err(error) => {
                    set_error_msg.set(format!("Ошибка загрузки списка проекций: {}", error));
                }
            }
        });
    });

    Effect::new(move || {
        spawn_local(async move {
            match api::get_aggregates().await {
                Ok(items) => {
                    if let Some(first) = items.first() {
                        set_selected_aggregate.set(first.key.clone());
                    }
                    set_aggregates.set(items);
                }
                Err(error) => {
                    set_error_msg.set(format!("Ошибка загрузки списка агрегатов: {}", error));
                }
            }
        });
    });

    Effect::new(move || {
        if session_id.get().is_none() {
            if let Some(sid) = load_session_id() {
                set_session_id.set(Some(sid));
            }
            if let Some(saved_progress) = load_progress() {
                set_progress.set(Some(saved_progress));
            }
        }
    });

    Effect::new(move || {
        if let Some(sid) = session_id.get() {
            let sid_clone = sid.clone();
            spawn_local(async move {
                loop {
                    match api::get_progress(&sid_clone).await {
                        Ok(current_progress) => {
                            save_progress(&current_progress);
                            let finished = is_finished(&current_progress);
                            set_progress.set(Some(current_progress));
                            if finished {
                                clear_storage();
                                set_session_id.set(None);
                                // Пересбор воронки завершён — тянем диагностику за тот же период.
                                if funnel_run_active.get_untracked() {
                                    funnel_run_active.set(false);
                                    let from = funnel_date_from.get_untracked();
                                    let to = funnel_date_to.get_untracked();
                                    let conns = funnel_connection_mp_refs.get_untracked();
                                    match api::get_funnel_diagnostics(&from, &to, &conns).await {
                                        Ok(summary) => set_funnel_diagnostics.set(Some(summary)),
                                        Err(error) => set_error_msg
                                            .set(format!("Ошибка загрузки диагностики: {}", error)),
                                    }
                                }
                                break;
                            }
                        }
                        Err(error) => {
                            if error.contains("404") {
                                clear_storage();
                                set_session_id.set(None);
                                set_progress.set(None);
                            } else {
                                set_error_msg.set(format!("Ошибка получения прогресса: {}", error));
                            }
                            break;
                        }
                    }

                    gloo_timers::future::TimeoutFuture::new(2000).await;
                }
            });
        }
    });

    let on_start_projection = move |_| {
        let projection_key = selected_projection.get();
        if projection_key.is_empty() {
            set_error_msg.set("Сначала выберите проекцию".to_string());
            return;
        }

        let request = RepostRequest {
            projection_key,
            date_from: projection_date_from.get(),
            date_to: projection_date_to.get(),
        };

        clear_storage();
        set_is_starting_projection.set(true);
        set_error_msg.set(String::new());
        set_progress.set(None);

        spawn_local(async move {
            match api::start_repost(request).await {
                Ok(response) => {
                    save_session_id(&response.session_id);
                    set_session_id.set(Some(response.session_id));
                }
                Err(error) => {
                    set_error_msg.set(format!("Ошибка запуска: {}", error));
                }
            }
            set_is_starting_projection.set(false);
        });
    };

    let on_start_aggregate = move |_| {
        let aggregate_key = selected_aggregate.get();
        if aggregate_key.is_empty() {
            set_error_msg.set("Сначала выберите агрегат".to_string());
            return;
        }

        let request = AggregateRepostRequest {
            aggregate_key,
            date_from: aggregate_date_from.get(),
            date_to: aggregate_date_to.get(),
            only_posted: aggregate_only_posted.get(),
            connection_mp_refs: aggregate_connection_mp_refs.get(),
        };

        clear_storage();
        set_is_starting_aggregate.set(true);
        set_error_msg.set(String::new());
        set_progress.set(None);

        spawn_local(async move {
            match api::start_aggregate_repost(request).await {
                Ok(response) => {
                    save_session_id(&response.session_id);
                    set_session_id.set(Some(response.session_id));
                }
                Err(error) => {
                    set_error_msg.set(format!("Ошибка запуска: {}", error));
                }
            }
            set_is_starting_aggregate.set(false);
        });
    };

    let on_start_funnel = move |_| {
        let request = FunnelRebuildRequest {
            date_from: funnel_date_from.get(),
            date_to: funnel_date_to.get(),
            connection_mp_refs: funnel_connection_mp_refs.get(),
        };

        clear_storage();
        set_is_starting_funnel.set(true);
        set_error_msg.set(String::new());
        set_progress.set(None);
        set_funnel_diagnostics.set(None);
        funnel_run_active.set(true);

        spawn_local(async move {
            match api::start_funnel_rebuild(request).await {
                Ok(response) => {
                    save_session_id(&response.session_id);
                    set_session_id.set(Some(response.session_id));
                }
                Err(error) => {
                    funnel_run_active.set(false);
                    set_error_msg.set(format!("Ошибка запуска: {}", error));
                }
            }
            set_is_starting_funnel.set(false);
        });
    };

    view! {
        <PageFrame page_id="u508_repost_documents--usecase" category="usecase">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"u508: Перепроведение документов и проекций"</h1>
                </div>
            </div>

            {move || {
                let err = error_msg.get();
                if !err.is_empty() {
                    view! {
                        <div style="padding:12px 16px;border-radius:var(--radius-md);border-left:3px solid var(--color-error);background:var(--color-error-50);margin-bottom:16px;font-size:var(--font-size-base);">
                            {err}
                        </div>
                    }
                    .into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            <div style="display:flex;flex-direction:column;gap:16px;margin-top:16px;">
                <div style="margin-left:16px;margin-right:16px;">
                    <Card>
                        <Flex vertical=true gap=FlexGap::Small>
                            <div style="font-weight:600;">"Пересборка проекций"</div>

                            <div class="doc-filters__row">
                                <Button
                                    appearance=ButtonAppearance::Primary
                                    on_click=on_start_projection
                                    disabled=move || {
                                        selected_projection.get().is_empty()
                                            || is_starting_projection.get()
                                            || session_id.get().is_some()
                                    }
                                >
                                    {move || {
                                        if is_starting_projection.get() {
                                            "Запуск..."
                                        } else if session_id.get().is_some() {
                                            "В работе"
                                        } else {
                                            "Пересобрать"
                                        }
                                    }}
                                </Button>

                                <Flex vertical=true gap=FlexGap::Small style="flex:1;min-width:0;">
                                    <div class="doc-filter">
                                        <label class="doc-filter__label">"Проекция:"</label>
                                        <select
                                            class="doc-filter__select"
                                            style="min-width:280px;"
                                            on:change=move |ev| set_selected_projection.set(event_target_value(&ev))
                                        >
                                            <option value="">"— выберите проекцию —"</option>
                                            {move || {
                                                projections
                                                    .get()
                                                    .into_iter()
                                                    .map(|projection| {
                                                        let is_selected =
                                                            projection.key == selected_projection.get();
                                                        view! {
                                                            <option selected=is_selected value={projection.key.clone()}>
                                                                {projection.label}
                                                            </option>
                                                        }
                                                    })
                                                    .collect_view()
                                            }}
                                        </select>
                                    </div>

                                    {move || {
                                        if let Some(projection) = projections
                                            .get()
                                            .into_iter()
                                            .find(|projection| projection.key == selected_projection.get())
                                        {
                                            view! {
                                                <div style="font-size:var(--font-size-sm);color:var(--color-text-secondary);">
                                                    {projection.description}
                                                </div>
                                            }
                                            .into_any()
                                        } else {
                                            view! { <></> }.into_any()
                                        }
                                    }}

                                    <div class="doc-filter">
                                        <label class="doc-filter__label">"Период:"</label>
                                        <input
                                            type="date"
                                            class="doc-filter__input"
                                            prop:value=move || projection_date_from.get()
                                            on:change=move |ev| set_projection_date_from.set(event_target_value(&ev))
                                        />
                                        <span>"—"</span>
                                        <input
                                            type="date"
                                            class="doc-filter__input"
                                            prop:value=move || projection_date_to.get()
                                            on:change=move |ev| set_projection_date_to.set(event_target_value(&ev))
                                        />
                                    </div>
                                </Flex>
                            </div>
                        </Flex>
                    </Card>
                </div>

                <div style="margin-left:16px;margin-right:16px;">
                    <Card>
                        <Flex vertical=true gap=FlexGap::Small>
                            <div style="font-weight:600;">"Перепроведение документов с проекциями"</div>

                            <div class="doc-filters__row">
                                <Button
                                    appearance=ButtonAppearance::Primary
                                    on_click=on_start_aggregate
                                    disabled=move || {
                                        selected_aggregate.get().is_empty()
                                            || is_starting_aggregate.get()
                                            || session_id.get().is_some()
                                    }
                                >
                                    {move || {
                                        if is_starting_aggregate.get() {
                                            "Запуск..."
                                        } else if session_id.get().is_some() {
                                            "В работе"
                                        } else {
                                            "Перепровести"
                                        }
                                    }}
                                </Button>

                                <Flex vertical=true gap=FlexGap::Small style="flex:1;min-width:0;">
                                    <div class="doc-filter">
                                        <label class="doc-filter__label">"Тип агрегата:"</label>
                                        <select
                                            class="doc-filter__select"
                                            style="min-width:280px;"
                                            on:change=move |ev| set_selected_aggregate.set(event_target_value(&ev))
                                        >
                                            <option value="">"— выберите агрегат —"</option>
                                            {move || {
                                                aggregates
                                                    .get()
                                                    .into_iter()
                                                    .map(|aggregate| {
                                                        let is_selected =
                                                            aggregate.key == selected_aggregate.get();
                                                        view! {
                                                            <option selected=is_selected value={aggregate.key.clone()}>
                                                                {aggregate.label}
                                                            </option>
                                                        }
                                                    })
                                                    .collect_view()
                                            }}
                                        </select>
                                    </div>

                                    {move || {
                                        if let Some(aggregate) = aggregates
                                            .get()
                                            .into_iter()
                                            .find(|aggregate| aggregate.key == selected_aggregate.get())
                                        {
                                            view! {
                                                <div style="font-size:var(--font-size-sm);color:var(--color-text-secondary);">
                                                    {aggregate.description}
                                                </div>
                                            }
                                            .into_any()
                                        } else {
                                            view! { <></> }.into_any()
                                        }
                                    }}

                                    <div class="doc-filter">
                                        <label class="doc-filter__label">"Период:"</label>
                                        <input
                                            type="date"
                                            class="doc-filter__input"
                                            prop:value=move || aggregate_date_from.get()
                                            on:change=move |ev| set_aggregate_date_from.set(event_target_value(&ev))
                                        />
                                        <span>"—"</span>
                                        <input
                                            type="date"
                                            class="doc-filter__input"
                                            prop:value=move || aggregate_date_to.get()
                                            on:change=move |ev| set_aggregate_date_to.set(event_target_value(&ev))
                                        />
                                    </div>
                                    {move || {
                                        if selected_aggregate.get() == "a012_wb_sales" {
                                            view! {
                                                <div class="doc-filter" style="align-items:flex-start;">
                                                    <label class="doc-filter__label">"Кабинеты:"</label>
                                                    <div style="display:flex;flex-direction:column;gap:6px;">
                                                        <ConnectionMpMultiSelect selected=aggregate_connection_mp_refs />
                                                        <span style="font-size:var(--font-size-xs);color:var(--color-text-secondary);">
                                                            "Если ничего не выбрано, будут обработаны все кабинеты"
                                                        </span>
                                                    </div>
                                                </div>
                                            }
                                            .into_any()
                                        } else {
                                            view! { <></> }.into_any()
                                        }
                                    }}
                                    <Checkbox
                                        checked=aggregate_only_posted
                                        label="Только проведенные"
                                    />
                                </Flex>
                            </div>
                        </Flex>
                    </Card>
                </div>

                <div style="margin-left:16px;margin-right:16px;">
                    <Card>
                        <Flex vertical=true gap=FlexGap::Small>
                            <div style="font-weight:600;">"Пересбор воронки за период"</div>
                            <div style="font-size:var(--font-size-sm);color:var(--color-text-secondary);">
                                "Перепроводит a015 (заказы/отмены) и a012 (выкупы/возвраты) за период и пересобирает верх воронки из сохранённых a036 → корректная воронка p916. По завершении — диагностика."
                            </div>

                            <div class="doc-filters__row">
                                <Button
                                    appearance=ButtonAppearance::Primary
                                    on_click=on_start_funnel
                                    disabled=move || {
                                        is_starting_funnel.get() || session_id.get().is_some()
                                    }
                                >
                                    {move || {
                                        if is_starting_funnel.get() {
                                            "Запуск..."
                                        } else if session_id.get().is_some() {
                                            "В работе"
                                        } else {
                                            "Пересобрать воронку"
                                        }
                                    }}
                                </Button>

                                <Flex vertical=true gap=FlexGap::Small style="flex:1;min-width:0;">
                                    <div class="doc-filter">
                                        <label class="doc-filter__label">"Период:"</label>
                                        <input
                                            type="date"
                                            class="doc-filter__input"
                                            prop:value=move || funnel_date_from.get()
                                            on:change=move |ev| funnel_date_from.set(event_target_value(&ev))
                                        />
                                        <span>"—"</span>
                                        <input
                                            type="date"
                                            class="doc-filter__input"
                                            prop:value=move || funnel_date_to.get()
                                            on:change=move |ev| funnel_date_to.set(event_target_value(&ev))
                                        />
                                    </div>
                                    <div class="doc-filter" style="align-items:flex-start;">
                                        <label class="doc-filter__label">"Кабинеты:"</label>
                                        <div style="display:flex;flex-direction:column;gap:6px;">
                                            <ConnectionMpMultiSelect selected=funnel_connection_mp_refs />
                                            <span style="font-size:var(--font-size-xs);color:var(--color-text-secondary);">
                                                "Если ничего не выбрано, будут обработаны все кабинеты"
                                            </span>
                                        </div>
                                    </div>
                                </Flex>
                            </div>
                        </Flex>
                    </Card>
                </div>

                <div style="margin-left:16px;margin-right:16px;">
                    <Card>
                        <Flex vertical=true gap=FlexGap::Small>
                            <div style="font-weight:600;">"Статус выполнения"</div>

                            {move || {
                                if let Some(current_progress) = progress.get() {
                                    let total = current_progress.total.unwrap_or(0);
                                    let percent = if total > 0 {
                                        ((current_progress.processed as f64 / total as f64) * 100.0)
                                            .clamp(0.0, 100.0) as i32
                                    } else if current_progress.processed > 0 {
                                        100
                                    } else {
                                        0
                                    };

                                    view! {
                                        <div style="display:flex;flex-direction:column;gap:10px;">
                                            <div style="display:flex;align-items:center;gap:10px;flex:1;min-width:0;">
                                                <span style="font-size:var(--font-size-sm);color:var(--color-text-secondary);min-width:140px;">
                                                    {format!("{:?}", current_progress.status)}
                                                </span>
                                                <div style="height:16px;border-radius:var(--radius-sm);overflow:hidden;background:var(--color-border);flex:1;min-width:200px;">
                                                    <div style={format!("width:{}%;height:100%;background:var(--colorBrandForeground1);transition:width 0.2s;", percent)}></div>
                                                </div>
                                                <span style="font-size:var(--font-size-sm);color:var(--color-text-secondary);min-width:85px;text-align:right;">
                                                    {if total > 0 {
                                                        format!("{} / {}", current_progress.processed, total)
                                                    } else {
                                                        format!("{}", current_progress.processed)
                                                    }}
                                                </span>
                                                <span style="font-size:var(--font-size-sm);color:var(--color-text-secondary);min-width:150px;">
                                                    {format!("ok: {}  err: {}", current_progress.reposted, current_progress.errors)}
                                                </span>
                                            </div>

                                            {if let Some(current_item) = current_progress.current_item {
                                                view! {
                                                    <div style="font-size:var(--font-size-sm);color:var(--color-text-secondary);">
                                                        {format!("Текущий документ: {}", current_item)}
                                                    </div>
                                                }
                                                .into_any()
                                            } else {
                                                view! { <></> }.into_any()
                                            }}

                                            {if let Some(chunk_label) = current_progress.current_chunk_label.clone() {
                                                view! {
                                                    <div style="font-size:var(--font-size-sm);color:var(--color-text-secondary);">
                                                        {format!(
                                                            "Текущий чанк: {} ({} / {})",
                                                            chunk_label,
                                                            current_progress.chunks_processed,
                                                            current_progress.chunks_total.unwrap_or(0)
                                                        )}
                                                    </div>
                                                }
                                                .into_any()
                                            } else {
                                                view! { <></> }.into_any()
                                            }}

                                            {if !current_progress.error_messages.is_empty() {
                                                view! {
                                                    <div style="margin-top:8px;padding:8px 12px;border-radius:var(--radius-md);border-left:3px solid var(--color-error);background:var(--color-error-50);font-size:var(--font-size-sm);max-height:120px;overflow-y:auto;">
                                                        {current_progress
                                                            .error_messages
                                                            .iter()
                                                            .map(|item| view! { <div>{item.clone()}</div> })
                                                            .collect_view()}
                                                    </div>
                                                }
                                                .into_any()
                                            } else {
                                                view! { <></> }.into_any()
                                            }}
                                        </div>
                                    }
                                    .into_any()
                                } else {
                                    view! {
                                        <span style="font-size:var(--font-size-sm);color:var(--color-text-secondary);">
                                            "Готово к запуску"
                                        </span>
                                    }
                                    .into_any()
                                }
                            }}
                        </Flex>
                    </Card>
                </div>

                {move || {
                    if let Some(s) = funnel_diagnostics.get() {
                        let pct = |num: i64, den: i64| -> String {
                            if den > 0 {
                                format!("{:.1}%", (num as f64 / den as f64) * 100.0)
                            } else {
                                "—".to_string()
                            }
                        };
                        let row = |label: &str, value: String| {
                            view! {
                                <div style="display:flex;justify-content:space-between;gap:16px;padding:2px 0;border-bottom:1px solid var(--color-border);">
                                    <span style="color:var(--color-text-secondary);">{label.to_string()}</span>
                                    <span style="font-variant-numeric:tabular-nums;">{value}</span>
                                </div>
                            }
                        };
                        view! {
                            <div style="margin-left:16px;margin-right:16px;">
                                <Card>
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <div style="font-weight:600;">
                                            {format!("Диагностика воронки за период {} — {}", s.date_from, s.date_to)}
                                        </div>
                                        <div style="display:grid;grid-template-columns:1fr 1fr;gap:0 32px;font-size:var(--font-size-sm);">
                                            <div>
                                                <div style="font-weight:600;margin-bottom:4px;">"Верх воронки"</div>
                                                {row("Переходы (openCard)", s.open_count.to_string())}
                                                {row("В корзину", s.cart_count.to_string())}
                                                {row("Отложенные", s.wishlist_count.to_string())}
                                                {row("Заказы (воронка a036)", format!("{} / {:.0} ₽", s.funnel_order_count, s.funnel_order_sum))}
                                                {row("Конв. в корзину", pct(s.cart_count, s.open_count))}
                                                {row("Конв. в заказ", pct(s.funnel_order_count, s.cart_count))}
                                            </div>
                                            <div>
                                                <div style="font-weight:600;margin-bottom:4px;">"Заказ → завершение"</div>
                                                {row("Заказы (a015)", format!("{} / {:.0} ₽", s.order_count, s.order_sum))}
                                                {row("Отмены", format!("{} / {:.0} ₽", s.cancel_count, s.cancel_sum))}
                                                {row("Выкупы (a012)", format!("{} / {:.0} ₽", s.buyout_count, s.buyout_sum))}
                                                {row("Возвраты", format!("{} / {:.0} ₽", s.return_count, s.return_sum))}
                                                {row("Доля отмен", pct(s.cancel_count, s.order_count))}
                                                {row("Выкуп (от заказов)", pct(s.buyout_count, s.order_count))}
                                                {row("Доля возвратов", pct(s.return_count, s.buyout_count))}
                                            </div>
                                        </div>
                                        <div style="font-size:var(--font-size-xs);color:var(--color-text-secondary);">
                                            {format!(
                                                "Строк движений p916: маркетинг {}, fulfillment {}. Примечание: «заказы воронки» (a036) и «заказы» (a015) — разные метрики и могут не совпадать.",
                                                s.marketing_rows, s.fulfillment_rows
                                            )}
                                        </div>
                                    </Flex>
                                </Card>
                            </div>
                        }
                        .into_any()
                    } else {
                        view! { <></> }.into_any()
                    }
                }}
            </div>
        </PageFrame>
    }
}
