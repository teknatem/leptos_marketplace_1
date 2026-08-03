use std::collections::{HashMap, HashSet};

use contracts::system::tickets::{
    CreateTicketRequest, TicketAttachmentDto, TicketCommentDto, TicketPriority, TicketStatus,
    TicketType, UpdateTicketRequest,
};
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;
use web_sys::Url;

use crate::layout::global_context::AppGlobalContext;
use crate::shared::auth_download::{
    download_object_url, fetch_authenticated_blob, open_object_url_in_new_tab,
};
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::components::popover::show_message_popover;
use crate::shared::date_utils::{format_datetime, format_datetime_utc_local};
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_SYSTEM;
use crate::shared::screenshot_editor::{
    first_image_from_data_transfer, is_editable_image, read_image_file_from_clipboard,
    PendingScreenshot, ScreenshotEditor,
};
use crate::system::auth::context::use_auth;
use crate::system::auth::guard::RequireAuth;
use crate::system::tickets::api;

fn format_size(bytes: i64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} МБ", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.0} КБ", bytes as f64 / 1024.0)
    } else {
        format!("{} Б", bytes)
    }
}

fn origin_label(origin: &str) -> &'static str {
    match origin {
        "llm_chat" => "LLM-чат",
        "bitrix" => "Битрикс24",
        _ => "Вручную",
    }
}

fn is_image_attachment(attachment: &TicketAttachmentDto) -> bool {
    if let Some(content_type) = &attachment.content_type {
        if content_type.starts_with("image/") {
            return true;
        }
    }
    let lower = attachment.filename.to_lowercase();
    ["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg"]
        .iter()
        .any(|ext| lower.ends_with(&format!(".{ext}")))
}

#[component]
fn AttachmentImagePreview(
    attachment_id: String,
    image_cache: RwSignal<HashMap<String, CachedAttachmentImage>>,
    on_open: UnsyncCallback<()>,
) -> impl IntoView {
    let attachment_id = StoredValue::new(attachment_id);

    view! {
        {move || image_cache.with(|cache| {
            cache
                .get(&attachment_id.get_value())
                .map(|cached| cached.object_url.clone())
        }).map(|src| view! {
            <img
                class="sys-ticket-details__attachment-preview"
                src=src
                alt=""
                on:click=move |ev| {
                    ev.stop_propagation();
                    on_open.run(());
                }
            />
        })}
    }
}

#[derive(Clone)]
struct CachedAttachmentImage {
    object_url: String,
}

fn clear_attachment_image_cache(image_cache: RwSignal<HashMap<String, CachedAttachmentImage>>) {
    image_cache.update(|cache| {
        for cached in cache.values() {
            let _ = Url::revoke_object_url(&cached.object_url);
        }
        cache.clear();
    });
}

/// Черновик комментария при загрузке вложения (если текст есть — создаём комментарий и привязываем).
#[derive(Clone, Copy, PartialEq)]
enum PendingUploadTarget {
    Attachment,
}

// ============================================================================
// Details page
// ============================================================================

#[component]
pub fn TicketDetailsPage(ticket_id: String) -> impl IntoView {
    let ticket_id = StoredValue::new(ticket_id);
    view! {
        <RequireAuth>
            <TicketDetailsInner ticket_id=ticket_id.get_value() />
        </RequireAuth>
    }
}

