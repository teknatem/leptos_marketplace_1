//! Страница просмотра плагина — рабочая версия (рантайм).
//!
//! Открывается из меню/сайдбара по ключу `plugin__<id>`. Вкладки: «Приложение»
//! (плагин в iframe на всю ширину/высоту) и «Журнал» (события жизненного цикла +
//! серверный журнал host.log.*). Редактирование кода — на странице разработки
//! (`plugin_dev__<id>`, [`crate::plugins::PluginHost`]).

use crate::plugins::api;
use crate::plugins::frame::PluginFrame;
use contracts::plugins::{PluginDefinition, PluginRunContext};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn PluginView(plugin_id: String) -> impl IntoView {
    let (def, set_def) = signal(None::<PluginDefinition>);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);
    let client_src = RwSignal::new(String::new());
    let styles_src = RwSignal::new(String::new());
    let context = RwSignal::new(PluginRunContext::default());
    let restart = RwSignal::new(0u64);
    // Журнал — выдвижная нижняя панель поверх iframe (тоггл).
    let show_log = RwSignal::new(false);

    // Журналы плагина — наполняются PluginFrame, рендерятся в выдвижной панели.
    let console = RwSignal::new(Vec::<String>::new());
    let events = RwSignal::new(Vec::<String>::new());

    {
        let id = plugin_id.clone();
        spawn_local(async move {
            match api::get_by_id(&id).await {
                Ok(plugin) => {
                    client_src.set(plugin.bundle.client_script.clone().unwrap_or_default());
                    styles_src.set(plugin.bundle.styles.clone().unwrap_or_default());
                    context.set(crate::plugins::host::model::default_run_context(
                        &plugin.bundle,
                    ));
                    set_def.set(Some(plugin));
                    restart.update(|value| *value += 1);
                }
                Err(message) => set_error.set(Some(message)),
            }
            set_loading.set(false);
        });
    }

    let restart_plugin = move |_| {
        console.set(Vec::new());
        events.set(Vec::new());
        restart.update(|value| *value += 1);
    };

    view! {
        <div class="plugin-host plugin-host--view">
            {move || error.get().map(|message| view! {
                <div class="plugin-host__alert plugin-host__alert--error">{message}</div>
            })}

            // Узкий заголовок: Наименование · код · Restart · Журнал.
            <div class="plugin-host__bar">
                <h2
                    class="plugin-host__title"
                    title=move || def.get().and_then(|p| p.bundle.manifest.description).unwrap_or_default()
                >
                    {move || def.get().map(|p| p.bundle.manifest.title)
                        .filter(|t| !t.is_empty())
                        .unwrap_or_else(|| if loading.get() { "Загрузка…".into() } else { "Плагин".into() })}
                </h2>
                <span class="plugin-host__code">
                    {move || def.get().map(|p| p.bundle.manifest.code).unwrap_or_default()}
                </span>
                <span class="plugin-host__bar-spacer"></span>
                <button class="plugin-host__run plugin-host__run--server" on:click=restart_plugin>
                    "Restart"
                </button>
                <button
                    class="plugin-host__run plugin-host__run--server"
                    class:plugin-host__run--active=move || show_log.get()
                    on:click=move |_| show_log.update(|v| *v = !*v)
                >
                    "Журнал"
                </button>
            </div>

            // Сцена: iframe на всё место + выдвижной журнал поверх него.
            <div class="plugin-host__stage">
                <PluginFrame
                    plugin_id=plugin_id
                    client_src=client_src
                    styles_src=styles_src
                    context=context
                    restart=restart
                    console=console
                    events=events
                    dev=true
                    frameless=true
                />

                <div
                    class="plugin-host__drawer"
                    class:plugin-host__hidden=move || !show_log.get()
                >
                    <div class="plugin-host__drawer-head">
                        <span class="plugin-host__drawer-title">"Журнал"</span>
                        <button
                            class="plugin-host__console-clear"
                            on:click=move |_| { events.set(Vec::new()); console.set(Vec::new()); }
                        >
                            "Очистить"
                        </button>
                        <span class="plugin-host__bar-spacer"></span>
                        <button
                            class="plugin-host__drawer-close"
                            on:click=move |_| show_log.set(false)
                            aria-label="Закрыть журнал"
                        >
                            "×"
                        </button>
                    </div>
                    <div class="plugin-host__drawer-body">
                        {move || {
                            let event_lines = events.get();
                            let console_lines = console.get();
                            if event_lines.is_empty() && console_lines.is_empty() {
                                view! {
                                    <div class="plugin-host__state">
                                        "Журнал пуст. Взаимодействуйте с плагином — события появятся здесь."
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="plugin-host__logs">
                                        {(!event_lines.is_empty()).then(|| view! {
                                            <div class="plugin-host__console plugin-host__events">
                                                <div class="plugin-host__console-head">
                                                    <span class="plugin-host__console-title">"Журнал событий"</span>
                                                </div>
                                                <div class="plugin-host__console-body">
                                                    {event_lines.into_iter().map(|line| view! {
                                                        <div class="plugin-host__console-line">{line}</div>
                                                    }).collect_view()}
                                                </div>
                                            </div>
                                        })}
                                        {(!console_lines.is_empty()).then(|| view! {
                                            <div class="plugin-host__console">
                                                <div class="plugin-host__console-head">
                                                    <span class="plugin-host__console-title">"Серверный журнал"</span>
                                                </div>
                                                <div class="plugin-host__console-body">
                                                    {console_lines.into_iter().map(|line| view! {
                                                        <div class="plugin-host__console-line">{line}</div>
                                                    }).collect_view()}
                                                </div>
                                            </div>
                                        })}
                                    </div>
                                }.into_any()
                            }
                        }}
                    </div>
                </div>
            </div>
        </div>
    }
}
