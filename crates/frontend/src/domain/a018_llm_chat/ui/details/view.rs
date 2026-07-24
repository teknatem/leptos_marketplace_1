//! LLM Chat Details - View Component
//!
//! Унифицирован с detail-страницами: PageFrame, page__header, page__content.
//! Агент отображается по имени (agent_name из API), а не по UUID.

use super::artifact_card::ArtifactCard;
use super::model::{
    cancel_job, delete_chat, fetch_chat, fetch_chat_context, fetch_connection_allowed_models,
    fetch_messages, poll_until_done, send_message, set_rating, JobProgress, PollOutcome,
};

/// Предопределённое сообщение для кнопки «Диагностика»: модель разбирает текущий диалог
/// текстом, без вызова инструментов. Комментарий пользователя (если есть) дописывается следом.
const DIAGNOSTIC_PROMPT: &str = "Проведи диагностику этого диалога. Только ТЕКСТОВЫЙ разбор — \
НЕ вызывай инструменты и ничего не создавай/не пересоздавай.\n\nПроанализируй:\n\
1) что хотел пользователь;\n2) какие шаги и инструменты выполнялись, какие ошибки встречались;\n\
3) что не получилось и почему (корневая причина);\n4) конкретные следующие шаги для решения.\n\n\
Ответь кратко и по делу на русском.";
use super::tool_calls_trace::ToolCallsTrace;
use super::view_model::LlmChatDetailsVm;
use crate::domain::a018_llm_chat::ui::pending_first_message_key;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::date_utils::{format_datetime_utc_local, format_utc_local};
use crate::shared::icons::icon;
use crate::shared::knowledge_base::links::KbLinkedText;
use crate::shared::markdown::Markdown;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_DETAIL;
use crate::shared::speech::{DictationButton, DictationDiagnostics};
use contracts::domain::a018_llm_chat::aggregate::{ChatRole, LlmChatMessage};
use contracts::domain::a018_llm_chat::context::ContextPackageSummary;
use contracts::domain::common::AggregateId;
use leptos::prelude::*;
use thaw::*;
use uuid::Uuid;

/// Один элемент ленты чата: либо сообщение, либо событие прикрепления контекста.
/// Оба сортируются по времени создания для хронологического показа.
enum FeedItem {
    Message(LlmChatMessage),
    Context(ContextPackageSummary),
}

struct FeedRow {
    ts: chrono::DateTime<chrono::Utc>,
    key: String,
    item: FeedItem,
}

/// Левый «жёлоб» строки ленты: аватар блока, имя автора и время (до секунд),
/// выровненные по левой границе блока.
#[allow(non_snake_case)]
fn FeedGutter(avatar: &'static str, author: &'static str, time: String) -> impl IntoView {
    view! {
        <div style="flex: 0 0 104px; display: flex; align-items: flex-start; gap: 8px;">
            <div style="flex: 0 0 auto; line-height: 0; margin-top: 1px;">{icon(avatar)}</div>
            <div style="display: flex; flex-direction: column; gap: 2px; text-align: left; min-width: 0;">
                <div style="font-size: 11px; font-weight: 600; letter-spacing: .02em; opacity: 0.6;">
                    {author}
                </div>
                <div style="font-size: 11px; opacity: 0.45; font-variant-numeric: tabular-nums;">
                    {time}
                </div>
            </div>
        </div>
    }
}