#[component]
fn TicketDetailsInner(ticket_id: String) -> impl IntoView {
    let ticket_id = StoredValue::new(ticket_id);

    let (loading, set_loading) = signal(true);
    let (saving, set_saving) = signal(false);
    let (uploading, set_uploading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (success, set_success) = signal::<Option<String>>(None);

    // Форма
    let code = RwSignal::new(String::new());
    let title = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let ticket_type = RwSignal::new(TicketType::Question.as_str().to_string());
    let status = RwSignal::new(TicketStatus::New.as_str().to_string());
    let priority = RwSignal::new(TicketPriority::Normal.as_str().to_string());
    let deadline = RwSignal::new(String::new());
    let assignee_user_id = RwSignal::new(String::new());
    let tags_text = RwSignal::new(String::new());

    // Read-only атрибуты
    let author_name = RwSignal::new(String::new());
    let origin = RwSignal::new(String::new());
    let created_at = RwSignal::new(String::new());
    let updated_at = RwSignal::new(String::new());
    let context_page_key = RwSignal::new(Option::<String>::None);
    let source_chat_id = RwSignal::new(Option::<String>::None);
    let bitrix_task_id = RwSignal::new(Option::<String>::None);
    let bitrix_synced_at = RwSignal::new(Option::<String>::None);
    let bitrix_received_at = RwSignal::new(Option::<String>::None);

    // Данные
    let comments: RwSignal<Vec<TicketCommentDto>> = RwSignal::new(Vec::new());
    let attachments: RwSignal<Vec<TicketAttachmentDto>> = RwSignal::new(Vec::new());
    let show_previews = RwSignal::new(false);
    let image_cache: RwSignal<HashMap<String, CachedAttachmentImage>> =
        RwSignal::new(HashMap::new());
    let preview_cache_loading = RwSignal::new(false);
    let preview_cache_failed: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());

    // Комментарий-композер (только текст)
    let comment_body = RwSignal::new(String::new());
    let (comment_sending, set_comment_sending) = signal(false);

    // Выделение связей комментарий ↔ вложение
    let selected_comment_id = RwSignal::new(None::<String>);
    let selected_attachment_id = RwSignal::new(None::<String>);

    // Редактор скриншота
    let pending_screenshot = RwSignal::new_local(None::<PendingScreenshot>);
    let screenshot_target = RwSignal::new_local(None::<PendingUploadTarget>);

    // Права
    let (auth_state, _) = use_auth();
    let is_admin = Signal::derive(move || {
        auth_state
            .get()
            .user_info
            .as_ref()
            .map(|u| u.is_admin)
            .unwrap_or(false)
    });

    let load_data = move || {
        let id = ticket_id.get_value();
        spawn_local(async move {
            set_loading.set(true);
            match api::fetch_ticket_details(&id).await {
                Ok(data) => {
                    let t = data.ticket;
                    code.set(t.code);
                    title.set(t.title);
                    description.set(t.description);
                    ticket_type.set(t.ticket_type.as_str().to_string());
                    status.set(t.status.as_str().to_string());
                    priority.set(t.priority.as_str().to_string());
                    deadline.set(t.deadline.unwrap_or_default());
                    assignee_user_id.set(t.assignee_user_id.unwrap_or_default());
                    tags_text.set(t.tags.join(", "));
                    author_name.set(
                        t.author_username
                            .unwrap_or_else(|| t.author_user_id.clone()),
                    );
                    origin.set(t.origin);
                    created_at.set(t.created_at);
                    updated_at.set(t.updated_at);
                    context_page_key.set(t.context_page_key);
                    source_chat_id.set(t.source_chat_id);
                    bitrix_task_id.set(t.bitrix_task_id);
                    bitrix_synced_at.set(t.bitrix_synced_at);
                    bitrix_received_at.set(t.bitrix_received_at);
                    comments.set(data.comments);
                    attachments.set(data.attachments);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(format!("Не удалось загрузить тикет: {}", e))),
            }
            set_loading.set(false);
        });
    };

    Effect::new(move |_| {
        load_data();
    });

    Effect::new(move |_| {
        let enabled = show_previews.get();
        let is_loading = preview_cache_loading.get();
        let image_attachments = attachments
            .get()
            .into_iter()
            .filter(is_image_attachment)
            .filter(|attachment| {
                image_cache.with(|cache| !cache.contains_key(&attachment.id))
                    && preview_cache_failed.with(|failed| !failed.contains(&attachment.id))
            })
            .collect::<Vec<_>>();

        if !enabled {
            if !image_cache.with(|cache| cache.is_empty()) {
                clear_attachment_image_cache(image_cache);
            }
            if !preview_cache_failed.with(|failed| failed.is_empty()) {
                preview_cache_failed.update(HashSet::clear);
            }
            return;
        }
        if is_loading || image_attachments.is_empty() {
            return;
        }

        let ticket = ticket_id.get_value();
        preview_cache_loading.set(true);
        spawn_local(async move {
            for attachment in image_attachments {
                if !show_previews.get_untracked() {
                    break;
                }
                if image_cache.with_untracked(|cache| cache.contains_key(&attachment.id)) {
                    continue;
                }

                let url = api::attachment_download_url(&ticket, &attachment.id);
                let mut loaded_blob = None;
                let mut last_error = None;
                for attempt in 0..3 {
                    match fetch_authenticated_blob(&url).await {
                        Ok(blob) => {
                            loaded_blob = Some(blob);
                            break;
                        }
                        Err(err) => last_error = Some(err),
                    }
                    if !show_previews.get_untracked() {
                        break;
                    }
                    if attempt < 2 {
                        TimeoutFuture::new(750 * (attempt + 1)).await;
                    }
                }

                if !show_previews.get_untracked() {
                    break;
                }
                match loaded_blob {
                    Some(blob) if show_previews.get_untracked() => {
                        match Url::create_object_url_with_blob(&blob) {
                            Ok(object_url) => {
                                image_cache.update(|cache| {
                                    if let Some(previous) = cache.insert(
                                        attachment.id.clone(),
                                        CachedAttachmentImage { object_url },
                                    ) {
                                        let _ = Url::revoke_object_url(&previous.object_url);
                                    }
                                });
                            }
                            Err(err) => {
                                preview_cache_failed.update(|failed| {
                                    failed.insert(attachment.id.clone());
                                });
                                set_error.set(Some(format!(
                                    "Не удалось подготовить превью «{}»: {err:?}",
                                    attachment.filename
                                )));
                            }
                        }
                    }
                    Some(_) => {}
                    None => {
                        preview_cache_failed.update(|failed| {
                            failed.insert(attachment.id.clone());
                        });
                        set_error.set(Some(format!(
                            "Не удалось загрузить превью «{}» после 3 попыток: {}",
                            attachment.filename,
                            last_error.unwrap_or_else(|| "неизвестная ошибка".to_string())
                        )));
                    }
                }
            }
            preview_cache_loading.set(false);
        });
    });

    on_cleanup(move || clear_attachment_image_cache(image_cache));

    let on_save = move |_| {
        set_saving.set(true);
        set_error.set(None);
        set_success.set(None);
        let id = ticket_id.get_value();
        let req = UpdateTicketRequest {
            title: title.get(),
            description: description.get(),
            ticket_type: ticket_type.get().as_str().into(),
            status: status.get().as_str().into(),
            priority: priority.get().as_str().into(),
            deadline: {
                let v = deadline.get();
                if v.trim().is_empty() {
                    None
                } else {
                    Some(v)
                }
            },
            assignee_user_id: {
                let v = assignee_user_id.get();
                if v.trim().is_empty() {
                    None
                } else {
                    Some(v)
                }
            },
            tags: tags_text
                .get()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
        };
        spawn_local(async move {
            match api::update_ticket(&id, &req).await {
                Ok(_) => {
                    set_success.set(Some("Изменения сохранены".to_string()));
                    load_data();
                }
                Err(e) => set_error.set(Some(format!("Ошибка сохранения: {}", e))),
            }
            set_saving.set(false);
        });
    };

    let global_ctx = use_context::<AppGlobalContext>();
    let file_input_ref = NodeRef::<leptos::html::Input>::new();
    let (reading_clipboard, set_reading_clipboard) = signal(false);
    let attachments_drag_over = RwSignal::new(false);

    let on_close = move |_| {
        if let Some(ctx) = global_ctx {
            ctx.close_tab(&format!("sys_ticket_details_{}", ticket_id.get_value()));
        }
    };

    // Загрузка вложений; при непустом тексте комментария — создаём комментарий и привязываем.
    let upload_files = StoredValue::new({
        move |files: Vec<web_sys::File>| {
            if files.is_empty() {
                return;
            }
            let id = ticket_id.get_value();
            set_uploading.set(true);
            spawn_local(async move {
                let draft = comment_body.get_untracked();
                let comment_id = if draft.trim().is_empty() {
                    None
                } else {
                    match api::add_comment(&id, draft.trim().to_string()).await {
                        Ok(comment) => {
                            comment_body.set(String::new());
                            Some(comment.id)
                        }
                        Err(e) => {
                            set_error.set(Some(format!(
                                "Не удалось создать комментарий для вложения: {}",
                                e
                            )));
                            set_uploading.set(false);
                            return;
                        }
                    }
                };
                for file in files {
                    if let Err(e) = api::upload_attachment(&id, comment_id.as_deref(), file).await {
                        set_error.set(Some(format!("Ошибка загрузки файла: {}", e)));
                    }
                }
                load_data();
                set_uploading.set(false);
            });
        }
    });

    let upload_single_file = move |file: web_sys::File| {
        upload_files.with_value(|upload| upload(vec![file]));
    };

    // Открыть редактор для изображения.
    let open_screenshot_editor = StoredValue::new_local({
        move |file: web_sys::File| {
            if let Some(previous) = pending_screenshot.get_untracked() {
                previous.revoke();
            }
            match PendingScreenshot::open(file) {
                Ok(pending) => {
                    screenshot_target.set(Some(PendingUploadTarget::Attachment));
                    pending_screenshot.set(Some(pending));
                }
                Err(()) => set_error.set(Some("Не удалось открыть изображение.".to_string())),
            }
        }
    });

    on_cleanup(move || {
        if let Some(pending) = pending_screenshot.get_untracked() {
            pending.revoke();
        }
    });

    let cancel_screenshot = UnsyncCallback::new(move |_| {
        if let Some(pending) = pending_screenshot.get_untracked() {
            pending.revoke();
        }
        pending_screenshot.set(None);
        screenshot_target.set(None);
    });

    let confirm_screenshot = UnsyncCallback::new(move |edited: web_sys::File| {
        let Some(pending) = pending_screenshot.get_untracked() else {
            return;
        };
        pending.revoke();
        pending_screenshot.set(None);
        screenshot_target.set(None);
        upload_single_file(edited);
    });

    let paste_screenshot = move |event: leptos::ev::MouseEvent| {
        if reading_clipboard.get_untracked() || uploading.get_untracked() {
            return;
        }
        let popover_x = event.client_x();
        let popover_y = event.client_y();
        set_reading_clipboard.set(true);
        spawn_local(async move {
            match read_image_file_from_clipboard().await {
                Ok(file) => open_screenshot_editor.with_value(|open| open(file)),
                Err(message) => {
                    let title = if message.contains("нет изображения") {
                        "В буфере обмена нет изображения"
                    } else {
                        "Не удалось получить скриншот"
                    };
                    let description = if message.contains("нет изображения") {
                        "В Windows 10/11 нажмите Win + Shift + S, выделите нужную область экрана и дождитесь копирования снимка. Затем вернитесь в тикет и снова нажмите «+Скриншот»."
                    } else {
                        "Разрешите браузеру доступ к буферу обмена. Затем в Windows 10/11 нажмите Win + Shift + S, выделите область экрана и снова нажмите «+Скриншот»."
                    };
                    show_message_popover(title, description, popover_x, popover_y);
                }
            }
            set_reading_clipboard.set(false);
        });
    };

    let trigger_file_picker = move |_| {
        if uploading.get_untracked() {
            return;
        }
        if let Some(input) = file_input_ref.get() {
            let _ = input.click();
        }
    };

    let on_attachments_drop = move |ev: web_sys::DragEvent| {
        ev.prevent_default();
        attachments_drag_over.set(false);
        let Some(dt) = ev.data_transfer() else {
            return;
        };
        let Some(files) = dt.files() else {
            return;
        };
        let mut batch: Vec<web_sys::File> = Vec::new();
        let mut editor_opened = false;
        for index in 0..files.length() {
            let Some(file) = files.get(index) else {
                continue;
            };
            if is_editable_image(&file) {
                if !editor_opened {
                    open_screenshot_editor.with_value(|open| open(file));
                    editor_opened = true;
                } else {
                    batch.push(file);
                }
            } else {
                batch.push(file);
            }
        }
        if !batch.is_empty() {
            upload_files.with_value(|upload| upload(batch));
        }
    };

    let on_attachments_paste = move |ev: web_sys::ClipboardEvent| {
        let Some(dt) = ev.clipboard_data() else {
            return;
        };
        if let Some(file) = first_image_from_data_transfer(&dt) {
            ev.prevent_default();
            open_screenshot_editor.with_value(|open| open(file));
        }
    };

    let on_file_input = move |ev: web_sys::Event| {
        let Some(input) = ev
            .target()
            .and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlInputElement>(t).ok())
        else {
            return;
        };
        let Some(files) = input.files() else {
            return;
        };
        let mut batch: Vec<web_sys::File> = Vec::new();
        let mut editor_opened = false;
        for index in 0..files.length() {
            if let Some(file) = files.get(index) {
                if is_editable_image(&file) && !editor_opened {
                    open_screenshot_editor.with_value(|open| open(file));
                    editor_opened = true;
                } else {
                    batch.push(file);
                }
            }
        }
        if !batch.is_empty() {
            upload_files.with_value(|upload| upload(batch));
        }
        input.set_value("");
    };

    let send_comment = move || {
        let body = comment_body.get_untracked();
        if body.trim().is_empty() {
            return;
        }
        let id = ticket_id.get_value();
        set_comment_sending.set(true);
        spawn_local(async move {
            match api::add_comment(&id, body).await {
                Ok(_) => {
                    comment_body.set(String::new());
                    load_data();
                }
                Err(e) => set_error.set(Some(format!("Ошибка отправки комментария: {}", e))),
            }
            set_comment_sending.set(false);
        });
    };

    let download = move |attachment: &TicketAttachmentDto| {
        if let Some(cached) = image_cache.with_untracked(|cache| cache.get(&attachment.id).cloned())
        {
            if let Err(e) = download_object_url(&cached.object_url, &attachment.filename) {
                set_error.set(Some(format!("Ошибка скачивания: {}", e)));
            }
            return;
        }

        let ticket = ticket_id.get_value();
        let id = attachment.id.clone();
        let filename = attachment.filename.clone();
        spawn_local(async move {
            if let Err(e) = api::download_attachment(&ticket, &id, &filename).await {
                set_error.set(Some(format!("Ошибка скачивания: {}", e)));
            }
        });
    };

    let open_attachment = move |attachment: &TicketAttachmentDto| {
        if let Some(cached) = image_cache.with_untracked(|cache| cache.get(&attachment.id).cloned())
        {
            if let Err(e) = open_object_url_in_new_tab(&cached.object_url) {
                set_error.set(Some(format!("Не удалось открыть файл: {}", e)));
            }
            return;
        }

        let ticket = ticket_id.get_value();
        let id = attachment.id.clone();
        spawn_local(async move {
            if let Err(e) = api::open_attachment_in_new_tab(&ticket, &id).await {
                set_error.set(Some(format!("Не удалось открыть файл: {}", e)));
            }
        });
    };

    let remove_attachment = move |attachment_id: String| {
        let confirmed = web_sys::window()
            .and_then(|win| win.confirm_with_message("Удалить вложение?").ok())
            .unwrap_or(false);
        if !confirmed {
            return;
        }
        let ticket = ticket_id.get_value();
        spawn_local(async move {
            match api::delete_attachment(&ticket, &attachment_id).await {
                Ok(()) => load_data(),
                Err(e) => set_error.set(Some(format!("Ошибка удаления вложения: {}", e))),
            }
        });
    };

    view! {
        <PageFrame page_id="sys_ticket_details" category=PAGE_CAT_SYSTEM class="sys-ticket-details-page">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">
                        {move || if loading.get() {
                            "Тикет".to_string()
                        } else {
                            format!("Тикет {}", code.get())
                        }}
                    </h1>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=on_save
                        disabled=Signal::derive(move || saving.get() || loading.get())
                    >
                        {icon("save")}
                        {move || if saving.get() { " Сохранение..." } else { " Сохранить" }}
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| load_data()
                        disabled=Signal::derive(move || loading.get())
                    >
                        {icon("refresh")}
                        " Обновить"
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Subtle
                        on_click=on_close
                        disabled=Signal::derive(move || loading.get())
                    >
                        {icon("x")}
                        " Закрыть"
                    </Button>
                </div>
            </div>

            {move || error.get().map(|e| view! { <div class="alert alert--error" style="margin: var(--spacing-sm) var(--spacing-md);">{e}</div> })}
            {move || success.get().map(|s| view! { <div class="alert alert--success" style="margin: var(--spacing-sm) var(--spacing-md);">{s}</div> })}

            <div class="page__content sys-ticket-details">
                <div class="sys-ticket-details__layout">
                    // ── Колонка 1: Реквизиты ───────────────────────────────
                    <div class="sys-ticket-details__col">
                        <div class="sys-ticket-details__col-card">
                        <CardAnimated delay_ms=0 nav_id="sys_ticket_details_requisites">
                            <h4 class="details-section__title">"Реквизиты"</h4>

                            <div class="form__group">
                                <label class="form__label">"Статус"</label>
                                <select
                                    class="form__select"
                                    style="width: 100%;"
                                    prop:value=move || status.get()
                                    on:change=move |ev| status.set(event_target_value(&ev))
                                    disabled=move || !is_admin.get() || saving.get() || loading.get()
                                >
                                    {TicketStatus::all().iter().map(|s| view! {
                                        <option value=s.as_str()>{s.label_ru()}</option>
                                    }).collect_view()}
                                </select>
                                {move || (!is_admin.get()).then(|| view! {
                                    <span class="form__hint">"Статус меняет администратор"</span>
                                })}
                            </div>

                            <div class="form__group">
                                <label class="form__label">"Тип"</label>
                                <select
                                    class="form__select"
                                    style="width: 100%;"
                                    prop:value=move || ticket_type.get()
                                    on:change=move |ev| ticket_type.set(event_target_value(&ev))
                                    disabled=move || saving.get() || loading.get()
                                >
                                    {TicketType::all().iter().map(|t| view! {
                                        <option value=t.as_str()>{t.label_ru()}</option>
                                    }).collect_view()}
                                </select>
                            </div>

                            <div class="form__group">
                                <label class="form__label">
                                    "Заголовок "
                                    <span style="color: var(--color-error);">"*"</span>
                                </label>
                                <Input
                                    value=title
                                    placeholder="Кратко: что случилось / что предлагаете"
                                    disabled=Signal::derive(move || saving.get() || loading.get())
                                />
                            </div>

                            <div class="form__group">
                                <label class="form__label">"Описание"</label>
                                <textarea
                                    class="form__textarea sys-ticket-details__native-textarea"
                                    rows="16"
                                    prop:value=move || description.get()
                                    on:input=move |event| {
                                        description.set(event_target_value(&event));
                                    }
                                    placeholder="Подробное описание..."
                                    disabled=move || saving.get() || loading.get()
                                ></textarea>
                            </div>

                            <div class="form__group">
                                <label class="form__label">"Теги (через запятую)"</label>
                                <Input
                                    value=tags_text
                                    placeholder="ui, отчёты, wb"
                                    disabled=Signal::derive(move || saving.get() || loading.get())
                                />
                            </div>

                            <div class="sys-ticket-details__section-divider" />

                            <div class="form__group">
                                <label class="form__label">"Приоритет"</label>
                                <select
                                    class="form__select"
                                    style="width: 100%;"
                                    prop:value=move || priority.get()
                                    on:change=move |ev| priority.set(event_target_value(&ev))
                                    disabled=move || saving.get() || loading.get()
                                >
                                    {TicketPriority::all().iter().map(|p| view! {
                                        <option value=p.as_str()>{p.label_ru()}</option>
                                    }).collect_view()}
                                </select>
                            </div>

                            <div class="sys-ticket-details__meta">
                                <div class="sys-ticket-details__meta-row">
                                    <span class="sys-ticket-details__meta-label">"Автор:"</span>
                                    <span>{move || author_name.get()}</span>
                                </div>
                                <div class="sys-ticket-details__meta-row">
                                    <span class="sys-ticket-details__meta-label">"Источник:"</span>
                                    <span>{move || origin_label(&origin.get()).to_string()}</span>
                                </div>
                                {move || context_page_key.get().map(|k| {
                                    let key = k.clone();
                                    view! {
                                        <div class="sys-ticket-details__meta-row">
                                            <span class="sys-ticket-details__meta-label">"Контекст:"</span>
                                            <button
                                                class="sys-ticket-details__link-button"
                                                title="Открыть страницу, на которой возникло обращение"
                                                on:click=move |_| {
                                                    if let Some(ctx) = global_ctx {
                                                        ctx.open_tab(&key, "Страница обращения");
                                                    }
                                                }
                                            >
                                                {k}
                                            </button>
                                        </div>
                                    }
                                })}
                                {move || source_chat_id.get().map(|chat_id| {
                                    let key = format!("a018_llm_chat_details_{chat_id}");
                                    view! {
                                        <div class="sys-ticket-details__meta-row">
                                            <span class="sys-ticket-details__meta-label">"Чат-исток:"</span>
                                            <button
                                                class="sys-ticket-details__link-button"
                                                title="Открыть диалог, из которого оформлено обращение"
                                                on:click=move |_| {
                                                    if let Some(ctx) = global_ctx {
                                                        ctx.open_tab(&key, "Поддержка");
                                                    }
                                                }
                                            >
                                                "Открыть диалог"
                                            </button>
                                        </div>
                                    }
                                })}
                                <div class="sys-ticket-details__meta-row">
                                    <span class="sys-ticket-details__meta-label">"Создан:"</span>
                                    <span>{move || format_datetime(&created_at.get())}</span>
                                </div>
                                <div class="sys-ticket-details__meta-row">
                                    <span class="sys-ticket-details__meta-label">"Обновлён:"</span>
                                    <span>{move || format_datetime(&updated_at.get())}</span>
                                </div>
                                {move || bitrix_synced_at.get().map(|value| view! {
                                    <div class="sys-ticket-details__meta-row">
                                        <span class="sys-ticket-details__meta-label">
                                            "Успешно отправлено в Битрикс24:"
                                        </span>
                                        <span title="Московское время (UTC+3)">
                                            {format_datetime_utc_local(&value, "%d.%m.%Y %H:%M:%S")}
                                            " (UTC+3)"
                                        </span>
                                    </div>
                                })}
                                {move || bitrix_received_at.get().map(|value| view! {
                                    <div class="sys-ticket-details__meta-row">
                                        <span class="sys-ticket-details__meta-label">
                                            "Успешно получено из Битрикс24:"
                                        </span>
                                        <span title="Московское время (UTC+3)">
                                            {format_datetime_utc_local(&value, "%d.%m.%Y %H:%M:%S")}
                                            " (UTC+3)"
                                        </span>
                                    </div>
                                })}
                                {move || bitrix_task_id.get().map(|task_id| {
                                    let href = format!(
                                        "https://sanstar.bitrix24.ru/workgroups/group/5/tasks/task/view/{task_id}/"
                                    );
                                    view! {
                                        <div class="sys-ticket-details__meta-row">
                                            <span class="sys-ticket-details__meta-label">"Битрикс24:"</span>
                                            <a href=href target="_blank" rel="noopener noreferrer">
                                                {format!("Открыть задачу #{task_id}")}
                                            </a>
                                        </div>
                                    }
                                })}
                            </div>
                        </CardAnimated>
                        </div>
                    </div>

                    // ── Колонка 2: Комментарии (только текст) ───────────────
                    <div class="sys-ticket-details__col">
                        <div class="sys-ticket-details__col-card">
                        <CardAnimated delay_ms=80 nav_id="sys_ticket_details_comments">
                            <h4 class="details-section__title">
                                "Комментарии"
                                {move || {
                                    let n = comments.get().len();
                                    if n > 0 { format!(" ({})", n) } else { String::new() }
                                }}
                            </h4>

                            <div class="sys-ticket-details__comments">
                                <For
                                    each=move || comments.get()
                                    key=|c| c.id.clone()
                                    children=move |comment| {
                                        let author = comment
                                            .author_username
                                            .clone()
                                            .unwrap_or_else(|| comment.author_user_id.clone());
                                        let created = format_datetime(&comment.created_at);
                                        let comment_id = comment.id.clone();
                                        let comment_id_click = comment_id.clone();
                                        let comment_id_sel = comment_id.clone();
                                        let is_selected = move || {
                                            selected_comment_id.get().as_deref()
                                                == Some(comment_id_sel.as_str())
                                        };
                                        let comment_id_link = comment_id.clone();
                                        let is_linked = move || {
                                            selected_attachment_id
                                                .with(|att_id| {
                                                    att_id.as_ref().is_some_and(|id| {
                                                        attachments.with(|list| {
                                                            list.iter().any(|a| {
                                                                a.id == *id
                                                                    && a.comment_id.as_deref()
                                                                        == Some(comment_id_link.as_str())
                                                            })
                                                        })
                                                    })
                                                })
                                        };
                                        view! {
                                            <div
                                                class="sys-ticket-details__comment"
                                                class:sys-ticket-details__comment--selected=is_selected
                                                class:sys-ticket-details__comment--linked=is_linked
                                                on:click=move |_| {
                                                    selected_attachment_id.set(None);
                                                    selected_comment_id.update(|sel| {
                                                        if sel.as_deref() == Some(comment_id_click.as_str()) {
                                                            *sel = None;
                                                        } else {
                                                            *sel = Some(comment_id_click.clone());
                                                        }
                                                    });
                                                }
                                            >
                                                <div class="sys-ticket-details__comment-header">
                                                    <span class="sys-ticket-details__comment-author">{author}</span>
                                                    <span class="sys-ticket-details__comment-date">{created}</span>
                                                </div>
                                                <div class="sys-ticket-details__comment-body">{comment.body.clone()}</div>
                                            </div>
                                        }
                                    }
                                />
                                {move || comments.get().is_empty().then(|| view! {
                                    <div class="sys-ticket-details__comments-empty">"Комментариев пока нет"</div>
                                })}
                            </div>

                            <div class="sys-ticket-details__composer">
                                <textarea
                                    class="form__textarea sys-ticket-details__native-textarea"
                                    rows="8"
                                    prop:value=move || comment_body.get()
                                    on:input=move |event| {
                                        comment_body.set(event_target_value(&event));
                                    }
                                    placeholder="Текст комментария..."
                                    disabled=move || comment_sending.get()
                                ></textarea>
                                <div class="sys-ticket-details__composer-actions">
                                    <Button
                                        appearance=ButtonAppearance::Primary
                                        on_click=move |_| send_comment()
                                        disabled=Signal::derive(move || comment_sending.get())
                                    >
                                        {icon("send")}
                                        {move || if comment_sending.get() { " Отправка..." } else { " Отправить" }}
                                    </Button>
                                </div>
                            </div>
                        </CardAnimated>
                        </div>
                    </div>

                    // ── Колонка 3: Вложения ─────────────────────────────────
                    <div class="sys-ticket-details__col">
                        <div class="sys-ticket-details__col-card">
                        <CardAnimated delay_ms=160 nav_id="sys_ticket_details_attachments">
                            <h4 class="details-section__title">
                                {move || format!("Вложения ({})", attachments.get().len())}
                            </h4>

                            <div class="sys-ticket-details__attachments-toolbar">
                                <Button
                                    appearance=ButtonAppearance::Secondary
                                    disabled=Signal::derive(move || uploading.get() || reading_clipboard.get())
                                    on_click=move |ev| paste_screenshot(ev)
                                >
                                    {icon("copy")}
                                    " +Скриншот"
                                </Button>
                                <Button
                                    appearance=ButtonAppearance::Secondary
                                    disabled=Signal::derive(move || uploading.get())
                                    on_click=move |ev| trigger_file_picker(ev)
                                >
                                    {icon("upload")}
                                    " +Файл"
                                </Button>
                                <Checkbox checked=show_previews label="Превью" />
                                <input
                                    node_ref=file_input_ref
                                    type="file"
                                    multiple=true
                                    style="display: none;"
                                    on:change=on_file_input
                                />
                            </div>
                            {move || preview_cache_loading.get().then(|| view! {
                                <div class="sys-ticket-details__preview-status">
                                    "Загрузка превью в кэш…"
                                </div>
                            })}

                            <div
                                class="sys-ticket-details__attachments-list"
                                class:sys-ticket-details__attachments-list--dragover=move || attachments_drag_over.get()
                                tabindex="0"
                                on:paste=on_attachments_paste
                                on:dragover=move |ev: web_sys::DragEvent| {
                                    ev.prevent_default();
                                    attachments_drag_over.set(true);
                                }
                                on:dragleave=move |_| attachments_drag_over.set(false)
                                on:drop=on_attachments_drop
                            >
                                <For
                                    each=move || attachments.get()
                                    key=|a| a.id.clone()
                                    children=move |attachment| {
                                        let att_id = attachment.id.clone();
                                        let att_id_click = att_id.clone();
                                        let for_download = attachment.clone();
                                        let for_open = attachment.clone();
                                        let id_for_delete = attachment.id.clone();
                                        let attachment_id_preview = attachment.id.clone();
                                        let is_image = is_image_attachment(&attachment);
                                        let comment_id = attachment.comment_id.clone();
                                        let comment_id_preview = comment_id.clone();
                                        let comment_preview = comment_id_preview.and_then(|cid| {
                                            comments.with_untracked(|list| {
                                                list.iter()
                                                    .find(|c| c.id == cid)
                                                    .map(|c| {
                                                        let body = c.body.trim();
                                                        if body.len() > 48 {
                                                            format!("{}…", &body[..48])
                                                        } else {
                                                            body.to_string()
                                                        }
                                                    })
                                            })
                                        });
                                        let comment_id_for_link = comment_id.clone();
                                        let is_linked = move || {
                                            comment_id_for_link.as_ref().is_some_and(|cid| {
                                                selected_comment_id.get().as_deref()
                                                    == Some(cid.as_str())
                                            })
                                        };
                                        let is_selected = move || {
                                            selected_attachment_id.get().as_deref()
                                                == Some(att_id.as_str())
                                        };
                                        let uploaded = format_datetime(&attachment.created_at);
                                        let size = format_size(attachment.file_size);
                                        let open_cb = UnsyncCallback::new({
                                            let attachment = for_open.clone();
                                            move |_| open_attachment(&attachment)
                                        });
                                        view! {
                                            <div
                                                class="sys-ticket-details__attachment-item"
                                                class:sys-ticket-details__attachment-item--selected=is_selected
                                                class:sys-ticket-details__attachment-item--linked=is_linked
                                                on:click=move |_| {
                                                    selected_attachment_id.update(|sel| {
                                                        if sel.as_deref() == Some(att_id_click.as_str()) {
                                                            *sel = None;
                                                        } else {
                                                            *sel = Some(att_id_click.clone());
                                                        }
                                                    });
                                                    if let Some(cid) = comment_id.clone() {
                                                        selected_comment_id.set(Some(cid));
                                                    } else {
                                                        selected_comment_id.set(None);
                                                    }
                                                }
                                            >
                                                <div class="sys-ticket-details__attachment-body">
                                                    <div class="sys-ticket-details__attachment-main">
                                                        <span
                                                            class="sys-ticket-details__attachment-name"
                                                            class:sys-ticket-details__attachment-name--image=is_image
                                                            on:click=move |ev| {
                                                                ev.stop_propagation();
                                                                if is_image {
                                                                    open_attachment(&for_open);
                                                                }
                                                            }
                                                        >
                                                            {attachment.filename.clone()}
                                                        </span>
                                                        <button
                                                            type="button"
                                                            class="sys-ticket-details__attachment-action"
                                                            title="Скачать"
                                                            on:click=move |ev: web_sys::MouseEvent| {
                                                                ev.stop_propagation();
                                                                download(&for_download);
                                                            }
                                                        >
                                                            {icon("download")}
                                                        </button>
                                                        <button
                                                            type="button"
                                                            class="sys-ticket-details__attachment-action sys-ticket-details__attachment-action--delete"
                                                            title="Удалить"
                                                            on:click=move |ev: web_sys::MouseEvent| {
                                                                ev.stop_propagation();
                                                                remove_attachment(id_for_delete.clone());
                                                            }
                                                        >
                                                            {icon("x")}
                                                        </button>
                                                    </div>
                                                    <div class="sys-ticket-details__attachment-meta">
                                                        <span>{uploaded}</span>
                                                        <span class="sys-ticket-details__attachment-meta-sep">"·"</span>
                                                        <span>{size}</span>
                                                        {comment_preview.map(|preview| view! {
                                                            <>
                                                                <span class="sys-ticket-details__attachment-meta-sep">"·"</span>
                                                                <span class="sys-ticket-details__attachment-comment">{preview}</span>
                                                            </>
                                                        })}
                                                    </div>
                                                </div>
                                                {is_image.then(|| view! {
                                                    <AttachmentImagePreview
                                                        attachment_id=attachment_id_preview.clone()
                                                        image_cache=image_cache
                                                        on_open=open_cb
                                                    />
                                                })}
                                            </div>
                                        }
                                    }
                                />
                            </div>
                        </CardAnimated>
                        </div>
                    </div>
                </div>
            </div>
            {move || pending_screenshot.get().map(|pending| view! {
                <ScreenshotEditor
                    source_file=pending.file
                    preview_url=pending.preview_url
                    on_cancel=cancel_screenshot
                    on_confirm=confirm_screenshot
                />
            })}
        </PageFrame>
    }
}

