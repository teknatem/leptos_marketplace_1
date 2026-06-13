//! Страница-реестр плагинов: список всех плагинов с ключевыми данными,
//! ссылкой открытия (вкладка `plugin__<id>`) и кнопкой создания тестового плагина.

use crate::layout::global_context::AppGlobalContext;
use crate::plugins::api;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_LIST;
use contracts::plugins::PluginListItem;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn PluginList() -> impl IntoView {
    let ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    let (items, set_items) = signal(Vec::<PluginListItem>::new());
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);
    let (creating, set_creating) = signal(false);

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
                    <button
                        class="plugins-btn plugins-btn--primary"
                        on:click=create_test
                        disabled=Signal::derive(move || creating.get())
                    >
                        {move || if creating.get() {
                            "Создание…"
                        } else {
                            "＋ Создать тестовый плагин"
                        }}
                    </button>
                </div>
            </div>

            <div class="plugins-page__content">
                {move || error.get().map(|e| view! {
                    <div class="plugins-alert plugins-alert--error">{e}</div>
                })}

                {move || loading.get().then(|| view! {
                    <div class="plugins-page__state">"Загрузка…"</div>
                })}

                {move || {
                    let list = items.get();
                    if loading.get() {
                        ().into_any()
                    } else if list.is_empty() {
                        view! {
                            <div class="plugins-empty">
                                <div class="plugins-empty__icon">"🧩"</div>
                                <div class="plugins-empty__title">"Плагинов пока нет"</div>
                                <div class="plugins-empty__hint">
                                    "Нажмите «Создать тестовый плагин», чтобы добавить демонстрационный отчёт."
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
                                            <th>"Обновлён"</th>
                                            <th class="plugins-table__action-col">"Действие"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {list.into_iter().map(|p| {
                                            let key = format!("plugin__{}", p.id);
                                            let title = p.title.clone();
                                            let updated = p.updated_at.format("%Y-%m-%d %H:%M").to_string();
                                            let status = p.status.clone();
                                            let status_mod = format!("plugins-badge plugins-badge--{}", status);
                                            view! {
                                                <tr>
                                                    <td><span class="plugins-code">{p.code.clone()}</span></td>
                                                    <td class="plugins-table__title">{p.title.clone()}</td>
                                                    <td><span class="plugins-chip">{p.runtime.clone()}</span></td>
                                                    <td><span class=status_mod>{status}</span></td>
                                                    <td>
                                                        {if p.is_enabled {
                                                            view! { <span class="plugins-dot plugins-dot--on" title="включён"></span> }.into_any()
                                                        } else {
                                                            view! { <span class="plugins-dot plugins-dot--off" title="выключен"></span> }.into_any()
                                                        }}
                                                    </td>
                                                    <td class="plugins-table__muted">{updated}</td>
                                                    <td class="plugins-table__action-col">
                                                        <a
                                                            href="#"
                                                            class="plugins-link"
                                                            on:click=move |ev| {
                                                                ev.prevent_default();
                                                                ctx.open_tab(&key, &title);
                                                            }
                                                        >
                                                            "Открыть →"
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