/// Строка сообщения чата (пользователь / ассистент).
#[allow(non_snake_case)]
fn MessageRow(msg: LlmChatMessage) -> impl IntoView {
    let is_user = matches!(msg.role, ChatRole::User);
    let tokens = msg.tokens_used;
    let model = msg.model_name.clone();
    let conf = msg.confidence;
    let duration = msg.duration_ms;
    let intent = msg.intent.clone();
    let artifact_id = msg.artifact_id.as_ref().map(|id| id.as_string());
    let tool_trace = msg.tool_trace.clone();
    let message_id = msg.id.to_string();
    let content = msg.content.clone();
    let time = format_utc_local(&msg.created_at, "%d.%m %H:%M:%S");
    let (avatar, author) = if is_user {
        ("avatar-user", "ВЫ")
    } else {
        ("avatar-assistant", "АССИСТЕНТ")
    };
    // Нейтральные оттенки серого вместо синеватого фона: пользователь — чуть
    // темнее (Background3), ассистент — базовый фон (Background1).
    let row_style = if is_user {
        "width: 100%; padding: 12px 16px; background: var(--colorNeutralBackground3);"
    } else {
        "width: 100%; padding: 12px 16px; background: var(--colorNeutralBackground1);"
    };
    view! {
        <div style=row_style>
            <div style="max-width: 980px; margin: 0 auto; display: flex; gap: 16px;">
                {FeedGutter(avatar, author, time)}
                <div style="flex: 1; min-width: 0;">
                    {if is_user {
                        view! { <KbLinkedText text=content /> }.into_any()
                    } else {
                        view! { <Markdown text=content /> }.into_any()
                    }}
                    {move || {
                        let mut meta_parts = Vec::new();
                        if let Some(i) = &intent {
                            let label = match i.as_str() {
                                "func_help" => "🧭 функционал",
                                "data_query" => "📊 данные",
                                "bi_authoring" => "📈 BI-сборка",
                                "plugin_dev" => "🧩 плагин",
                                "sys_admin" => "🛠 система",
                                "kb_curation" => "📚 база знаний",
                                "meta_smalltalk" => "💬 диалог",
                                other => other,
                            };
                            meta_parts.push(label.to_string());
                        }
                        if let Some(t) = tokens {
                            meta_parts.push(format!("🎫 {} tokens", t));
                        }
                        if let Some(m) = &model {
                            meta_parts.push(format!("🤖 {}", m));
                        }
                        if let Some(d) = duration {
                            meta_parts.push(format!("⏱ {:.1}s", d as f64 / 1000.0));
                        }
                        if let Some(c) = conf {
                            meta_parts.push(format!("📊 {:.1}%", c * 100.0));
                        }
                        if !meta_parts.is_empty() {
                            Some(
                                view! {
                                    <div style="font-size: 11px; opacity: 0.7; margin-top: 6px;">
                                        {meta_parts.join(" • ")}
                                    </div>
                                },
                            )
                        } else {
                            None
                        }
                    }}

                    {move || {
                        if !is_user {
                            Some(view! { <ToolCallsTrace tool_trace=tool_trace.clone() message_id=message_id.clone() /> })
                        } else {
                            None
                        }
                    }}
                    {move || {
                        artifact_id
                            .clone()
                            .map(|id| view! { <ArtifactCard artifact_id=id /> })
                    }}
                </div>
            </div>
        </div>
    }
}

/// Строка-событие: к чату прикреплён пакет контекста страницы. Ссылка ведёт на
/// details-страницу контекста (тот же дизайн, что был у чипа контекста).
#[allow(non_snake_case)]
fn ContextRow(p: ContextPackageSummary, nav_ctx: Option<AppGlobalContext>) -> impl IntoView {
    let time = format_datetime_utc_local(&p.created_at, "%d.%m %H:%M:%S");
    let tab_key = format!("a018_llm_context_details_{}", p.id);
    let title = p.title.clone();
    let link_title = p.title.clone();
    let page_key = p.page_key.clone();
    view! {
        <div style="width: 100%; padding: 10px 16px; background: var(--colorNeutralBackground2);">
            <div style="max-width: 980px; margin: 0 auto; display: flex; gap: 16px;">
                {FeedGutter("avatar-context", "КОНТЕКСТ", time)}
                <div style="flex: 1; min-width: 0; display: flex; align-items: center; flex-wrap: wrap; gap: 6px;">
                    <span style="opacity: 0.7; font-size: 13px;">"Добавлен документ в контекст:"</span>
                    <a
                        href="#"
                        title=page_key
                        style="display: inline-flex; align-items: center; gap: 6px; \
                               color: var(--colorBrandForeground1); text-decoration: none; \
                               cursor: pointer; font-size: 13px;"
                        on:click=move |e| {
                            e.prevent_default();
                            if let Some(c) = nav_ctx {
                                c.open_tab(&tab_key, &format!("Контекст: {}", title));
                            }
                        }
                    >
                        {icon("paperclip")}
                        {format!(" {}", link_title)}
                    </a>
                </div>
            </div>
        </div>
    }
}

