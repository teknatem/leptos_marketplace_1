use crate::domain::a018_llm_chat::ui::details::LlmChatDetails;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_DETAIL;
use contracts::domain::a031_kb_edit::aggregate::{KbEdit, KbEditStatus, KbEditType};
use contracts::domain::common::AggregateId;
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Serialize;
use thaw::*;

#[derive(Debug, Clone, Serialize)]
struct KbEditUpdateDto {
    id: Option<String>,
    code: Option<String>,
    title: String,
    comment: Option<String>,
    edit_type: Option<String>,
    status: Option<String>,
    agent_summary: String,
    target_articles: Vec<String>,
    applied_articles: Vec<String>,
    source_chat_ids: Vec<String>,
    agent_id: Option<String>,
    chat_id: Option<String>,
    analyze_task_run_id: Option<String>,
    post_task_run_id: Option<String>,
}

#[component]
pub fn KbEditDetails(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let tabs_ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let (item, set_item) = signal::<Option<KbEdit>>(None);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal::<Option<String>>(None);
    let articles_text = RwSignal::new(String::new());
    let active_tab = RwSignal::new("general".to_string());

    let id_store = StoredValue::new(id.clone());
    let load = move || {
        let id = id_store.get_value();
        let tabs_ctx = tabs_ctx.clone();
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            let url = format!("{}/api/a031-kb-edit/{}", api_base(), id);
            match Request::get(&url).send().await {
                Ok(resp) if resp.ok() => match resp.json::<KbEdit>().await {
                    Ok(payload) => {
                        tabs_ctx.update_tab_title(
                            &format!("a031_kb_edit_details_{}", payload.base.id.as_string()),
                            &payload.title,
                        );
                        articles_text.set(payload.target_articles.join("\n"));
                        set_item.set(Some(payload));
                    }
                    Err(e) => set_error.set(Some(format!("Ошибка парсинга: {}", e))),
                },
                Ok(resp) => set_error.set(Some(format!("Ошибка сервера: HTTP {}", resp.status()))),
                Err(e) => set_error.set(Some(format!("Ошибка сети: {}", e))),
            }
            set_loading.set(false);
        });
    };

    Effect::new(move |_| load());

    let save_articles = move || {
        let Some(current) = item.get_untracked() else {
            return;
        };
        let articles = articles_text
            .get_untracked()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        let dto = KbEditUpdateDto {
            id: Some(current.base.id.as_string()),
            code: Some(current.base.code.clone()),
            title: current.title.clone(),
            comment: current.base.comment.clone(),
            edit_type: Some(current.edit_type.as_str().to_string()),
            status: Some(current.status.as_str().to_string()),
            agent_summary: current.agent_summary.clone(),
            target_articles: articles,
            applied_articles: current.applied_articles.clone(),
            source_chat_ids: current.source_chat_ids.clone(),
            agent_id: current.agent_id.map(|id| id.as_string()),
            chat_id: current.chat_id.map(|id| id.as_string()),
            analyze_task_run_id: current.analyze_task_run_id.clone(),
            post_task_run_id: current.post_task_run_id.clone(),
        };
        spawn_local(async move {
            match Request::put(&format!(
                "{}/api/a031-kb-edit/{}",
                api_base(),
                id_store.get_value()
            ))
            .json(&dto)
            {
                Ok(req) => match req.send().await {
                    Ok(resp) if resp.ok() => load(),
                    Ok(resp) => {
                        set_error.set(Some(format!("Ошибка сохранения: HTTP {}", resp.status())))
                    }
                    Err(e) => set_error.set(Some(format!("Ошибка сети: {}", e))),
                },
                Err(e) => set_error.set(Some(format!("Ошибка запроса: {}", e))),
            }
        });
    };

    let post_action = move |action: &'static str| {
        let id = id_store.get_value();
        spawn_local(async move {
            let url = format!("{}/api/a031-kb-edit/{}/{}", api_base(), id, action);
            match Request::post(&url).send().await {
                Ok(resp) if resp.ok() => load(),
                Ok(resp) => set_error.set(Some(format!("Ошибка действия: HTTP {}", resp.status()))),
                Err(e) => set_error.set(Some(format!("Ошибка сети: {}", e))),
            }
        });
    };

    view! {
        <PageFrame page_id="a031_kb_edit_details" category=PAGE_CAT_DETAIL>
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">
                        {move || item.get().map(|i| i.title).unwrap_or_else(|| "Редактирование базы знаний".to_string())}
                    </h1>
                    {move || item.get().map(|i| {
                        view! {
                            <Badge
                                appearance=BadgeAppearance::Filled
                                color=badge_color(&i.status)
                            >
                                {i.status.display_name()}
                            </Badge>
                        }
                    })}
                </div>
                <div class="page__header-right">
                    <Space>
                        <Button
                            appearance=ButtonAppearance::Primary
                            disabled=Signal::derive(move || {
                                item.get()
                                    .map(|i| !matches!(i.status, KbEditStatus::Pending | KbEditStatus::InDialog))
                                    .unwrap_or(true)
                            })
                            on_click=move |_| post_action("approve")
                        >
                            "Утвердить"
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            disabled=Signal::derive(move || {
                                item.get()
                                    .map(|i| matches!(i.status, KbEditStatus::Closed | KbEditStatus::Cancelled))
                                    .unwrap_or(true)
                            })
                            on_click=move |_| post_action("cancel")
                        >
                            "Отменить"
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            on_click=move |_| load()
                            disabled=Signal::derive(move || loading.get())
                        >
                            "Обновить"
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            size=ButtonSize::Medium
                            on_click=move |_| on_close.run(())
                        >
                            "Закрыть"
                        </Button>
                    </Space>
                </div>
            </div>

            <div class="page__tabs">
                <button
                    class="page__tab"
                    class:page__tab--active=move || active_tab.get() == "general"
                    on:click=move |_| active_tab.set("general".to_string())
                >
                    {icon("file-text")} "Общие"
                </button>
                <button
                    class="page__tab"
                    class:page__tab--active=move || active_tab.get() == "articles"
                    on:click=move |_| active_tab.set("articles".to_string())
                >
                    {icon("book-open")} "Статьи"
                    <Badge
                        appearance=BadgeAppearance::Tint
                        color=BadgeColor::Informative
                        attr:style="margin-left: 6px;"
                    >
                        {move || item.get().map(|i| i.target_articles.len()).unwrap_or(0).to_string()}
                    </Badge>
                </button>
                <button
                    class="page__tab"
                    class:page__tab--active=move || active_tab.get() == "chat"
                    on:click=move |_| active_tab.set("chat".to_string())
                >
                    {icon("message-square")}
                    {move || {
                        if item
                            .get()
                            .map(|i| matches!(i.edit_type, KbEditType::Question))
                            .unwrap_or(false)
                        {
                            "Ответить"
                        } else {
                            "Диалог"
                        }
                    }}
                </button>
            </div>

            <div class="page__content">
                {move || {
                    if loading.get() {
                        return view! {
                            <Flex gap=FlexGap::Small style="align-items: center; padding: var(--spacing-4xl); justify-content: center;">
                                <Spinner />
                                <span>"Загрузка..."</span>
                            </Flex>
                        }.into_any();
                    }
                    if let Some(err) = error.get() {
                        return view! { <div class="alert alert--error">{err}</div> }.into_any();
                    }
                    let Some(current) = item.get() else {
                        return view! { <div>"Нет данных"</div> }.into_any();
                    };

                    match active_tab.get().as_str() {
                        "articles" => articles_tab(
                            current,
                            articles_text,
                            Callback::new(move |_: ()| save_articles()),
                        )
                        .into_any(),
                        "chat" => chat_tab(current).into_any(),
                        _ => general_tab(current).into_any(),
                    }
                }}
            </div>
        </PageFrame>
    }
}

