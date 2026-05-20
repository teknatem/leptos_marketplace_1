use contracts::system::favorites::{
    FavoriteDto, FavoriteUpdateRequest, FavoriteUpsertRequest, FAVORITE_COLOR_YELLOW,
};
use gloo_timers::future::TimeoutFuture;
use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

use crate::layout::global_context::AppGlobalContext;
use crate::shared::modal_frame::ModalFrame;
use crate::system::auth::context::use_auth;

use super::api;

fn color_hex(color: &str) -> &'static str {
    match color {
        "red" => "#ef4444",
        "green" => "#22c55e",
        "blue" => "#3b82f6",
        "purple" => "#a855f7",
        "gray" => "#9ca3af",
        _ => "#f5c542",
    }
}

fn text_area_value(ev: ev::Event) -> String {
    ev.target()
        .and_then(|target| target.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
        .map(|target| target.value())
        .unwrap_or_default()
}

fn checkbox_checked(ev: ev::Event) -> bool {
    ev.target()
        .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|target| target.checked())
        .unwrap_or(false)
}

fn star_icon(filled: bool, color: &str) -> AnyView {
    let fill = if filled { color } else { "none" };
    let stroke = if filled { color } else { "currentColor" };
    view! {
        <svg
            class="favorite-star-svg"
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill=fill
            stroke=stroke
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <path d="M11.48 3.5a.6.6 0 0 1 1.04 0l2.6 5.27a.6.6 0 0 0 .45.33l5.82.85a.6.6 0 0 1 .33 1.02l-4.21 4.1a.6.6 0 0 0-.17.53l.99 5.79a.6.6 0 0 1-.87.63l-5.2-2.73a.6.6 0 0 0-.56 0l-5.2 2.73a.6.6 0 0 1-.87-.63l.99-5.79a.6.6 0 0 0-.17-.53l-4.21-4.1a.6.6 0 0 1 .33-1.02l5.82-.85a.6.6 0 0 0 .45-.33l2.6-5.27Z"/>
        </svg>
    }
    .into_any()
}

fn edit_icon() -> AnyView {
    view! {
        <svg
            class="favorite-action-svg"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <path d="M12 20h9"/>
            <path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z"/>
        </svg>
    }
    .into_any()
}

fn scope_icon(is_global: bool) -> AnyView {
    if is_global {
        view! {
            <svg
                class="favorite-scope-svg"
                width="13"
                height="13"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                aria-hidden="true"
            >
                <circle cx="12" cy="12" r="10"/>
                <path d="M2 12h20"/>
                <path d="M12 2a15.3 15.3 0 0 1 0 20"/>
                <path d="M12 2a15.3 15.3 0 0 0 0 20"/>
            </svg>
        }
        .into_any()
    } else {
        view! {
            <svg
                class="favorite-scope-svg"
                width="13"
                height="13"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                aria-hidden="true"
            >
                <path d="M20 21a8 8 0 0 0-16 0"/>
                <circle cx="12" cy="7" r="4"/>
            </svg>
        }
        .into_any()
    }
}

#[component]
fn ColorPicker(color: RwSignal<String>) -> impl IntoView {
    let button_class = move |name: &'static str| {
        move || {
            if color.get() == name {
                "favorite-color favorite-color--active"
            } else {
                "favorite-color"
            }
        }
    };

    view! {
        <div class="favorite-color-row">
            <button class=button_class("yellow") style="background: #f5c542;" title="yellow" on:click=move |_| color.set("yellow".to_string()) />
            <button class=button_class("red") style="background: #ef4444;" title="red" on:click=move |_| color.set("red".to_string()) />
            <button class=button_class("green") style="background: #22c55e;" title="green" on:click=move |_| color.set("green".to_string()) />
            <button class=button_class("blue") style="background: #3b82f6;" title="blue" on:click=move |_| color.set("blue".to_string()) />
            <button class=button_class("purple") style="background: #a855f7;" title="purple" on:click=move |_| color.set("purple".to_string()) />
            <button class=button_class("gray") style="background: #9ca3af;" title="gray" on:click=move |_| color.set("gray".to_string()) />
        </div>
    }
}

