use chrono::Utc;
use leptos::prelude::*;
use leptos::task::spawn_local;
use contracts::projections::p905_wb_commission_history::dto::CommissionHistoryDto;

use crate::projections::p905_wb_commission_history::api;
use crate::layout::global_context::AppGlobalContext;

#[component]
pub fn CommissionHistoryList() -> impl IntoView {
    let (data, set_data) = signal(Vec::<CommissionHistoryDto>::new());
    let (total_count, set_total_count) = signal(0u64);
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);
    let (sync_status, set_sync_status) = signal(None::<String>);

    // Фильтры - период по умолчанию (последние 30 дней)
    let now = Utc::now().date_naive();
    let default_start = now - chrono::Duration::days(30);
    let default_end = now;

    let (date_from, set_date_from) = signal(default_start.format("%Y-%m-%d").to_string());
    let (date_to, set_date_to) = signal(default_end.format("%Y-%m-%d").to_string());
    let (subject_id_filter, set_subject_id_filter) = signal("".to_string());

    // Загрузка данных
    let load_data = move || {
        set_loading.set(true);
        set_error.set(None);

        let date_from_val = if date_from.get().is_empty() {
            None
        } else {
            Some(date_from.get())
        };
        let date_to_val = if date_to.get().is_empty() {
            None
        } else {
            Some(date_to.get())
        };
        let subject_id_val = if subject_id_filter.get().is_empty() {
            None
        } else {
            subject_id_filter.get().parse::<i32>().ok()
        };

        spawn_local(async move {
            match api::list_commissions(
                date_from_val,
                date_to_val,
                subject_id_val,
                Some("date".to_string()),
                Some(true),
                Some(100),
                Some(0),
            )
            .await
            {
                Ok(response) => {
                    set_data.set(response.items);
                    set_total_count.set(response.total_count);
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(format!("Failed to load data: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    // Загружаем данные при монтировании
    Effect::new(move |_| {
        load_data();
    });

    // Синхронизация с API
    let sync_with_api = move || {
        set_sync_status.set(Some("Синхронизация...".to_string()));

        spawn_local(async move {
            match api::sync_commissions().await {
                Ok(response) => {
                    set_sync_status.set(Some(response.message.clone()));
                    // Перезагрузка данных после синхронизации
                    load_data();
                }
                Err(e) => {
                    set_sync_status.set(Some(format!("Ошибка синхронизации: {}", e)));
                }
            }
        });
    };

    // Удаление записи
    let delete_commission = move |id: String| {
        spawn_local(async move {
            match api::delete_commission(&id).await {
                Ok(_) => {
                    load_data();
                }
                Err(e) => {
                    set_error.set(Some(format!("Failed to delete: {}", e)));
                }
            }
        });
    };

    let app_context = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext not found");

    // Открыть форму создания
    let create_new = move |_| {
        app_context.open_tab(
            "p905-commission-new",
            "Новая комиссия",
        );
    };

    // Открыть форму редактирования
    let edit_commission = move |id: String| {
        app_context.open_tab(
            &format!("p905-commission-{}", id),
            "Редактирование комиссии",
        );
    };

    view! {
        <div class="commission-history-list" style="padding: 20px;">
            <div style="display: flex; align-items: center; gap: 12px; margin-bottom: 20px;">
                <h2 style="margin: 0; font-size: 1.5rem; flex-grow: 1;">
                    "История комиссий Wildberries (P905)"
                    {move || {
                        let count = total_count.get();
                        if count > 0 {
                            format!(" - {} записей", count)
                        } else {
                            String::new()
                        }
                    }}
                </h2>
            </div>

            // Фильтры
            <div style="background: #f5f5f5; padding: 15px; border-radius: 8px; margin-bottom: 20px;">
                <div style="display: flex; gap: 15px; flex-wrap: wrap; align-items: center;">
                    <div>
                        <label style="display: block; font-size: 0.875rem; margin-bottom: 4px;">"Дата от:"</label>
                        <input
                            type="date"
                            prop:value=move || date_from.get()
                            on:input=move |ev| {
                                set_date_from.set(event_target_value(&ev));
                            }
                            style="padding: 6px; border-radius: 4px; border: 1px solid #ccc;"
                        />
                    </div>

                    <div>
                        <label style="display: block; font-size: 0.875rem; margin-bottom: 4px;">"Дата до:"</label>
                        <input
                            type="date"
                            prop:value=move || date_to.get()
                            on:input=move |ev| {
                                set_date_to.set(event_target_value(&ev));
                            }
                            style="padding: 6px; border-radius: 4px; border: 1px solid #ccc;"
                        />
                    </div>

                    <div>
                        <label style="display: block; font-size: 0.875rem; margin-bottom: 4px;">"Subject ID:"</label>
                        <input
                            type="text"
                            placeholder="ID категории"
                            prop:value=move || subject_id_filter.get()
                            on:input=move |ev| {
                                set_subject_id_filter.set(event_target_value(&ev));
                            }
                            style="padding: 6px; border-radius: 4px; border: 1px solid #ccc; width: 150px;"
                        />
                    </div>

                    <div style="display: flex; gap: 10px; align-items: flex-end;">
                        <button
                            on:click=move |_| load_data()
                            style="padding: 6px 16px; background: #28a745; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 500;"
                        >
                            "🔄 Обновить"
                        </button>

                        <button
                            on:click=move |_| sync_with_api()
                            style="padding: 6px 16px; background: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 500;"
                        >
                            "🔄 Синхронизировать с API"
                        </button>

                        <button
                            on:click=create_new
                            style="padding: 6px 16px; background: #17a2b8; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 500;"
                        >
                            "+ Создать"
                        </button>
                    </div>
                </div>

                {move || {
                    sync_status.get().map(|msg| {
                        view! {
                            <div style="margin-top: 10px; padding: 8px; background: #e3f2fd; border-radius: 4px; font-size: 0.875rem;">
                                {msg}
                            </div>
                        }
                    })
                }}
            </div>

            // Отображение ошибок
            {move || {
                error.get().map(|err| {
                    view! {
                        <div style="padding: 12px; background: #f8d7da; border: 1px solid #f5c6cb; border-radius: 4px; color: #721c24; margin-bottom: 15px;">
                            {err}
                        </div>
                    }
                })
            }}

            // Индикатор загрузки
            {move || {
                if loading.get() {
                    view! {
                        <div style="text-align: center; padding: 40px; color: #666;">
                            "Загрузка данных..."
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}

            // Таблица данных
            {move || {
                if !loading.get() && data.get().is_empty() {
                    view! {
                        <div style="text-align: center; padding: 40px; color: #999;">
                            "Нет данных для отображения"
                        </div>
                    }.into_any()
                } else if !loading.get() {
                    let items = data.get();
                    view! {
                        <div style="overflow-x: auto;">
                            <table style="width: 100%; border-collapse: collapse; font-size: 0.875rem; background: white;">
                                <thead>
                                    <tr style="background: #f8f9fa; border-bottom: 2px solid #dee2e6;">
                                        <th style="padding: 12px; text-align: left; font-weight: 600;">"Дата"</th>
                                        <th style="padding: 12px; text-align: left; font-weight: 600;">"Subject ID"</th>
                                        <th style="padding: 12px; text-align: left; font-weight: 600;">"Категория"</th>
                                        <th style="padding: 12px; text-align: left; font-weight: 600;">"Родительская"</th>
                                        <th style="padding: 12px; text-align: right; font-weight: 600;">"Букинг"</th>
                                        <th style="padding: 12px; text-align: right; font-weight: 600;">"Маркетплейс"</th>
                                        <th style="padding: 12px; text-align: right; font-weight: 600;">"Пикап"</th>
                                        <th style="padding: 12px; text-align: right; font-weight: 600;">"Поставщик"</th>
                                        <th style="padding: 12px; text-align: center; font-weight: 600;">"Действия"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {items.into_iter().map(|item| {
                                        let id_for_edit = item.id.clone();
                                        let id_for_delete = item.id.clone();
                                        
                                        view! {
                                            <tr style="border-bottom: 1px solid #dee2e6;">
                                                <td style="padding: 10px;">{item.date.clone()}</td>
                                                <td style="padding: 10px;">{item.subject_id}</td>
                                                <td style="padding: 10px;">{item.subject_name.clone()}</td>
                                                <td style="padding: 10px;">{item.parent_name.clone()}</td>
                                                <td style="padding: 10px; text-align: right;">{format!("{:.2}%", item.kgvp_booking)}</td>
                                                <td style="padding: 10px; text-align: right;">{format!("{:.2}%", item.kgvp_marketplace)}</td>
                                                <td style="padding: 10px; text-align: right;">{format!("{:.2}%", item.kgvp_pickup)}</td>
                                                <td style="padding: 10px; text-align: right;">{format!("{:.2}%", item.kgvp_supplier)}</td>
                                                <td style="padding: 10px; text-align: center;">
                                                    <button
                                                        on:click=move |_| {
                                                            let id = id_for_edit.clone();
                                                            edit_commission(id);
                                                        }
                                                        style="padding: 4px 10px; background: #ffc107; color: #000; border: none; border-radius: 4px; cursor: pointer; margin-right: 5px; font-size: 0.75rem;"
                                                    >
                                                        "Изменить"
                                                    </button>
                                                    <button
                                                        on:click=move |_| {
                                                            if web_sys::window()
                                                                .unwrap()
                                                                .confirm_with_message("Удалить эту запись?")
                                                                .unwrap_or(false)
                                                            {
                                                                let id = id_for_delete.clone();
                                                                delete_commission(id);
                                                            }
                                                        }
                                                        style="padding: 4px 10px; background: #dc3545; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.75rem;"
                                                    >
                                                        "Удалить"
                                                    </button>
                                                </td>
                                            </tr>
                                        }
                                    }).collect_view()}
                                </tbody>
                            </table>
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}
        </div>
    }
}

