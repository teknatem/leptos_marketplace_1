//! LLM Chat Details - View Component
//!
//! Унифицирован с detail-страницами: PageFrame, page__header, page__content.
//! Агент отображается по имени (agent_name из API), а не по UUID.

use super::artifact_card::ArtifactCard;
use super::model::{
    fetch_chat, fetch_chat_context, fetch_messages, poll_until_done, send_message, PollOutcome,
};
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

/// Левый «жёлоб» строки ленты: имя автора и время (до секунд), выровненные
/// по левой границе блока.
#[allow(non_snake_case)]
fn FeedGutter(author: &'static str, time: String) -> impl IntoView {
    view! {
        <div style="flex: 0 0 96px; display: flex; flex-direction: column; gap: 2px; text-align: left;">
            <div style="font-size: 11px; font-weight: 600; letter-spacing: .02em; opacity: 0.6;">
                {author}
            </div>
            <div style="font-size: 11px; opacity: 0.45; font-variant-numeric: tabular-nums;">
                {time}
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
    let content = msg.content.clone();
    let time = format_utc_local(&msg.created_at, "%d.%m %H:%M:%S");
    let author = if is_user {
        "ВЫ"
    } else {
        "АССИСТЕНТ"
    };
    let row_style = if is_user {
        "width: 100%; padding: 12px 16px; background: var(--colorBrandBackground2);"
    } else {
        "width: 100%; padding: 12px 16px; background: var(--colorNeutralBackground1);"
    };
    view! {
        <div style=row_style>
            <div style="max-width: 980px; margin: 0 auto; display: flex; gap: 16px;">
                {FeedGutter(author, time)}
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
                            Some(view! { <ToolCallsTrace tool_trace=tool_trace.clone() /> })
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
                {FeedGutter("КОНТЕКСТ", time)}
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
                let job_id = match send_message(&chat_id, &content, attachment_ids).await {
                    Ok(id) => id,
                    Err(e) => {
                        vm.error.set(Some(format!("Ошибка отправки: {}", e)));
                        vm.is_sending.set(false);
                        return;
                    }
                };

                // 2. Опрос статуса каждые 2с. Бюджет — 6 минут: агентные навыки
                //    (графики/плагины) делают много шагов tool-calling и идут дольше.
                let poll_result = poll_until_done(&job_id, 180, 2000).await;

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
                    Ok(PollOutcome::Error(msg)) => {
                        vm.error.set(Some(format!("Ошибка LLM: {}", msg)));
                    }
                    Err(e) => {
                        vm.error
                            .set(Some(format!("Ошибка связи при ожидании ответа: {}", e)));
                    }
                }

                vm.is_sending.set(false);
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
                        vm.chat.set(Some(chat));
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

    view! {
        <PageFrame page_id="a018_llm_chat--detail" category=PAGE_CAT_DETAIL class="a018-llm-chat-detail">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">
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
                        {move || {
                            vm.chat
                                .get()
                                .map(|c| format!("Модель: {}", c.chat.model_name))
                                .unwrap_or_default()
                        }}
                    </span>
                    <span class="page__header-meta">
                        {move || format!("Сообщений: {}", vm.messages.get().len())}
                    </span>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| on_close.run(())
                    >
                        {icon("x")}
                        " Закрыть"
                    </Button>
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

                // Loading indicator — показывается пока LLM обрабатывает запрос
                {move || {
                    if vm.is_sending.get() {
                        Some(view! {
                            <div class="chat-typing" style="align-self: flex-start; max-width: 70%;">
                                <div class="chat-typing__bubble">
                                    <span class="chat-typing__dot"></span>
                                    <span class="chat-typing__dot"></span>
                                    <span class="chat-typing__dot"></span>
                                    <span class="chat-typing__label">
                                        {move || format!(" LLM обрабатывает запрос… {} с", elapsed_secs.get())}
                                    </span>
                                </div>
                            </div>
                        })
                    } else {
                        None
                    }
                }}
                </div>

                // Input area
                <div style="display: flex; flex-direction: column; gap: 8px;">
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

                    <Button
                        appearance=ButtonAppearance::Secondary
                        disabled=vm.is_sending
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
                        on_click=move |_| handle_send.run(())
                    >
                        {icon("send")}
                        {move || if vm.is_sending.get() { " Отправка..." } else { " Отправить" }}
                    </Button>
                </Flex>
                </div>
            </div>
        </PageFrame>
    }
}