fn badge_color(status: &KbEditStatus) -> BadgeColor {
    match status {
        KbEditStatus::Approved | KbEditStatus::Closed => BadgeColor::Success,
        KbEditStatus::Processing | KbEditStatus::InDialog => BadgeColor::Informative,
        KbEditStatus::Cancelled => BadgeColor::Danger,
        KbEditStatus::Pending => BadgeColor::Warning,
    }
}

fn readonly_input(value: String) -> impl IntoView {
    view! { <Input value=RwSignal::new(value) attr:readonly=true /> }
}

fn general_tab(current: KbEdit) -> impl IntoView {
    let status = current.status.display_name().to_string();
    let edit_type = current.edit_type.display_name().to_string();
    let is_question = matches!(&current.edit_type, KbEditType::Question);
    let summary_label = if is_question {
        "Вопрос / запрос к пользователю"
    } else {
        "Описание / концепция"
    };
    let summary_title = if is_question {
        "Запрос к пользователю"
    } else {
        "Документ"
    };
    let summary_hint = if is_question {
        Some("Это тикет для сбора знаний. Прочитайте вопросы ниже и ответьте во вкладке \"Ответить\". После обсуждения можно утвердить тикет для публикации в базу знаний.")
    } else {
        None
    };
    let created_at = current
        .base
        .metadata
        .created_at
        .format("%d.%m.%Y %H:%M:%S")
        .to_string();
    let updated_at = current
        .base
        .metadata
        .updated_at
        .format("%d.%m.%Y %H:%M:%S")
        .to_string();
    let chat_id = current
        .chat_id
        .map(|id| id.as_string())
        .unwrap_or_else(|| "—".to_string());

    view! {
        <div class="detail-grid">
            <div class="detail-grid__col">
                <CardAnimated delay_ms=0 nav_id="a031_kb_edit_details_general_document">
                    <h4 class="details-section__title">{summary_title}</h4>
                    {summary_hint.map(|hint| view! {
                        <div class="alert alert--info" style="margin-bottom: var(--spacing-md);">
                            {hint}
                        </div>
                    })}
                    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                        <div class="form__group">
                            <label class="form__label">"Код"</label>
                            {readonly_input(current.base.code)}
                        </div>
                        <div class="form__group">
                            <label class="form__label">"Статус"</label>
                            {readonly_input(status)}
                        </div>
                    </div>
                    <div class="form__group">
                        <label class="form__label">"Заголовок"</label>
                        {readonly_input(current.title)}
                    </div>
                    <div class="form__group">
                        <label class="form__label">{summary_label}</label>
                        <textarea class="form__control" rows="8" readonly>{current.agent_summary}</textarea>
                    </div>
                </CardAnimated>
            </div>

            <div class="detail-grid__col">
                <CardAnimated delay_ms=80 nav_id="a031_kb_edit_details_general_meta">
                    <h4 class="details-section__title">"Метаданные"</h4>
                    <div class="form__group">
                        <label class="form__label">"Тип"</label>
                        {readonly_input(edit_type)}
                    </div>
                    <div class="form__group">
                        <label class="form__label">"Chat ID"</label>
                        {readonly_input(chat_id)}
                    </div>
                    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                        <div class="form__group">
                            <label class="form__label">"Создано"</label>
                            {readonly_input(created_at)}
                        </div>
                        <div class="form__group">
                            <label class="form__label">"Изменено"</label>
                            {readonly_input(updated_at)}
                        </div>
                    </div>
                </CardAnimated>
            </div>
        </div>
    }
}

