use leptos::prelude::*;
use leptos::ev::Event;
use wasm_bindgen::JsCast;
use contracts::shared::pivot::{SavedDashboardConfigSummary, DashboardConfig};

fn event_target_value(ev: &Event) -> String {
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|input: web_sys::HtmlInputElement| input.value())
        .unwrap_or_default()
}

fn event_target_textarea_value(ev: &Event) -> String {
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
        .map(|textarea: web_sys::HtmlTextAreaElement| textarea.value())
        .unwrap_or_default()
}

#[component]
pub fn SavedConfigsList(
    /// List of saved configurations
    #[prop(into)]
    configs: Signal<Vec<SavedDashboardConfigSummary>>,
    /// Callback when a config is selected to load
    on_load: Callback<String>,
    /// Callback when a config is deleted
    on_delete: Callback<String>,
) -> impl IntoView {
    view! {
        <div class="saved-configs-list">
            <h3>"Сохраненные настройки"</h3>
            <Show
                when=move || !configs.get().is_empty()
                fallback=|| view! { <p class="empty-message">"Нет сохраненных настроек"</p> }
            >
                <div class="configs-container">
                    {move || {
                        configs
                            .get()
                            .iter()
                            .map(|cfg| {
                                let config_id = cfg.id.clone();
                                let config_id_for_delete = cfg.id.clone();
                                let config_name = cfg.name.clone();
                                let config_updated_at = cfg.updated_at.clone();
                                let config_description = cfg.description.clone();
                                view! {
                                    <div class="config-item">
                                        <div class="config-info">
                                            <strong>{config_name}</strong>
                                            {config_description
                                                .map(|desc| {
                                                    view! { <p class="config-description">{desc}</p> }
                                                })}
                                            <small class="config-meta">
                                                "Обновлено: " {config_updated_at}
                                            </small>
                                        </div>
                                        <div class="config-actions">
                                            <button
                                                class="btn btn-sm btn-primary"
                                                on:click=move |_| {
                                                    on_load.run(config_id.clone());
                                                }
                                            >
                                                "Загрузить"
                                            </button>
                                            <button
                                                class="btn btn-sm btn-danger"
                                                on:click=move |_| {
                                                    on_delete.run(config_id_for_delete.clone());
                                                }
                                            >
                                                "Удалить"
                                            </button>
                                        </div>
                                    </div>
                                }
                            })
                            .collect_view()
                    }}
                </div>
            </Show>
        </div>
    }
}

#[component]
pub fn SaveConfigDialog(
    /// Show/hide the dialog
    #[prop(into)]
    show: Signal<bool>,
    /// Current configuration to save
    _config: DashboardConfig,
    /// Callback when save is clicked
    on_save: Callback<(String, Option<String>)>,
    /// Callback when cancel is clicked
    on_cancel: Callback<()>,
) -> impl IntoView {
    let (name, set_name) = create_signal(String::new());
    let (description, set_description) = create_signal(String::new());

    let handle_save = move |_| {
        let name_val = name.get();
        let desc_val = description.get();
        if !name_val.is_empty() {
            let desc_opt = if desc_val.is_empty() {
                None
            } else {
                Some(desc_val)
            };
            on_save.run((name_val, desc_opt));
            set_name.set(String::new());
            set_description.set(String::new());
        }
    };

    view! {
        <Show when=move || show.get() fallback=|| view! {}>
            <div class="modal-overlay">
                <div class="modal-dialog">
                    <h3>"Сохранить настройки"</h3>
                    <div class="form-group">
                        <label>"Название"</label>
                        <input
                            type="text"
                            class="form-control"
                            prop:value=name
                            on:input=move |ev| {
                                set_name.set(event_target_value(&ev));
                            }
                        />
                    </div>
                    <div class="form-group">
                        <label>"Описание (опционально)"</label>
                        <textarea
                            class="form-control"
                            prop:value=description
                            on:input=move |ev| {
                                set_description.set(event_target_textarea_value(&ev));
                            }
                        />
                    </div>
                    <div class="modal-actions">
                        <button class="btn btn-primary" on:click=handle_save>
                            "Сохранить"
                        </button>
                        <button
                            class="btn btn-secondary"
                            on:click=move |_| {
                                on_cancel.run(());
                            }
                        >
                            "Отмена"
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