#[component]
fn FavoriteFormModal(
    title: String,
    color: RwSignal<String>,
    comment: RwSignal<String>,
    is_global: RwSignal<bool>,
    saving: RwSignal<bool>,
    error: RwSignal<Option<String>>,
    can_delete: bool,
    on_save: Callback<()>,
    on_delete: Callback<()>,
    on_close: Callback<()>,
) -> impl IntoView {
    view! {
        <ModalFrame
            on_close=on_close
            modal_style="width: min(520px, calc(100vw - 32px)); max-height: calc(100vh - 48px); overflow: auto;".to_string()
            modal_class="favorite-modal".to_string()
        >
            <div class="favorite-modal__header">
                <h3>"Избранное"</h3>
                <button class="favorite-modal__close" on:click=move |_| on_close.run(())>"x"</button>
            </div>

            <div class="favorite-modal__body">
                <div class="favorite-modal__target">{title}</div>
                <div class="favorite-modal__label">"Цвет"</div>
                <ColorPicker color=color />

                <label class="favorite-modal__label" for="favorite-comment">"Комментарий"</label>
                <textarea
                    id="favorite-comment"
                    class="favorite-comment"
                    prop:value=move || comment.get()
                    on:input=move |ev| comment.set(text_area_value(ev))
                    placeholder="Комментарий к ссылке"
                ></textarea>

                <label class="favorite-checkbox">
                    <input
                        type="checkbox"
                        prop:checked=move || is_global.get()
                        on:change=move |ev| is_global.set(checkbox_checked(ev))
                    />
                    <span>"Глобальная"</span>
                </label>

                <Show when=move || error.get().is_some()>
                    <div class="favorite-modal__error">{move || error.get().unwrap_or_default()}</div>
                </Show>
            </div>

            <div class="favorite-modal__actions">
                <Show when=move || can_delete>
                    <button
                        class="button button--secondary"
                        on:click=move |_| on_delete.run(())
                        disabled=move || saving.get()
                    >
                        "Удалить"
                    </button>
                </Show>
                <div style="flex: 1;"></div>
                <button class="button button--secondary" on:click=move |_| on_close.run(()) disabled=move || saving.get()>
                    "Отмена"
                </button>
                <button class="button button--primary" on:click=move |_| on_save.run(()) disabled=move || saving.get()>
                    {move || if saving.get() { "Сохранение..." } else { "Сохранить" }}
                </button>
            </div>
        </ModalFrame>
    }
}

#[component]
pub fn FavoriteButton(
    target_kind: String,
    target_id: String,
    #[prop(into)] target_title: Signal<String>,
    tab_key: String,
) -> impl IntoView {
    let favorite = RwSignal::new(None::<FavoriteDto>);
    let loading = RwSignal::new(false);
    let saving = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let modal_open = RwSignal::new(false);
    let draft_color = RwSignal::new(FAVORITE_COLOR_YELLOW.to_string());
    let draft_comment = RwSignal::new(String::new());
    let draft_global = RwSignal::new(false);

    let target_kind_for_load = target_kind.clone();
    let target_id_for_load = target_id.clone();
    Effect::new(move |_| {
        loading.set(true);
        error.set(None);
        let target_kind = target_kind_for_load.clone();
        let target_id = target_id_for_load.clone();
        spawn_local(async move {
            match api::get_target_favorite(&target_kind, &target_id).await {
                Ok(value) => favorite.set(value),
                Err(err) => error.set(Some(err)),
            }
            loading.set(false);
        });
    });

    let open_modal = move |_| {
        if let Some(item) = favorite.get_untracked() {
            draft_color.set(item.color);
            draft_comment.set(item.comment.unwrap_or_default());
            draft_global.set(item.is_global);
        } else {
            draft_color.set(FAVORITE_COLOR_YELLOW.to_string());
            draft_comment.set(String::new());
            draft_global.set(false);
        }
        error.set(None);
        modal_open.set(true);
    };

    let save = {
        let target_kind = target_kind.clone();
        let target_id = target_id.clone();
        let tab_key = tab_key.clone();
        move |_: ()| {
            saving.set(true);
            error.set(None);
            let req = FavoriteUpsertRequest {
                target_kind: target_kind.clone(),
                target_id: target_id.clone(),
                target_title: target_title.get_untracked(),
                tab_key: tab_key.clone(),
                color: draft_color.get_untracked(),
                comment: Some(draft_comment.get_untracked()),
                is_global: draft_global.get_untracked(),
            };
            spawn_local(async move {
                match api::upsert_favorite(req).await {
                    Ok(value) => {
                        favorite.set(Some(value));
                        modal_open.set(false);
                    }
                    Err(err) => error.set(Some(err)),
                }
                saving.set(false);
            });
        }
    };

    let delete = move |_: ()| {
        let Some(item) = favorite.get_untracked() else {
            modal_open.set(false);
            return;
        };
        saving.set(true);
        error.set(None);
        spawn_local(async move {
            match api::delete_favorite(&item.id).await {
                Ok(_) => {
                    favorite.set(None);
                    modal_open.set(false);
                }
                Err(err) => error.set(Some(err)),
            }
            saving.set(false);
        });
    };

    let save_cb = Callback::new(save);
    let delete_cb = Callback::new(delete);

    view! {
        <button
            class="favorite-star-button"
            on:click=open_modal
            title=move || if favorite.get().is_some() { "Редактировать избранное" } else { "Добавить в избранное" }
            disabled=move || loading.get()
        >
            {move || {
                if let Some(item) = favorite.get() {
                    star_icon(true, color_hex(&item.color))
                } else {
                    star_icon(false, "currentColor")
                }
            }}
        </button>

        <Show when=move || modal_open.get()>
            <FavoriteFormModal
                title=target_title.get_untracked()
                color=draft_color
                comment=draft_comment
                is_global=draft_global
                saving=saving
                error=error
                can_delete=favorite.get_untracked().is_some()
                on_save=save_cb
                on_delete=delete_cb
                on_close=Callback::new(move |_| modal_open.set(false))
            />
        </Show>
    }
}