// ============================================================================
// Create page
// ============================================================================

#[component]
pub fn CreateTicketPage(on_close: Callback<()>) -> impl IntoView {
    let title = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let ticket_type = RwSignal::new(TicketType::Question.as_str().to_string());
    let priority = RwSignal::new(TicketPriority::Normal.as_str().to_string());
    let deadline = RwSignal::new(String::new());
    let (saving, set_saving) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);

    let global_ctx = use_context::<AppGlobalContext>();

    let on_save = move |_| {
        if title.get().trim().is_empty() {
            set_error.set(Some("Заголовок обязателен".to_string()));
            return;
        }

        let req = CreateTicketRequest {
            title: title.get(),
            description: description.get(),
            ticket_type: ticket_type.get().as_str().into(),
            priority: priority.get().as_str().into(),
            deadline: {
                let v = deadline.get();
                if v.trim().is_empty() {
                    None
                } else {
                    Some(v)
                }
            },
            assignee_user_id: None,
            tags: Vec::new(),
            context_page_key: None,
            context_json: None,
            origin: None,
        };

        set_saving.set(true);
        set_error.set(None);

        spawn_local(async move {
            match api::create_ticket(&req).await {
                Ok(ticket) => {
                    // Открываем созданный тикет (там можно добавить файлы) и закрываем форму
                    if let Some(ctx) = global_ctx {
                        ctx.open_tab(
                            &format!("sys_ticket_details_{}", ticket.id),
                            &format!("Тикет {}", ticket.code),
                        );
                    }
                    on_close.run(());
                }
                Err(e) => {
                    set_error.set(Some(format!("Ошибка создания: {}", e)));
                    set_saving.set(false);
                }
            }
        });
    };

    view! {
        <RequireAuth>
            <PageFrame page_id="sys_ticket_new" category=PAGE_CAT_SYSTEM class="sys-ticket-details-page">
                <div class="page__header">
                    <div class="page__header-left">
                        <h1 class="page__title">"Новый тикет"</h1>
                    </div>
                    <div class="page__header-right">
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=on_save
                            disabled=Signal::derive(move || saving.get())
                        >
                            {icon("save")}
                            {move || if saving.get() { " Сохранение..." } else { " Создать" }}
                        </Button>
                    </div>
                </div>

                {move || error.get().map(|e| view! {
                    <div class="alert alert--error" style="margin: var(--spacing-sm) var(--spacing-md);">{e}</div>
                })}

                <div class="page__content sys-ticket-details">
                    <div class="detail-grid">
                        <div class="detail-grid__col">
                            <CardAnimated delay_ms=0 nav_id="sys_ticket_new_main">
                                <h4 class="details-section__title">"Основные данные"</h4>

                                <div class="form__group">
                                    <label class="form__label">"Тип"</label>
                                    <select
                                        class="form__select"
                                        style="width: 100%;"
                                        prop:value=move || ticket_type.get()
                                        on:change=move |ev| ticket_type.set(event_target_value(&ev))
                                        disabled=move || saving.get()
                                    >
                                        {TicketType::all().iter().map(|t| view! {
                                            <option value=t.as_str()>{t.label_ru()}</option>
                                        }).collect_view()}
                                    </select>
                                </div>

                                <div class="form__group">
                                    <label class="form__label">
                                        "Заголовок "
                                        <span style="color: var(--color-error);">"*"</span>
                                    </label>
                                    <Input
                                        value=title
                                        placeholder="Кратко: что случилось / что предлагаете"
                                        disabled=Signal::derive(move || saving.get())
                                    />
                                </div>

                                <div class="form__group">
                                    <label class="form__label">"Описание"</label>
                                    <Textarea
                                        value=description
                                        placeholder="Подробное описание: шаги воспроизведения, ожидаемый результат, идея..."
                                        attr:rows=8
                                        attr:style="width: 100%; resize: vertical;"
                                        disabled=Signal::derive(move || saving.get())
                                    />
                                </div>
                            </CardAnimated>
                        </div>

                        <div class="detail-grid__col">
                            <CardAnimated delay_ms=80 nav_id="sys_ticket_new_attributes">
                                <h4 class="details-section__title">"Атрибуты"</h4>

                                <div class="form__group">
                                    <label class="form__label">"Приоритет"</label>
                                    <select
                                        class="form__select"
                                        style="width: 100%;"
                                        prop:value=move || priority.get()
                                        on:change=move |ev| priority.set(event_target_value(&ev))
                                        disabled=move || saving.get()
                                    >
                                        {TicketPriority::all().iter().map(|p| view! {
                                            <option value=p.as_str()>{p.label_ru()}</option>
                                        }).collect_view()}
                                    </select>
                                </div>

                                <div class="form__group">
                                    <label class="form__label">"Срок"</label>
                                    <input
                                        type="date"
                                        class="form__select"
                                        style="width: 100%;"
                                        prop:value=move || deadline.get()
                                        on:change=move |ev| deadline.set(event_target_value(&ev))
                                        disabled=move || saving.get()
                                    />
                                </div>

                                <p class="form__hint">
                                    "Файлы и скриншоты можно будет добавить после создания тикета"
                                </p>
                            </CardAnimated>
                        </div>
                    </div>
                </div>
            </PageFrame>
        </RequireAuth>
    }
}
