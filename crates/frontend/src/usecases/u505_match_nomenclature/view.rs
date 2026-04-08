use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_USECASE;
use contracts::usecases::u505_match_nomenclature::{
    progress::MatchStatus, MatchProgress, MatchRequest,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn MatchNomenclatureView() -> impl IntoView {
    let (session_id, set_session_id) = signal(Option::<String>::None);
    let (progress, set_progress) = signal(Option::<MatchProgress>::None);
    let (error_message, set_error_message) = signal(Option::<String>::None);
    let (is_loading, set_is_loading) = signal(false);

    let (overwrite_existing, set_overwrite_existing) = signal(false);
    let (ignore_case, set_ignore_case) = signal(true);

    let start_matching = move |_| {
        set_error_message.set(None);
        set_is_loading.set(true);

        let request = MatchRequest {
            marketplace_id: None,
            overwrite_existing: overwrite_existing.get(),
            ignore_case: ignore_case.get(),
        };

        spawn_local(async move {
            match super::api::start_matching(request).await {
                Ok(response) => {
                    set_session_id.set(Some(response.session_id.clone()));
                    set_is_loading.set(false);
                }
                Err(e) => {
                    set_error_message.set(Some(format!("Ошибка запуска: {}", e)));
                    set_is_loading.set(false);
                }
            }
        });
    };

    Effect::new(move || {
        if let Some(sid) = session_id.get() {
            let sid_clone = sid.clone();
            spawn_local(async move {
                loop {
                    match super::api::get_progress(&sid_clone).await {
                        Ok(prog) => {
                            let is_finished = !matches!(prog.status, MatchStatus::InProgress);
                            set_progress.set(Some(prog.clone()));
                            if is_finished {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                    gloo_timers::future::TimeoutFuture::new(2000).await;
                }
            });
        }
    });

    let status_label = |status: &MatchStatus| match status {
        MatchStatus::InProgress => "В процессе",
        MatchStatus::Completed => "Завершено",
        MatchStatus::CompletedWithErrors => "Завершено с ошибками",
        MatchStatus::Failed => "Провалено",
    };

    let status_class = |status: &MatchStatus| match status {
        MatchStatus::InProgress => "u505-match__status u505-match__status--info",
        MatchStatus::Completed => "u505-match__status u505-match__status--success",
        MatchStatus::CompletedWithErrors => "u505-match__status u505-match__status--warn",
        MatchStatus::Failed => "u505-match__status u505-match__status--danger",
    };

    view! {
        <PageFrame page_id="u505_match_nomenclature--usecase" category=PAGE_CAT_USECASE>
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Сопоставление номенклатуры"</h1>
                </div>
            </div>

            <div class="page__content">
                <div class="u505-match">
                    <section class="u505-match__section">
                        <div class="u505-match__lead">
                            "Массовое сопоставление товаров маркетплейсов с номенклатурой 1С по артикулу."
                        </div>

                        <div class="u505-match__options">
                            <label class="u505-match__checkbox">
                                <input
                                    type="checkbox"
                                    prop:checked=move || ignore_case.get()
                                    on:change=move |ev| set_ignore_case.set(event_target_checked(&ev))
                                />
                                <span>"Игнорировать регистр при сопоставлении"</span>
                            </label>

                            <label class="u505-match__checkbox">
                                <input
                                    type="checkbox"
                                    prop:checked=move || overwrite_existing.get()
                                    on:change=move |ev| set_overwrite_existing.set(event_target_checked(&ev))
                                />
                                <span>"Перезаписать существующие связи"</span>
                            </label>
                        </div>

                        <div class="u505-match__actions">
                            <button
                                class="u505-match__button"
                                on:click=start_matching
                                prop:disabled=move || is_loading.get() || session_id.get().is_some()
                            >
                                {move || if is_loading.get() {
                                    "Запуск..."
                                } else if session_id.get().is_some() {
                                    "Сопоставление запущено"
                                } else {
                                    "Запустить сопоставление"
                                }}
                            </button>
                        </div>

                        {move || {
                            error_message.get().map(|msg| {
                                view! { <div class="u505-match__message u505-match__message--error">{msg}</div> }
                            })
                        }}
                    </section>

                    {move || {
                        progress.get().map(|prog| {
                            let total = prog.total.unwrap_or(0);
                            let pct = if total > 0 {
                                ((prog.processed as f64 / total as f64) * 100.0).round() as i32
                            } else {
                                0
                            };

                            view! {
                                <section class="u505-match__section">
                                    <div class="u505-match__progress-header">
                                        <h2 class="u505-match__section-title">"Прогресс"</h2>
                                        <span class=status_class(&prog.status)>
                                            {status_label(&prog.status)}
                                        </span>
                                    </div>

                                    <div class="u505-match__progress-meta">
                                        <div>
                                            <strong>"Обработано:"</strong>
                                            " "
                                            {prog.processed}
                                            {if total > 0 {
                                                format!(" / {}", total)
                                            } else {
                                                String::new()
                                            }}
                                        </div>
                                        {if total > 0 {
                                            view! { <div><strong>"Выполнено:"</strong> " " {format!("{pct}%")}</div> }.into_any()
                                        } else {
                                            view! { <></> }.into_any()
                                        }}
                                    </div>

                                    <div class="u505-match__progressbar">
                                        <div
                                            class="u505-match__progressbar-fill"
                                            style=format!("width: {}%;", pct.clamp(0, 100))
                                        ></div>
                                    </div>

                                    <div class="u505-match__stats">
                                        <div class="u505-match__stat">
                                            <span class="u505-match__stat-label">"Сопоставлено"</span>
                                            <strong>{prog.matched}</strong>
                                        </div>
                                        <div class="u505-match__stat">
                                            <span class="u505-match__stat-label">"Очищено"</span>
                                            <strong>{prog.cleared}</strong>
                                        </div>
                                        <div class="u505-match__stat">
                                            <span class="u505-match__stat-label">"Без изменений"</span>
                                            <strong>{prog.skipped}</strong>
                                        </div>
                                        <div class="u505-match__stat">
                                            <span class="u505-match__stat-label">"Неоднозначно"</span>
                                            <strong>{prog.ambiguous}</strong>
                                        </div>
                                        <div class="u505-match__stat">
                                            <span class="u505-match__stat-label">"Ошибки"</span>
                                            <strong>{prog.errors}</strong>
                                        </div>
                                    </div>

                                    {prog.current_item.as_ref().map(|item| {
                                        view! {
                                            <div class="u505-match__current-item">
                                                <strong>"Текущий элемент:"</strong>
                                                " "
                                                {item.clone()}
                                            </div>
                                        }
                                    })}

                                    {if prog.error_list.is_empty() {
                                        view! { <></> }.into_any()
                                    } else {
                                        view! {
                                            <div class="u505-match__errors">
                                                <h3 class="u505-match__section-title">"Ошибки"</h3>
                                                <div class="u505-match__error-list">
                                                    {prog.error_list.iter().map(|error| {
                                                        view! {
                                                            <div class="u505-match__error-card">
                                                                <div class="u505-match__error-title">{error.message.clone()}</div>
                                                                {error.article.as_ref().map(|article| {
                                                                    view! { <div class="u505-match__error-detail"><strong>"Артикул:"</strong> " " {article.clone()}</div> }
                                                                })}
                                                                {error.details.as_ref().map(|details| {
                                                                    view! { <div class="u505-match__error-detail">{details.clone()}</div> }
                                                                })}
                                                            </div>
                                                        }
                                                    }).collect_view()}
                                                </div>
                                            </div>
                                        }.into_any()
                                    }}
                                </section>
                            }
                        })
                    }}
                </div>
            </div>
        </PageFrame>
    }
}
