//! LLM Connection Details - View Component
//!
//! Main form component for creating/editing LLM connections

use super::model::{fetch_connection, fetch_models_from_api, save_connection, test_connection};
use super::view_model::LlmConnectionDetailsVm;
use crate::shared::icons::icon;
use leptos::prelude::*;
use thaw::*;

#[component]
#[allow(non_snake_case)]
pub fn LlmConnectionDetails(
    id: Signal<Option<String>>,
    on_saved: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let vm = LlmConnectionDetailsVm::new();

    // Load connection data when editing
    Effect::new(move |_| {
        if let Some(conn_id) = id.get() {
            wasm_bindgen_futures::spawn_local(async move {
                match fetch_connection(&conn_id).await {
                    Ok(connection) => {
                        // Считываем allowed_models ДО перемещения полей (метод берёт &self).
                        vm.allowed_models.set(connection.allowed_models_list());
                        vm.code.set(connection.base.code);
                        vm.description.set(connection.base.description);
                        vm.comment.set(connection.base.comment.unwrap_or_default());
                        vm.provider_type
                            .set(connection.provider_type.as_str().to_string());
                        vm.api_endpoint.set(connection.api_endpoint);
                        vm.api_key.set(connection.api_key);
                        vm.model_name.set(connection.model_name);
                        vm.temperature.set(connection.temperature.to_string());
                        vm.max_tokens.set(connection.max_tokens.to_string());
                        vm.system_prompt
                            .set(connection.system_prompt.unwrap_or_default());
                        vm.agent_type.set(connection.agent_type.as_str().to_string());
                        vm.is_primary.set(connection.is_primary);

                        // Load available models if present
                        if let Some(models_json) = connection.available_models {
                            if let Ok(models) =
                                serde_json::from_str::<Vec<serde_json::Value>>(&models_json)
                            {
                                vm.set_available_models.set(models);
                            }
                        }
                    }
                    Err(e) => vm.set_error.set(Some(e)),
                }
            });
        }
    });

    // Save handler
    let handle_save = move |_| {
        let id_value = id.get();
        let dto = vm.build_save_dto(id_value);

        wasm_bindgen_futures::spawn_local(async move {
            match save_connection(dto).await {
                Ok(_) => on_saved.run(()),
                Err(e) => vm.set_error.set(Some(e)),
            }
        });
    };

    // Test connection handler
    let handle_test = move |_| {
        let id_value = match id.get() {
            Some(v) => v,
            None => {
                vm.set_test_result.set(Some((
                    false,
                    "Сохраните подключение перед тестированием".to_string(),
                )));
                return;
            }
        };

        vm.set_is_testing.set(true);
        vm.set_test_result.set(None);

        wasm_bindgen_futures::spawn_local(async move {
            match test_connection(&id_value).await {
                Ok(result) => {
                    vm.set_test_result
                        .set(Some((result.success, result.message)));
                    vm.set_is_testing.set(false);
                }
                Err(e) => {
                    vm.set_test_result
                        .set(Some((false, format!("Ошибка: {}", e))));
                    vm.set_is_testing.set(false);
                }
            }
        });
    };

    // Fetch models handler
    let handle_fetch_models = move |_| {
        let id_value = match id.get() {
            Some(v) => v,
            None => {
                vm.set_fetch_models_result.set(Some((
                    false,
                    "Сохраните подключение перед загрузкой моделей".to_string(),
                )));
                return;
            }
        };

        vm.set_is_fetching_models.set(true);
        vm.set_fetch_models_result.set(None);

        wasm_bindgen_futures::spawn_local(async move {
            match fetch_models_from_api(&id_value).await {
                Ok(response) => {
                    if response.success {
                        vm.set_available_models.set(response.models);
                        vm.set_fetch_models_result.set(Some((
                            true,
                            format!("Загружено {} моделей", response.count),
                        )));
                        vm.is_models_dropdown_open.set(true);
                    } else {
                        vm.set_fetch_models_result
                            .set(Some((false, response.message)));
                    }
                    vm.set_is_fetching_models.set(false);
                }
                Err(e) => {
                    vm.set_fetch_models_result
                        .set(Some((false, format!("Ошибка: {}", e))));
                    vm.set_is_fetching_models.set(false);
                }
            }
        });
    };

    let is_edit_mode = Signal::derive(move || id.get().is_some());

    view! {
        <div class="details-form" style="padding: 20px;">
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center style="margin-bottom: 20px;">
                <h2 style="font-size: 20px; font-weight: bold;">
                    {move || if is_edit_mode.get() { "Редактирование подключения LLM" } else { "Новое подключение LLM" }}
                </h2>
                <Space>
                    <Button appearance=ButtonAppearance::Primary on_click=handle_save>
                        {icon("save")}
                        " Сохранить"
                    </Button>
                    <Show when=move || is_edit_mode.get()>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            on_click=handle_test
                            disabled=vm.is_testing
                        >
                            {icon("link")}
                            {move || if vm.is_testing.get() { " Тестирование..." } else { " Тест подключения" }}
                        </Button>
                    </Show>
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| on_cancel.run(())>
                        {icon("close")}
                        " Отмена"
                    </Button>
                </Space>
            </Flex>

            {move || {
                vm.error
                    .get()
                    .map(|e| {
                        view! {
                            <div style="padding: 12px; margin-bottom: 16px; background: var(--color-error-50); border: 1px solid var(--color-error-100); border-radius: 8px;">
                                <span style="color: var(--color-error);">{e}</span>
                            </div>
                        }
                    })
            }}

            {move || {
                vm.test_result
                    .get()
                    .map(|(success, message)| {
                        let bg = if success { "var(--color-success-50)" } else { "var(--color-error-50)" };
                        let border = if success { "var(--color-success-100)" } else { "var(--color-error-100)" };
                        let color = if success { "var(--color-success)" } else { "var(--color-error)" };
                        view! {
                            <div style=format!(
                                "padding: 12px; margin-bottom: 16px; background: {}; border: 1px solid {}; border-radius: 8px;",
                                bg, border,
                            )>
                                <span style=format!("color: {};", color)>{message}</span>
                            </div>
                        }
                    })
            }}

            {move || {
                vm.fetch_models_result
                    .get()
                    .map(|(success, message)| {
                        let bg = if success { "var(--color-success-50)" } else { "var(--color-error-50)" };
                        let border = if success { "var(--color-success-100)" } else { "var(--color-error-100)" };
                        let color = if success { "var(--color-success)" } else { "var(--color-error)" };
                        view! {
                            <div style=format!(
                                "padding: 12px; margin-bottom: 16px; background: {}; border: 1px solid {}; border-radius: 8px;",
                                bg, border,
                            )>
                                <span style=format!("color: {};", color)>{message}</span>
                            </div>
                        }
                    })
            }}

            <div style="display: grid; grid-template-columns: 500px 500px; gap: var(--spacing-md); max-width: 1050px; align-items: start; align-content: start;">
                <Card>
                    <div class="form__group">
                        <label class="form__label">"Код"</label>
                        <Input value=vm.code placeholder="OPENROUTER-MAIN" />
                    </div>

                    <div class="form__group">
                        <label class="form__label">
                            "Наименование"
                            <span style="color: red;">"*"</span>
                        </label>
                        <Input value=vm.description placeholder="OpenRouter основное подключение" />
                    </div>

                    <div class="form__group">
                        <label class="form__label">"Провайдер"</label>
                        <select
                            style="height: 32px; padding: 4px 8px; border: 1px solid var(--colorNeutralStroke2); border-radius: 6px; width: 100%; background: var(--color-surface); color: var(--color-text);"
                            prop:value=move || vm.provider_type.get()
                            on:change=move |ev| {
                                let provider = event_target_value(&ev);
                                let previous_endpoint = vm.api_endpoint.get();
                                let previous_model = vm.model_name.get();
                                vm.provider_type.set(provider.clone());

                                if provider == "OpenRouter" {
                                    vm.api_endpoint.set("https://openrouter.ai/api/v1".to_string());
                                    if previous_model.trim().is_empty() || previous_model == "gpt-4o" {
                                        vm.model_name.set("openai/gpt-4o".to_string());
                                    }
                                } else if provider == "OpenAI" {
                                    if previous_endpoint.trim().is_empty()
                                        || previous_endpoint == "https://openrouter.ai/api/v1"
                                    {
                                        vm.api_endpoint.set("https://api.openai.com/v1".to_string());
                                    }
                                    if previous_model.trim().is_empty() || previous_model == "openai/gpt-4o" {
                                        vm.model_name.set("gpt-4o".to_string());
                                    }
                                }
                            }
                        >
                            <option value="OpenAI">"OpenAI"</option>
                            <option value="OpenRouter">"OpenRouter"</option>
                        </select>
                        <div style="font-size: 12px; color: var(--colorNeutralForeground3);">
                            "Реально поддерживаются OpenAI и OpenRouter (OpenAI-совместимый роутинг)."
                        </div>
                    </div>

                    <div class="form__group">
                        <label class="form__label">"Роль (персона)"</label>
                        <select
                            style="height: 32px; padding: 4px 8px; border: 1px solid var(--colorNeutralStroke2); border-radius: 6px; width: 100%; background: var(--color-surface); color: var(--color-text);"
                            prop:value=move || vm.agent_type.get()
                            on:change=move |ev| {
                                vm.agent_type.set(event_target_value(&ev));
                            }
                        >
                            <option value="business_analyst">"Бизнес-аналитик"</option>
                            <option value="general">"Общий (все инструменты)"</option>
                            <option value="plugin_admin">"Разработчик плагинов"</option>
                            <option value="system_admin">"Системный администратор"</option>
                            <option value="kb_admin">"Администратор базы знаний"</option>
                        </select>
                        <div style="font-size: 12px; color: var(--colorNeutralForeground3);">
                            "Определяет набор навыков/инструментов чата."
                        </div>
                    </div>

                    <div class="form__group">
                        <label class="form__label">"API Endpoint"</label>
                        <Input value=vm.api_endpoint placeholder="https://openrouter.ai/api/v1" />
                    </div>

                    <div class="form__group">
                        <label class="form__label">"Комментарий"</label>
                        <Textarea
                            value=vm.comment
                            placeholder="Используется для аналитики продаж"
                        />
                    </div>

                </Card>
                <Card>
                    <div class="form__group">
                        <label class="form__label">
                            "API Ключ"
                            <span style="color: red;">"*"</span>
                        </label>
                        <Input value=vm.api_key placeholder="sk-..." />
                    </div>

                    <div class="form__group">
                        <label class="form__label">"Модель по умолчанию"</label>
                        <div style="position: relative;">
                            <Input
                                value=vm.model_name
                                placeholder="openai/gpt-4o"
                                attr:style="width: 100%; padding-right: 0px;"
                            >
                                <InputSuffix slot>
                                    <div style="display: flex; gap: 0px;">
                                        <Show when=move || is_edit_mode.get()>
                                            <Button
                                                appearance=ButtonAppearance::Subtle
                                                shape=ButtonShape::Square
                                                size=ButtonSize::Small
                                                on_click=handle_fetch_models
                                                disabled=vm.is_fetching_models
                                                attr:style="width: 28px; height: 28px; min-width: 28px; padding: 0; display: flex; align-items: center; justify-content: center;"
                                                attr:title="Загрузить модели из API"
                                            >
                                                {move || if vm.is_fetching_models.get() { "⏳" } else { "⬇" }}
                                            </Button>
                                            <Show when=move || !vm.available_models.get().is_empty()>
                                                <Button
                                                    appearance=ButtonAppearance::Subtle
                                                    shape=ButtonShape::Square
                                                    size=ButtonSize::Small
                                                    on_click=move |_| {
                                                        let is_open_val = vm.is_models_dropdown_open.get();
                                                        vm.is_models_dropdown_open.set(!is_open_val);
                                                    }
                                                    attr:style="width: 28px; height: 28px; min-width: 28px; padding: 0; display: flex; align-items: center; justify-content: center;"
                                                    attr:title="Выбрать из списка"
                                                >
                                                    "▼"
                                                </Button>
                                            </Show>
                                        </Show>
                                    </div>
                                </InputSuffix>
                            </Input>

                            {move || {
                                if !vm.is_models_dropdown_open.get()
                                    || vm.available_models.get().is_empty()
                                {
                                    return view! { <></> }.into_any();
                                }
                                let current = vm.model_name.get().to_lowercase();
                                let opts = vm
                                    .available_models
                                    .get()
                                    .into_iter()
                                    .filter_map(|m| {
                                        m.get("id").and_then(|v| v.as_str()).map(|s| s.to_string())
                                    })
                                    .filter(|model_id| {
                                        if current.trim().is_empty() {
                                            true
                                        } else {
                                            model_id.to_lowercase().contains(&current)
                                        }
                                    })
                                    .take(50)
                                    .collect::<Vec<_>>();
                                view! {
                                    <div style="position: absolute; top: calc(100% + 4px); left: 0; right: 0; max-height: 220px; overflow-y: auto; background: var(--color-surface); border: 1px solid var(--color-border); border-radius: var(--radius-md); box-shadow: var(--shadow-md); z-index: 1000;">
                                        {if opts.is_empty() {
                                            view! {
                                                <div style="padding: 8px 12px; color: var(--color-text-tertiary);">
                                                    "Нет совпадений"
                                                </div>
                                            }
                                                .into_any()
                                        } else {
                                            opts.into_iter()
                                                .map(|opt| {
                                                    let opt2 = opt.clone();
                                                    view! {
                                                        <div
                                                            style="padding: 8px 12px; cursor: pointer; border-bottom: 1px solid var(--color-border-light);"
                                                            on:mousedown=move |_| {
                                                                vm.model_name.set(opt2.clone());
                                                                vm.is_models_dropdown_open.set(false);
                                                            }
                                                        >
                                                            {opt}
                                                        </div>
                                                    }
                                                })
                                                .collect_view()
                                                .into_any()
                                        }}
                                    </div>
                                }
                                    .into_any()
                            }}

                        </div>
                        <div style="font-size: 12px; color: var(--colorNeutralForeground3);">
                            "Должна входить в список разрешённых моделей ниже."
                        </div>
                    </div>

                    <div class="form__group">
                        <label class="form__label">"Temperature (0.0-2.0)"</label>
                        <Input value=vm.temperature placeholder="0.7" />
                    </div>

                    <div class="form__group">
                        <label class="form__label">"Max Tokens"</label>
                        <Input value=vm.max_tokens placeholder="4096" />
                    </div>

                    <div class="form__group">
                        <label class="form__label">"Системный промпт"</label>
                        <Textarea
                            attr:style="min-height : 80px"
                            value=vm.system_prompt
                            placeholder="Ты аналитик данных маркетплейсов..."
                        />
                    </div>

                    <div style="display: flex; align-items: center; gap: 8px;">
                        <input
                            type="checkbox"
                            prop:checked=move || vm.is_primary.get()
                            on:change=move |ev| {
                                let checked = event_target_checked(&ev);
                                vm.is_primary.set(checked);
                            }
                        />
                        <span>"Использовать как основное подключение"</span>
                    </div>

                </Card>

            </div>

            // ── Разрешённые модели (курируемый короткий список) ───────────────────
            <Card>
                <div class="form__group">
                    <label class="form__label">"Разрешённые модели"</label>
                    <div style="font-size: 12px; color: var(--colorNeutralForeground3); margin-bottom: 8px;">
                        "Отметьте 3-8 технически совместимых и целесообразных моделей. Только они будут доступны для выбора в чате. Сначала загрузите каталог кнопкой ⬇ у поля модели."
                    </div>
                    {move || {
                        let available: Vec<String> = vm
                            .available_models
                            .get()
                            .into_iter()
                            .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
                            .collect();
                        // Показать также уже отмеченные модели, которых нет в каталоге (не потерять их).
                        let mut ids = available.clone();
                        for a in vm.allowed_models.get() {
                            if !ids.contains(&a) {
                                ids.push(a);
                            }
                        }
                        if ids.is_empty() {
                            return view! {
                                <div style="color: var(--color-text-tertiary); padding: 8px 0;">
                                    "Каталог моделей пуст — загрузите его (⬇), затем отметьте разрешённые."
                                </div>
                            }
                            .into_any();
                        }
                        ids.sort();
                        view! {
                            <div style="display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 4px 16px; max-height: 260px; overflow-y: auto; padding: 8px; border: 1px solid var(--colorNeutralStroke2); border-radius: 6px;">
                                {ids.into_iter().map(|model_id| {
                                    let mid_for_check = model_id.clone();
                                    let mid_for_toggle = model_id.clone();
                                    view! {
                                        <label style="display: flex; align-items: center; gap: 6px; cursor: pointer; font-size: 13px;">
                                            <input
                                                type="checkbox"
                                                prop:checked=move || vm.allowed_models.get().contains(&mid_for_check)
                                                on:change=move |_| vm.toggle_allowed(&mid_for_toggle)
                                            />
                                            <span>{model_id}</span>
                                        </label>
                                    }
                                }).collect_view()}
                            </div>
                        }
                        .into_any()
                    }}
                </div>
            </Card>
        </div>
    }
}
