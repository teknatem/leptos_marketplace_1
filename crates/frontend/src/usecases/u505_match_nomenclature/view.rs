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

    // Параметры запроса
    let (overwrite_existing, set_overwrite_existing) = signal(false);
    let (ignore_case, set_ignore_case) = signal(true);

    // Запустить сопоставление
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

    // Polling прогресса
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
                        Err(_e) => {
                            break;
                        }
                    }
                    // Пауза 2 секунды
                    gloo_timers::future::TimeoutFuture::new(2000).await;
                }
            });
        }
    });

    view! {
        <div id="u505_match_nomenclature--usecase" data-page-category="legacy" style="padding: 20px; border: 1px solid #ccc; border-radius: 8px; max-width: 800px; margin: 20px auto;">
            <h2>"u505: Сопоставление номенклатуры"</h2>

            <div style="margin: 20px 0;">
                <p style="color: #666;">
                    "Автоматическое сопоставление товаров маркетплейсов с номенклатурой 1С по артикулу"
                </p>
            </div>

            // Настройки
            <div style="margin: 20px 0; padding: 15px; background: #f5f5f5; border-radius: 4px;">
                <h3 style="font-weight: bold; margin-bottom: 10px;">"Параметры"</h3>
                <div style="margin: 10px 0;">
                    <label>
                        <input
                            type="checkbox"
                            prop:checked=move || ignore_case.get()
                            on:change=move |ev| { set_ignore_case.set(event_target_checked(&ev)); }
                        />
                        " Игнорировать регистр при сопоставлении"
                    </label>
                </div>
                <div style="margin: 10px 0;">
                    <label>
                        <input
                            type="checkbox"
                            prop:checked=move || overwrite_existing.get()
                            on:change=move |ev| { set_overwrite_existing.set(event_target_checked(&ev)); }
                        />
                        " Перезаписать существующие связи"
                    </label>
                </div>
            </div>

            // Кнопка запуска
            <div style="margin: 20px 0;">
                <button
                    style="padding: 10px 20px; background: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 16px;"
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

            // Сообщение об ошибке
            {move || {
                if let Some(msg) = error_message.get() {
                    view! {
                        <div style="padding: 10px; background: #fee; border: 1px solid #fcc; border-radius: 4px; color: #c00; margin: 10px 0;">
                            {msg}
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}

            // Прогресс
            {move || {
                if let Some(prog) = progress.get() {
                    view! {
                        <div style="margin-top: 20px; padding: 15px; background: #f9f9f9; border-radius: 8px; border: 1px solid #ddd;">
                            <h3>"Прогресс сопоставления"</h3>

                            <div style="margin: 10px 0;">
                                <strong>"Статус: "</strong>
                                <span style={move || format!("color: {}; font-weight: bold;",
                                    match prog.status {
                                        MatchStatus::InProgress => "#007bff",
                                        MatchStatus::Completed => "#28a745",
                                        MatchStatus::CompletedWithErrors => "#ffc107",
                                        MatchStatus::Failed => "#dc3545",
                                    }
                                )}>
                                    {match prog.status {
                                        MatchStatus::InProgress => "В процессе",
                                        MatchStatus::Completed => "Завершено",
                                        MatchStatus::CompletedWithErrors => "Завершено с ошибками",
                                        MatchStatus::Failed => "Провалено",
                                    }}
                                </span>
                            </div>

                            {prog.total.map(|total| view! {
                                <div style="margin: 10px 0;">
                                    <strong>"Прогресс: "</strong>
                                    {prog.processed} " / " {total}
                                    " (" {(prog.processed as f64 / total as f64 * 100.0).round() as i32} "%)"
                                </div>
                                <div style="background: #e0e0e0; height: 20px; border-radius: 4px; overflow: hidden; margin: 10px 0;">
                                    <div style={format!("width: {}%; height: 100%; background: #007bff; transition: width 0.3s;",
                                        (prog.processed as f64 / total as f64 * 100.0).round() as i32
                                    )}></div>
                                </div>
                            })}

                            <div style="margin: 10px 0; display: grid; grid-template-columns: 1fr 1fr; gap: 10px;">
                                <div><strong>"Сопоставлено:"</strong> {prog.matched}</div>
                                <div><strong>"Очищено:"</strong> {prog.cleared}</div>
                                <div><strong>"Пропущено:"</strong> {prog.skipped}</div>
                                <div><strong>"Неоднозначных:"</strong> {prog.ambiguous}</div>
                                <div><strong>"Ошибок:"</strong> {prog.errors}</div>
                            </div>

                            {prog.current_item.map(|item| view! {
                                <div style="margin: 10px 0; font-size: 12px; color: #666;">
                                    <strong>"Текущий элемент:"</strong> {item}
                                </div>
                            })}

                            {if !prog.error_list.is_empty() {
                                view! {
                                    <div style="margin-top: 15px;">
                                        <h4 style="color: #dc3545; font-weight: bold;">"Ошибки:"</h4>
                                        <div style="max-height: 300px; overflow-y: auto;">
                                            {prog.error_list.iter().map(|error| {
                                                view! {
                                                    <div style="margin: 5px 0; padding: 8px; background: #fee; border: 1px solid #fcc; border-radius: 4px; font-size: 12px;">
                                                        <div style="font-weight: bold;">{error.message.clone()}</div>
                                                        {error.article.as_ref().map(|art| view! {
                                                            <div style="color: #666; margin-top: 3px;">"Артикул: " {art.clone()}</div>
                                                        })}
                                                        {error.details.as_ref().map(|details| view! {
                                                            <div style="color: #666; margin-top: 3px;">{details.clone()}</div>
                                                        })}
                                                    </div>
                                                }
                                            }).collect_view()}
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }}
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}
        </div>
    }
}