#[component]
#[allow(non_snake_case)]
pub fn LlmChatDetails(id: String, on_close: Callback<()>) -> impl IntoView {
    let vm = LlmChatDetailsVm::new();
    let chat_id = id.clone();
    let messages_container_ref = NodeRef::<leptos::html::Div>::new();
    let context_pkgs = RwSignal::new(Vec::<
        contracts::domain::a018_llm_chat::context::ContextPackageSummary,
    >::new());
    // Сколько секунд выполняется текущий запрос к LLM (тикает раз в секунду,
    // пока vm.is_sending == true). Показывается под индикатором набора.
    let elapsed_secs = RwSignal::new(0u32);
    // Текущий этап выполнения LLM-задачи (с бэкенда через polling). None — пока
    // не известен; тогда показываем дефолтную подпись.
    let progress = RwSignal::new(None::<JobProgress>);
    // job_id текущей фоновой задачи — для кнопки «Стоп» (cancel).
    let current_job_id = RwSignal::new(None::<String>);
    // Панель «Диагностика»: открыта ли и текст опционального комментария пользователя.
    let diag_open = RwSignal::new(false);
    let diag_comment = RwSignal::new(String::new());
    // Переключатель модели в чате: allowed_models — курируемый список моделей подключения,
    // selected_model — текущий выбор (прокидывается на каждое сообщение).
    let allowed_models = RwSignal::new(Vec::<String>::new());
    let selected_model = RwSignal::new(String::new());
    let nav_ctx = use_context::<AppGlobalContext>();

    // Scroll to bottom helper
    let scroll_to_bottom = {
        let messages_container_ref = messages_container_ref.clone();
        move || {
            if let Some(container) = messages_container_ref.get() {
                request_animation_frame(move || {
                    container.set_scroll_top(container.scroll_height());
                });
            }
        }
    };

    // Send message handler - using Callback to avoid move issues
    let handle_send = Callback::new({
        let chat_id = chat_id.clone();
        let scroll_to_bottom = scroll_to_bottom.clone();
        move |_| {
            let content = vm.new_message.get();
            if content.trim().is_empty() {
                return;
            }

            vm.is_sending.set(true);
            vm.new_message.set(String::new());

            // Запустить секундный таймер на время ожидания ответа LLM.
            elapsed_secs.set(0);
            progress.set(None);
            {
                let start = js_sys::Date::now();
                let is_sending = vm.is_sending;
                wasm_bindgen_futures::spawn_local(async move {
                    while is_sending.get_untracked() {
                        gloo_timers::future::TimeoutFuture::new(1000).await;
                        if !is_sending.get_untracked() {
                            break;
                        }
                        elapsed_secs.set(((js_sys::Date::now() - start) / 1000.0) as u32);
                    }
                });
            }

            // Create optimistic user message
            let chat_uuid = Uuid::parse_str(&chat_id).unwrap_or_else(|_| Uuid::new_v4());
            let chat_id_obj =
                contracts::domain::a018_llm_chat::aggregate::LlmChatId::new(chat_uuid);
            let optimistic_msg = LlmChatMessage::user(chat_id_obj, content.clone());
            let optimistic_id = optimistic_msg.id;

            // Add optimistic message immediately
            let mut current_msgs = vm.messages.get();
            current_msgs.push(optimistic_msg);
            vm.messages.set(current_msgs);
            scroll_to_bottom();

            let chat_id = chat_id.clone();
            let scroll_to_bottom = scroll_to_bottom.clone();
            let attachment_ids = vm
                .uploaded_files
                .get()
                .iter()
                .map(|f| f.id.clone())
                .collect();
            wasm_bindgen_futures::spawn_local(async move {
                // 1. POST → immediately get job_id (server returns 202)
                let model_choice = Some(selected_model.get_untracked());
                let job_id =
                    match send_message(&chat_id, &content, attachment_ids, model_choice).await {
                        Ok(id) => id,
                        Err(e) => {
                            vm.error.set(Some(format!("Ошибка отправки: {}", e)));
                            vm.is_sending.set(false);
                            return;
                        }
                    };
                current_job_id.set(Some(job_id.clone()));

                // 2. Опрос статуса каждые 500мс: progress.partial_text несёт частичный
                //    текст ответа (стриминг), поэтому частый опрос = живой вывод.
                //    Бюджет — 6 минут: агентные навыки делают много шагов tool-calling.
                let poll_result = poll_until_done(&job_id, 720, 500, progress).await;
                current_job_id.set(None);

                // 3. Always reload messages from DB after completion
                match fetch_messages(&chat_id).await {
                    Ok(msgs) => {
                        vm.messages.set(msgs);
                        scroll_to_bottom();
                    }
                    Err(_) => {
                        let mut current_msgs = vm.messages.get();
                        current_msgs.retain(|msg| msg.id != optimistic_id);
                        vm.messages.set(current_msgs);
                    }
                }

                // Перезагрузить пакеты контекста: документы, прикреплённые во время
                // сессии, должны появиться в ленте на своих местах по времени.
                if let Ok(pkgs) = fetch_chat_context(&chat_id).await {
                    context_pkgs.set(pkgs);
                }

                match poll_result {
                    Ok(PollOutcome::Done) => {
                        vm.uploaded_files.set(Vec::new());
                        vm.error.set(None);
                    }
                    Ok(PollOutcome::StillRunning { waited_secs }) => {
                        // Задача не уложилась в бюджет ожидания, но продолжает выполняться
                        // на сервере и допишет ответ сама. Это не ошибка — мягко поясняем.
                        vm.error.set(Some(format!(
                            "Ответ готовится дольше обычного (прошло ~{} мин, сложная задача \
                             с несколькими шагами). Он появится в чате автоматически — \
                             обновите страницу через минуту, если не появился.",
                            waited_secs.max(60) / 60
                        )));
                    }
                    Ok(PollOutcome::Error(msg)) if msg == "cancelled" => {
                        // Пользователь нажал «Стоп» — это не ошибка.
                        vm.error.set(Some("Генерация остановлена.".to_string()));
                    }
                    Ok(PollOutcome::Error(msg)) => {
                        vm.error.set(Some(format!("Ошибка LLM: {}", msg)));
                    }
                    Err(e) => {
                        vm.error
                            .set(Some(format!("Ошибка связи при ожидании ответа: {}", e)));
                    }
                }

                vm.is_sending.set(false);
                progress.set(None);
            });
        }
    });

    // Load chat and messages; затем, если чат только что создан со страницы списка,
    // автоматически отправить первый вопрос пользователя (handle_send покажет
    // оптимистичное сообщение, индикатор набора и подгрузит ответ).
    Effect::new({
        let chat_id = chat_id.clone();
        let scroll_to_bottom = scroll_to_bottom.clone();
        let ctx = use_context::<AppGlobalContext>();
        move |_| {
            let chat_id = chat_id.clone();
            let scroll_to_bottom = scroll_to_bottom.clone();
            wasm_bindgen_futures::spawn_local(async move {
                // Load chat
                match fetch_chat(&chat_id).await {
                    Ok(chat) => {
                        let conn_id = chat.chat.agent_id.as_string();
                        if selected_model.get_untracked().is_empty() {
                            selected_model.set(chat.chat.model_name.clone());
                        }
                        vm.chat.set(Some(chat));
                        // Курируемый список моделей подключения для переключателя.
                        if let Ok(models) = fetch_connection_allowed_models(&conn_id).await {
                            allowed_models.set(models);
                        }
                    }
                    Err(e) => vm.error.set(Some(e)),
                }

                // Load messages
                match fetch_messages(&chat_id).await {
                    Ok(msgs) => {
                        vm.messages.set(msgs);
                        vm.error.set(None);
                        scroll_to_bottom();
                    }
                    Err(e) => vm.error.set(Some(e)),
                }

                // Load attached page-context packages (for the context chip strip).
                if let Ok(pkgs) = fetch_chat_context(&chat_id).await {
                    context_pkgs.set(pkgs);
                }

                // Авто-отправка первого вопроса для только что созданного чата.
                if let Some(ctx) = ctx {
                    let key = pending_first_message_key(&chat_id);
                    let pending = ctx
                        .get_form_state(&key)
                        .and_then(|v| v.as_str().map(|s| s.to_string()));
                    if let Some(pending) = pending {
                        // Одноразово: очистить, чтобы не переотправлять при ремоунте вкладки.
                        ctx.set_form_state(key, serde_json::Value::Null);
                        if !pending.trim().is_empty() {
                            vm.new_message.set(pending);
                            handle_send.run(());
                        }
                    }
                }
            });
        }
    });

    // Реактивно перезагружать ленту контекста, когда документ добавлен из шапки
    // (`AiChatHeaderButton`) к уже открытому чату: вкладка не переоткрывается, а
    // версия контекста в `form_states` инкрементируется — на это и реагируем.
    Effect::new({
        let chat_id = chat_id.clone();
        let ctx = use_context::<AppGlobalContext>();
        let last_seen = RwSignal::new(None::<u64>);
        move |_| {
            let Some(ctx) = ctx else { return };
            let key = crate::domain::a018_llm_chat::ui::context_version_key(&chat_id);
            // Tracked-чтение карты: эффект перевызывается при любом изменении.
            let version = ctx
                .form_states
                .with(|m| m.get(&key).and_then(|v| v.as_u64()).unwrap_or(0));
            match last_seen.get_untracked() {
                None => {
                    // Первый прогон: запомнить текущую версию, не перезагружая.
                    last_seen.set(Some(version));
                    return;
                }
                Some(prev) if prev == version => return,
                _ => {}
            }
            last_seen.set(Some(version));

            let chat_id = chat_id.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(pkgs) = fetch_chat_context(&chat_id).await {
                    context_pkgs.set(pkgs);
                }
            });
        }
    });

    // Клон chat_id для виджета оценки (остальные клоны разошлись по замыканиям выше).
    let chat_id_for_rating = chat_id.clone();
    let chat_id_for_delete = chat_id.clone();

    view! {
        <PageFrame page_id="a018_llm_chat--detail" category=PAGE_CAT_DETAIL class="a018-llm-chat-detail">
            <div class="page__header" style="flex-wrap: wrap; height: auto; gap: 8px 12px;">
                <div class="page__header-left" style="flex-wrap: wrap;">
                    <h1 class="page__title" style="white-space: normal; overflow-wrap: anywhere;">
                        {move || {
                            vm.chat
                                .get()
                                .map(|c| c.chat.base.description.clone())
                                .unwrap_or_else(|| "Загрузка...".to_string())
                        }}
                    </h1>
                    <span class="page__header-meta">
                        {move || {
                            vm.chat.get().map(|c| {
                                let agent_display = c.agent_name.clone().unwrap_or_else(|| c.chat.agent_id.as_string());
                                format!("Агент: {}", agent_display)
                            }).unwrap_or_default()
                        }}
                    </span>
                    <span class="page__header-meta">
                        "Модель: "
                        <select
                            style="height: 24px; padding: 0 4px; border: 1px solid var(--colorNeutralStroke2); border-radius: 4px; background: var(--color-surface); color: var(--color-text);"
                            prop:value=move || selected_model.get()
                            on:change=move |ev| selected_model.set(event_target_value(&ev))
                            title="Модель ограничена списком allowed_models подключения"
                        >
                            {move || {
                                let mut list = allowed_models.get();
                                let current = selected_model.get();
                                if !current.is_empty() && !list.contains(&current) {
                                    list.insert(0, current);
                                }
                                if list.is_empty() {
                                    let m = selected_model.get();
                                    if !m.is_empty() {
                                        list = vec![m];
                                    }
                                }
                                list.into_iter()
                                    .map(|m| {
                                        let label = m.clone();
                                        view! { <option value=m>{label}</option> }
                                    })
                                    .collect_view()
                            }}
                        </select>
                    </span>
                    <span class="page__header-meta">
                        {move || format!("Сообщений: {}", vm.messages.get().len())}
                    </span>
                </div>
                <div class="page__header-right" style="display: flex; align-items: center; gap: 12px;">
                    // Оценка чата: 5 звёзд. Клик по текущей звезде снимает оценку.
                    // Звёзды идут перед кнопками.
                    <div
                        title="Оценить чат"
                        style="display: inline-flex; gap: 2px; font-size: 20px; line-height: 1;"
                    >
                        {move || {
                            let cid = chat_id_for_rating.clone();
                            let current = vm.chat.get().and_then(|c| c.chat.rating).unwrap_or(0);
                            (1..=5)
                                .map(|n| {
                                    let cid = cid.clone();
                                    let filled = n <= current;
                                    view! {
                                        <button
                                            type="button"
                                            title=move || format!("Оценка: {}", n)
                                            style=move || format!(
                                                "background:none;border:none;cursor:pointer;padding:0 1px;line-height:1;color:{};",
                                                if filled { "#f5a623" } else { "var(--color-text-secondary, #9ca3af)" }
                                            )
                                            on:click=move |_| {
                                                let cid = cid.clone();
                                                let target = if current == n { None } else { Some(n) };
                                                wasm_bindgen_futures::spawn_local(async move {
                                                    match set_rating(&cid, target).await {
                                                        Ok(()) => vm.chat.update(|opt| {
                                                            if let Some(c) = opt { c.chat.rating = target; }
                                                        }),
                                                        Err(e) => vm.error.set(Some(format!("Ошибка оценки: {}", e))),
                                                    }
                                                });
                                            }
                                        >
                                            {if filled { "★" } else { "☆" }}
                                        </button>
                                    }
                                })
                                .collect_view()
                        }}
                    </div>
                    // Кнопки заголовка идут рядом, после звёзд.
                    <div style="display: flex; align-items: center; gap: 8px;">
                        // Диагностика: открывает модальный диалог с комментарием;
                        // запуск проверки закрывает диалог и отправляет промпт в чат.
                        <Button
                            appearance=ButtonAppearance::Secondary
                            disabled=vm.is_sending
                            on_click=move |_| diag_open.set(true)
                        >
                            {icon("search")}
                            " Диагностика"
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            on_click=move |_| on_close.run(())
                        >
                            {icon("x")}
                            " Закрыть"
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Subtle
                            on_click=move |_| {
                                let confirmed = web_sys::window()
                                    .and_then(|win| win.confirm_with_message("Удалить чат?").ok())
                                    .unwrap_or(false);
                                if !confirmed {
                                    return;
                                }
                                let id = chat_id_for_delete.clone();
                                wasm_bindgen_futures::spawn_local(async move {
                                    match delete_chat(&id).await {
                                        Ok(()) => on_close.run(()),
                                        Err(e) => vm.error.set(Some(format!("Ошибка удаления: {}", e))),
                                    }
                                });
                            }
                        >
                            {icon("delete")}
                            " Удалить"
                        </Button>
                    </div>
                </div>
            </div>

            <div class="page__content" style="display: flex; flex-direction: column; min-height: 0;">
                // Error display
                {move || {
                    vm.error
                        .get()
                        .map(|e| {
                            view! {
                                <div class="warning-box" style="background: var(--color-error-50); border-color: var(--color-error-100); margin-bottom: var(--spacing-md);">
                                    <span class="warning-box__text" style="color: var(--color-error);">{e}</span>
                                </div>
                            }
                        })
                }}

                // Messages area — full width, no frame; rows distinguished by background.
                // Сообщения и события прикрепления контекста показываются единой
                // лентой в хронологическом порядке (сортировка по времени создания).
                <div
                node_ref=messages_container_ref
                style="flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 2px; margin-bottom: 16px;"
            >
                <For
                    each=move || {
                        let mut rows: Vec<FeedRow> = Vec::new();
                        for m in vm.messages.get() {
                            rows.push(FeedRow {
                                ts: m.created_at,
                                key: format!("m-{}", m.id),
                                item: FeedItem::Message(m),
                            });
                        }
                        for p in context_pkgs.get() {
                            let ts = chrono::DateTime::parse_from_rfc3339(&p.created_at)
                                .map(|d| d.with_timezone(&chrono::Utc))
                                .unwrap_or_else(|_| chrono::Utc::now());
                            rows.push(FeedRow {
                                ts,
                                key: format!("c-{}", p.id),
                                item: FeedItem::Context(p),
                            });
                        }
                        rows.sort_by(|a, b| a.ts.cmp(&b.ts));
                        rows
                    }
                    key=|row| row.key.clone()
                    let:row
                >
                    {match row.item {
                        FeedItem::Message(msg) => MessageRow(msg).into_any(),
                        FeedItem::Context(p) => ContextRow(p, nav_ctx).into_any(),
                    }}
                </For>

                // Loading indicator — показывается пока LLM обрабатывает запрос.
                // Если стриминг уже принёс частичный текст ответа — рендерим его
                // (живой вывод, как в Claude/ChatGPT), под ним — этап и кнопка «Стоп».
                {move || {
                    if vm.is_sending.get() {
                        Some(view! {
                            <div style="display: flex; flex-direction: column; gap: 6px; align-self: stretch;">
                                {move || {
                                    progress.get()
                                        .and_then(|p| p.partial_text)
                                        .filter(|t| !t.trim().is_empty())
                                        .map(|partial| view! {
                                            <div style="display: flex; gap: 12px; padding: 10px 14px; border-radius: 8px; background: var(--colorNeutralBackground1); opacity: 0.9;">
                                                <div style="flex: 0 0 104px; font-size: 11px; font-weight: 600; opacity: 0.6;">
                                                    "АССИСТЕНТ"
                                                </div>
                                                <div style="flex: 1; min-width: 0;">
                                                    <Markdown text=partial />
                                                </div>
                                            </div>
                                        })
                                }}
                                <div class="chat-typing" style="align-self: flex-start; max-width: 70%; display: flex; align-items: center; gap: 10px;">
                                    <div class="chat-typing__bubble">
                                        <span class="chat-typing__dot"></span>
                                        <span class="chat-typing__dot"></span>
                                        <span class="chat-typing__dot"></span>
                                        <span class="chat-typing__label">
                                            {move || {
                                                let secs = elapsed_secs.get();
                                                match progress.get() {
                                                    Some(p) if p.step > 0 => {
                                                        format!(" Шаг {} · {} · {} с", p.step, p.stage, secs)
                                                    }
                                                    Some(p) => format!(" {} · {} с", p.stage, secs),
                                                    None => format!(" LLM обрабатывает запрос… {} с", secs),
                                                }
                                            }}
                                        </span>
                                    </div>
                                    {move || {
                                        current_job_id.get().map(|job_id| view! {
                                            <button
                                                title="Остановить генерацию"
                                                style="border: 1px solid var(--colorNeutralStroke2); background: var(--colorNeutralBackground2); border-radius: 6px; padding: 4px 10px; font-size: 12px; cursor: pointer;"
                                                on:click=move |_| {
                                                    let job_id = job_id.clone();
                                                    wasm_bindgen_futures::spawn_local(async move {
                                                        let _ = cancel_job(&job_id).await;
                                                    });
                                                }
                                            >
                                                "■ Стоп"
                                            </button>
                                        })
                                    }}
                                </div>
                            </div>
                        })
                    } else {
                        None
                    }
                }}
                </div>

                // Input area — фиксированная ширина по колонке ленты, по центру,
                // чтобы поле ввода не растягивалось на весь экран.
                <div style="display: flex; flex-direction: column; gap: 8px; max-width: 980px; width: 100%; margin: 0 auto;">
                // File attachments display
                {move || {
                    let files = vm.uploaded_files.get();
                    if !files.is_empty() {
                        Some(
                            view! {
                                <Flex style="gap: 8px; flex-wrap: wrap;">
                                    <For
                                        each=move || vm.uploaded_files.get()
                                        key=|f| f.id.clone()
                                        let:file
                                    >
                                        <div style="padding: 6px 12px; background: var(--colorNeutralBackground2); border: 1px solid var(--colorNeutralStroke2); border-radius: 6px; display: flex; align-items: center; gap: 8px;">
                                            <span style="font-size: 14px;">
                                                {icon("document")}
                                                " "
                                                {file.filename.clone()}
                                            </span>
                                            <button
                                                style="background: none; border: none; cursor: pointer; padding: 2px; color: var(--colorNeutralForeground3);"
                                                on:click={
                                                    let file_id = file.id.clone();
                                                    move |_| {
                                                        let mut files = vm.uploaded_files.get();
                                                        files.retain(|f| f.id != file_id);
                                                        vm.uploaded_files.set(files);
                                                    }
                                                }
                                            >
                                                {icon("close")}
                                            </button>
                                        </div>
                                    </For>
                                </Flex>
                            },
                        )
                    } else {
                        None
                    }
                }}

                <Flex style="gap: 8px; align-items: flex-end;">
                    <input
                        type="file"
                        accept=".txt,.md,.rs,.toml,.json,.sql,.js,.ts,.py,.go,.java,.c,.cpp,.h,.hpp,.cs,.rb,.php,.html,.css,.xml,.yaml,.yml"
                        style="display: none;"
                        id="file-input"
                        on:change={
                            let chat_id = chat_id.clone();
                            move |ev| {
                                use wasm_bindgen::JsCast;
                                let input: web_sys::HtmlInputElement = ev.target().unwrap().dyn_into().unwrap();
                                if let Some(files) = input.files() {
                                    if let Some(file) = files.get(0) {
                                        let chat_id = chat_id.clone();
                                        wasm_bindgen_futures::spawn_local(async move {
                                            match super::model::upload_file(&chat_id, file).await {
                                                Ok(file_info) => {
                                                    let mut uploaded = vm.uploaded_files.get();
                                                    uploaded.push(file_info);
                                                    vm.uploaded_files.set(uploaded);
                                                }
                                                Err(e) => {
                                                    vm.error.set(Some(format!("Ошибка загрузки файла: {}", e)));
                                                }
                                            }
                                        });
                                    }
                                }
                                // Clear input
                                input.set_value("");
                            }
                        }
                    />

                    <div style="flex: 1;">
                        <Textarea
                            value=vm.new_message
                            placeholder="Введите сообщение... (Ctrl+Enter для отправки)"
                            attr:style="width: 100%; min-height: 60px; max-height: 200px; resize: vertical;"
                            disabled=vm.is_sending
                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                if ev.key() == "Enter" && ev.ctrl_key() {
                                    ev.prevent_default();
                                    handle_send.run(());
                                }
                            }
                        />
                    </div>

                    // Голосовой ввод: распознанный текст дописывается в поле ввода,
                    // дальше работает обычный handle_send. Компонент самодостаточен.
                    <DictationButton
                        target=vm.new_message
                        disabled=vm.is_sending
                        on_error=Callback::new(move |m: String| vm.error.set(Some(m)))
                    />

                    // Диагностика микрофона + подсказка по разблокировке на HTTP
                    // (chrome-флаг unsafely-treat-insecure-origin-as-secure).
                    <DictationDiagnostics />

                    // Компактные иконочные кнопки (узкие по ширине): прикрепить и отправить.
                    <Button
                        appearance=ButtonAppearance::Secondary
                        disabled=vm.is_sending
                        attr:title="Прикрепить файл"
                        attr:style="min-width: 40px; padding-left: 8px; padding-right: 8px;"
                        on_click=move |_| {
                            if let Some(window) = web_sys::window() {
                                if let Some(document) = window.document() {
                                    if let Some(input) = document.get_element_by_id("file-input") {
                                        use wasm_bindgen::JsCast;
                                        if let Ok(input) = input.dyn_into::<web_sys::HtmlElement>() {
                                            input.click();
                                        }
                                    }
                                }
                            }
                        }
                    >
                        {icon("attach")}
                    </Button>

                    <Button
                        appearance=ButtonAppearance::Primary
                        disabled=vm.is_sending
                        attr:title=move || if vm.is_sending.get() { "Отправка…" } else { "Отправить" }
                        attr:style="min-width: 40px; padding-left: 8px; padding-right: 8px;"
                        on_click=move |_| handle_send.run(())
                    >
                        {icon("send")}
                    </Button>
                </Flex>
                </div>
            </div>

            // Диалог диагностики: предопределённый разбор диалога моделью + опц.
            // комментарий. «Запустить проверку» закрывает диалог и отправляет промпт.
            <Dialog open=diag_open>
                <DialogSurface>
                    <DialogBody>
                        <DialogTitle>"Диагностика диалога"</DialogTitle>
                        <DialogContent>
                            <div style="display: flex; flex-direction: column; gap: 8px;">
                                <span style="font-size: 13px; opacity: 0.7;">
                                    "Модель разберёт диалог текстом (без вызова инструментов): что хотел \
                                     пользователь, какие шаги/ошибки были и что делать дальше."
                                </span>
                                <Textarea
                                    value=diag_comment
                                    placeholder="Комментарий или вопрос для проверки (необязательно)…"
                                    attr:style="width: 100%; min-height: 80px;"
                                    disabled=vm.is_sending
                                />
                                // Голосовой ввод комментария (обычный размер кнопки).
                                <div style="display: flex; align-items: center; gap: 8px;">
                                    <DictationButton
                                        target=diag_comment
                                        disabled=vm.is_sending
                                        on_error=Callback::new(move |m: String| vm.error.set(Some(m)))
                                    />
                                    <span style="font-size: 12px; opacity: 0.6;">"Голосовой ввод"</span>
                                </div>
                            </div>
                        </DialogContent>
                        <DialogActions>
                            <Button
                                appearance=ButtonAppearance::Secondary
                                on_click=move |_| diag_open.set(false)
                            >
                                "Отмена"
                            </Button>
                            <Button
                                appearance=ButtonAppearance::Primary
                                disabled=vm.is_sending
                                on_click=move |_| {
                                    let comment = diag_comment.get();
                                    let mut msg = String::from(DIAGNOSTIC_PROMPT);
                                    if !comment.trim().is_empty() {
                                        msg.push_str("\n\nКомментарий/вопрос пользователя: ");
                                        msg.push_str(comment.trim());
                                    }
                                    vm.new_message.set(msg);
                                    diag_open.set(false);
                                    diag_comment.set(String::new());
                                    handle_send.run(());
                                }
                            >
                                {icon("search")}
                                " Запустить проверку"
                            </Button>
                        </DialogActions>
                    </DialogBody>
                </DialogSurface>
            </Dialog>
        </PageFrame>
    }
}
