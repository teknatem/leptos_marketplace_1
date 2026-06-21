//! Страница-реестр плагинов: список всех плагинов с ключевыми данными,
//! ссылкой открытия (вкладка `plugin__<id>`) и кнопкой создания JS-примера.

use crate::layout::global_context::AppGlobalContext;
use crate::plugins::api;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_LIST;
use contracts::plugins::{PluginHealth, PluginListItem, PluginRunBrief};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;
use wasm_bindgen::JsCast;

/// Подпись и CSS-модификатор бейджа «здоровья» плагина.
fn health_badge(health: PluginHealth) -> (&'static str, &'static str) {
    match health {
        PluginHealth::Ok => ("OK", "ok"),
        PluginHealth::Warn => ("Внимание", "warn"),
        PluginHealth::Crit => ("Критично", "crit"),
        PluginHealth::NoData => ("—", "nodata"),
    }
}

#[component]
pub fn PluginList() -> impl IntoView {
    let ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    let (items, set_items) = signal(Vec::<PluginListItem>::new());
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);
    let (creating, set_creating) = signal(false);
    let (import_msg, set_import_msg) = signal(None::<String>);
    let (summaries, set_summaries) = signal(HashMap::<String, PluginRunBrief>::new());

    let reload = move || {
        set_loading.set(true);
        set_error.set(None);
        spawn_local(async move {
            match api::list_all().await {
                Ok(list) => set_items.set(list),
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
        // Сводки запусков подгружаем параллельно; их отсутствие не критично.
        spawn_local(async move {
            if let Ok(briefs) = api::runs_summary(7).await {
                set_summaries.set(
                    briefs
                        .into_iter()
                        .map(|b| (b.plugin_id.clone(), b))
                        .collect(),
                );
            }
        });
    };

    // Первичная загрузка.
    reload();

    let create_test = move |_| {
        set_creating.set(true);
        spawn_local(async move {
            match api::insert_test_data().await {
                Ok(()) => {
                    set_creating.set(false);
                    reload();
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_creating.set(false);
                }
            }
        });
    };

    let handle_import = move |ev: web_sys::Event| {
        let Some(input) = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        else {
            return;
        };
        let Some(file) = input.files().and_then(|files| files.get(0)) else {
            return;
        };
        // Сброс значения, чтобы повторный выбор того же файла снова сработал.
        input.set_value("");
        set_error.set(None);
        set_import_msg.set(Some("Импорт…".to_string()));
        spawn_local(async move {
            let buffer = match wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await {
                Ok(buffer) => buffer,
                Err(e) => {
                    set_error.set(Some(format!("Не удалось прочитать файл: {e:?}")));
                    set_import_msg.set(None);
                    return;
                }
            };
            let array = js_sys::Uint8Array::new(&buffer);
            let mut bytes = vec![0u8; array.length() as usize];
            array.copy_to(&mut bytes);

            match api::import_archive(bytes).await {
                Ok(body) => {
                    let code = body
                        .get("code")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if body.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                        set_import_msg.set(Some(format!("Импортирован «{code}» (статус draft)")));
                        reload();
                    } else {
                        let errors = body
                            .get("validate")
                            .and_then(|v| v.get("errors"))
                            .map(|e| e.to_string())
                            .unwrap_or_default();
                        set_error.set(Some(format!("«{code}» не прошёл валидацию: {errors}")));
                        set_import_msg.set(None);
                    }
                }
                Err(message) => {
                    set_error.set(Some(message));
                    set_import_msg.set(None);
                }
            }
        });
    };

    view! {
        <PageFrame page_id="plugins--list" category=PAGE_CAT_LIST class="plugins-page">
            <div class="plugins-page__header">
                <div class="plugins-page__heading">
                    <h1 class="plugins-page__title">"Плагины"</h1>
                    <p class="plugins-page__subtitle">
                        "Реестр расширений платформы. "
                        {move || {
                            let n = items.get().len();
                            format!("Всего: {}", n)
                        }}
                    </p>
                </div>
                <div class="plugins-page__actions">
                    <button class="plugins-btn plugins-btn--ghost" on:click=move |_| reload()>
                        "Обновить"
                    </button>
                    <label class="plugins-btn plugins-btn--ghost plugins-import">
                        "⭳ Импорт .zip"
                        <input
                            type="file"
                            accept=".zip,application/zip"
                            class="plugins-import__input"
                            on:change=handle_import
                        />
                    </label>
                    <button
                        class="plugins-btn plugins-btn--primary"
                        on:click=create_test
                        disabled=Signal::derive(move || creating.get())
                    >
                        {move || if creating.get() {
                            "Создание…"
                        } else {
                            "＋ Создать пример JS-плагина"
                        }}
                    </button>
                </div>
            </div>

            <div class="plugins-page__content">
                {move || error.get().map(|e| view! {
                    <div class="plugins-alert plugins-alert--error">{e}</div>
                })}

                {move || import_msg.get().map(|m| view! {
                    <div class="plugins-alert plugins-alert--info">{m}</div>
                })}

                {move || loading.get().then(|| view! {
                    <div class="plugins-page__state">"Загрузка…"</div>
                })}

                {move || {
                    let list = items.get();
                    let sums = summaries.get();
                    if loading.get() {
                        ().into_any()
                    } else if list.is_empty() {
                        view! {
                            <div class="plugins-empty">
                                <div class="plugins-empty__icon">"🧩"</div>
                                <div class="plugins-empty__title">"Плагинов пока нет"</div>
                                <div class="plugins-empty__hint">
                                    "Нажмите «Создать пример JS-плагина», чтобы добавить демонстрационный отчёт."
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="plugins-table-wrap">
                                <table class="plugins-table">
                                    <thead>
                                        <tr>
                                            <th>"Код"</th>
                                            <th>"Название"</th>
                                            <th>"Исполнение"</th>
                                            <th>"Статус"</th>
                                            <th>"Вкл."</th>
                                            <th>"Здоровье (7д)"</th>
                                            <th>"Обновлён"</th>
                                            <th class="plugins-table__action-col">"Действие"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {list.into_iter().map(|p| {
                                            // Название → пользовательская страница плагина (`plugin__<id>`).
                                            let view_key = format!("plugin__{}", p.id);
                                            let view_title = p.title.clone();
                                            // «Изменить» → страница редактирования (`plugin_dev__<id>`).
                                            let edit_key = format!("plugin_dev__{}", p.id);
                                            let edit_title = format!("{} — разработка", p.title);
                                            let updated = p.updated_at.format("%Y-%m-%d %H:%M").to_string();
                                            let status = p.status.clone();
                                            let status_mod = format!("plugins-badge plugins-badge--{}", status);
                                            let brief = sums.get(&p.id).cloned();
                                            let (health_label, health_mod) = brief
                                                .as_ref()
                                                .map(|b| health_badge(b.health))
                                                .unwrap_or(("—", "nodata"));
                                            let health_title = brief
                                                .as_ref()
                                                .map(|b| format!("{} запусков, ошибок {:.0}%", b.runs, b.error_rate * 100.0))
                                                .unwrap_or_default();
                                            view! {
                                                <tr>
                                                    <td><span class="plugins-code">{p.code.clone()}</span></td>
                                                    <td class="plugins-table__title">
                                                        <a
                                                            href="#"
                                                            class="plugins-link"
                                                            on:click=move |ev| {
                                                                ev.prevent_default();
                                                                ctx.open_tab(&view_key, &view_title);
                                                            }
                                                        >
                                                            {p.title.clone()}
                                                        </a>
                                                    </td>
                                                    <td><span class="plugins-chip">{p.runtime.clone()}</span></td>
                                                    <td><span class=status_mod>{status}</span></td>
                                                    <td>
                                                        {if p.is_enabled {
                                                            view! { <span class="plugins-dot plugins-dot--on" title="включён"></span> }.into_any()
                                                        } else {
                                                            view! { <span class="plugins-dot plugins-dot--off" title="выключен"></span> }.into_any()
                                                        }}
                                                    </td>
                                                    <td>
                                                        <span
                                                            class=format!("plugins-health plugins-health--{health_mod}")
                                                            title=health_title
                                                        >
                                                            {health_label}
                                                        </span>
                                                    </td>
                                                    <td class="plugins-table__muted">{updated}</td>
                                                    <td class="plugins-table__action-col">
                                                        <a
                                                            href="#"
                                                            class="plugins-link"
                                                            on:click=move |ev| {
                                                                ev.prevent_default();
                                                                ctx.open_tab(&edit_key, &edit_title);
                                                            }
                                                        >
                                                            "Изменить"
                                                        </a>
                                                    </td>
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
        </PageFrame>
    }
}