#[component]
pub fn FavoritesHeaderButton() -> impl IntoView {
    let modal_open = RwSignal::new(false);
    let modal_closing = RwSignal::new(false);
    let close_modal = Callback::new(move |_| {
        if modal_closing.get_untracked() {
            return;
        }
        modal_closing.set(true);
        spawn_local(async move {
            TimeoutFuture::new(180).await;
            modal_open.set(false);
            modal_closing.set(false);
        });
    });

    view! {
        <button
            class="app-header__icon-button"
            on:click=move |_| {
                modal_closing.set(false);
                modal_open.set(true);
            }
            title="Избранное"
        >
            {star_icon(false, "currentColor")}
        </button>
        <Show when=move || modal_open.get()>
            <FavoritesModal on_close=close_modal closing=modal_closing />
        </Show>
    }
}

#[component]
fn FavoritesModal(on_close: Callback<()>, closing: RwSignal<bool>) -> impl IntoView {
    let items = RwSignal::new(Vec::<FavoriteDto>::new());
    let loading = RwSignal::new(true);
    let error = RwSignal::new(None::<String>);
    let saving = RwSignal::new(false);
    let edit_item = RwSignal::new(None::<FavoriteDto>);
    let draft_color = RwSignal::new(FAVORITE_COLOR_YELLOW.to_string());
    let draft_comment = RwSignal::new(String::new());
    let draft_global = RwSignal::new(false);
    let tabs = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let (auth_state, _) = use_auth();

    let reload = move || {
        loading.set(true);
        error.set(None);
        spawn_local(async move {
            match api::list_favorites().await {
                Ok(value) => items.set(value),
                Err(err) => error.set(Some(err)),
            }
            loading.set(false);
        });
    };
    Effect::new(move |_| reload());

    let can_edit = move |item: &FavoriteDto| {
        auth_state.with_untracked(|state| {
            state
                .user_info
                .as_ref()
                .map(|user| user.is_admin || user.id == item.owner_user_id)
                .unwrap_or(false)
        })
    };

    let save_edit = move |_: ()| {
        let Some(item) = edit_item.get_untracked() else {
            return;
        };
        saving.set(true);
        error.set(None);
        let req = FavoriteUpdateRequest {
            color: draft_color.get_untracked(),
            comment: Some(draft_comment.get_untracked()),
            is_global: draft_global.get_untracked(),
        };
        spawn_local(async move {
            match api::update_favorite(&item.id, req).await {
                Ok(updated) => {
                    items.update(|rows| {
                        if let Some(row) = rows.iter_mut().find(|row| row.id == updated.id) {
                            *row = updated;
                        }
                    });
                    edit_item.set(None);
                }
                Err(err) => error.set(Some(err)),
            }
            saving.set(false);
        });
    };

    let delete_edit = move |_: ()| {
        let Some(item) = edit_item.get_untracked() else {
            return;
        };
        saving.set(true);
        error.set(None);
        let id = item.id;
        spawn_local(async move {
            match api::delete_favorite(&id).await {
                Ok(_) => {
                    items.update(|rows| rows.retain(|row| row.id != id));
                    edit_item.set(None);
                }
                Err(err) => error.set(Some(err)),
            }
            saving.set(false);
        });
    };

    view! {
        <ModalFrame
            on_close=on_close
            overlay_style="align-items: stretch; justify-content: flex-end; padding: 0;".to_string()
            overlay_class_signal=Signal::derive(move || {
                if closing.get() {
                    "favorite-drawer-overlay favorite-drawer-overlay--closing".to_string()
                } else {
                    "favorite-drawer-overlay".to_string()
                }
            })
            modal_style="width: min(560px, 100vw); max-width: min(560px, 100vw); height: 100vh; max-height: 100vh; border-radius: 0; overflow: hidden;".to_string()
            modal_class_signal=Signal::derive(move || {
                if closing.get() {
                    "favorite-modal favorite-modal--list favorite-drawer favorite-drawer--closing".to_string()
                } else {
                    "favorite-modal favorite-modal--list favorite-drawer".to_string()
                }
            })
        >
            <div class="favorite-modal__header">
                <h3>"Избранное"</h3>
                <button class="favorite-modal__close" on:click=move |_| on_close.run(())>"x"</button>
            </div>
            <div class="favorite-modal__body">
                <Show when=move || loading.get()>
                    <div class="favorite-modal__loading">"Загрузка..."</div>
                </Show>
                <Show when=move || error.get().is_some()>
                    <div class="favorite-modal__error">{move || error.get().unwrap_or_default()}</div>
                </Show>
                <Show when=move || !loading.get() && items.get().is_empty()>
                    <div class="favorite-modal__empty">"Нет избранных ссылок"</div>
                </Show>
                <div class="favorite-list">
                    <For
                        each=move || items.get()
                        key=|item| item.id.clone()
                        children=move |item| {
                            let open_item = item.clone();
                            let edit_source = StoredValue::new(item.clone());
                            let editable = can_edit(&item);
                            let owner = item.owner_username.clone().unwrap_or(item.owner_user_id.clone());
                            let scope_title = if item.is_global { "Глобальная" } else { "Личная" };
                            view! {
                                <div class="favorite-list__item">
                                    <button
                                        class="favorite-list__main"
                                        on:click=move |_| {
                                            tabs.open_tab(&open_item.tab_key, &open_item.target_title);
                                            on_close.run(());
                                        }
                                    >
                                        <span class="favorite-list__star">{star_icon(true, color_hex(&item.color))}</span>
                                        <span class="favorite-list__text">
                                            <span class="favorite-list__topline">
                                                <span class="favorite-list__title">{item.target_title.clone()}</span>
                                                <span class="favorite-list__meta" title=scope_title>
                                                    {scope_icon(item.is_global)}
                                                    <span>{owner}</span>
                                                </span>
                                            </span>
                                            <Show when={
                                                let comment = item.comment.clone();
                                                move || comment.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
                                            }>
                                                <span class="favorite-list__comment">{item.comment.clone().unwrap_or_default()}</span>
                                            </Show>
                                        </span>
                                    </button>
                                    <Show when=move || editable>
                                        <div class="favorite-list__actions">
                                            <button
                                                class="favorite-icon-button"
                                                title="Править"
                                                on:click=move |_| {
                                                    let source = edit_source.get_value();
                                                    draft_color.set(source.color.clone());
                                                    draft_comment.set(source.comment.clone().unwrap_or_default());
                                                    draft_global.set(source.is_global);
                                                    edit_item.set(Some(source));
                                                }
                                            >
                                                {edit_icon()}
                                            </button>
                                        </div>
                                    </Show>
                                </div>
                            }
                        }
                    />
                </div>
            </div>
        </ModalFrame>

        <Show when=move || edit_item.get().is_some()>
            <FavoriteFormModal
                title=edit_item.get_untracked().map(|item| item.target_title).unwrap_or_default()
                color=draft_color
                comment=draft_comment
                is_global=draft_global
                saving=saving
                error=error
                can_delete=true
                on_save=Callback::new(save_edit)
                on_delete=Callback::new(delete_edit)
                on_close=Callback::new(move |_| edit_item.set(None))
            />
        </Show>
    }
}