fn articles_tab(
    current: KbEdit,
    articles_text: RwSignal<String>,
    save_articles: Callback<()>,
) -> impl IntoView {
    let applied_articles = current.applied_articles.clone();
    view! {
        <div class="detail-grid">
            <div class="detail-grid__col">
                <CardAnimated delay_ms=0 nav_id="a031_kb_edit_details_articles_target">
                    <h4 class="details-section__title">"Статьи для изменения"</h4>
                    <div class="form__group">
                        <label class="form__label">"Target articles"</label>
                        <textarea
                            class="form__control"
                            rows="10"
                            prop:value=articles_text
                            on:input=move |ev| articles_text.set(event_target_value(&ev))
                        ></textarea>
                        <div class="form__hint">"Один относительный путь .md на строку."</div>
                    </div>
                    <div class="form__actions">
                        <Button appearance=ButtonAppearance::Primary on_click=move |_| save_articles.run(())>
                            "Сохранить список"
                        </Button>
                    </div>
                </CardAnimated>
            </div>
            <div class="detail-grid__col">
                <CardAnimated delay_ms=80 nav_id="a031_kb_edit_details_articles_applied">
                    <h4 class="details-section__title">"Опубликованные статьи"</h4>
                    {if applied_articles.is_empty() {
                        view! { <p>"Пока нет опубликованных статей."</p> }.into_any()
                    } else {
                        view! {
                            <ul>
                                {applied_articles.into_iter().map(|path| view! {
                                    <li><code>{path}</code></li>
                                }).collect_view()}
                            </ul>
                        }.into_any()
                    }}
                </CardAnimated>
            </div>
        </div>
    }
}

fn chat_tab(current: KbEdit) -> impl IntoView {
    let is_question = matches!(&current.edit_type, KbEditType::Question);
    let title = if is_question {
        "Ответ пользователя"
    } else {
        "Диалог по правке"
    };
    let hint = if is_question {
        Some("Ответьте на вопросы регистратора прямо в этом диалоге. Эти ответы станут подтверждённой основой для будущей статьи базы знаний.")
    } else {
        None
    };
    view! {
        <div class="detail-grid">
            <div class="detail-grid__col" style="grid-column: 1 / -1;">
                <CardAnimated delay_ms=0 nav_id="a031_kb_edit_details_chat_main">
                    <h4 class="details-section__title">{title}</h4>
                    {hint.map(|text| view! {
                        <div class="alert alert--info" style="margin-bottom: var(--spacing-md);">
                            {text}
                        </div>
                    })}
                    {if let Some(chat_id) = current.chat_id {
                        view! {
                            <LlmChatDetails id=chat_id.as_string() on_close=Callback::new(|_| {}) />
                        }.into_any()
                    } else {
                        view! { <p>"Чат не привязан к тикету."</p> }.into_any()
                    }}
                </CardAnimated>
            </div>
        </div>
    }
}
