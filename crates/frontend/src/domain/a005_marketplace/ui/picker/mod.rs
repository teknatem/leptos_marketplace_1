use crate::shared::icons::icon;
use contracts::domain::a005_marketplace::aggregate::Marketplace;
use contracts::domain::common::AggregateId;
use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct MarketplacePickerItem {
    pub id: String,
    pub code: String,
    pub description: String,
    pub logo_path: Option<String>,
}

impl From<Marketplace> for MarketplacePickerItem {
    fn from(m: Marketplace) -> Self {
        Self {
            id: m.base.id.as_string(),
            code: m.base.code,
            description: m.base.description,
            logo_path: m.logo_path,
        }
    }
}

#[component]
pub fn MarketplacePicker<F, G>(
    initial_selected_id: Option<String>,
    on_selected: F,
    on_cancel: G,
) -> impl IntoView
where
    F: Fn(Option<MarketplacePickerItem>) + 'static + Clone + Send,
    G: Fn(()) + 'static + Clone + Send,
{
    let (items, set_items) = signal::<Vec<MarketplacePickerItem>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (selected_id, set_selected_id) = signal::<Option<String>>(initial_selected_id);

    // Загрузка списка маркетплейсов при монтировании
    wasm_bindgen_futures::spawn_local(async move {
        match fetch_marketplaces().await {
            Ok(v) => {
                let rows: Vec<MarketplacePickerItem> = v.into_iter().map(Into::into).collect();
                set_items.set(rows);
                set_error.set(None);
            }
            Err(e) => set_error.set(Some(e)),
        }
    });

    let handle_select = {
        let on_selected = on_selected.clone();
        move |_| {
            let selected = selected_id.get();
            if let Some(id) = selected {
                let items_vec = items.get();
                if let Some(item) = items_vec.iter().find(|i| i.id == id) {
                    on_selected(Some(item.clone()));
                    return;
                }
            }
            on_selected(None);
        }
    };

    view! {
        <div class="picker">
            {move || error.get().map(|e| view! {
                <div class="warning-box" style="background: var(--color-error-50); border-color: var(--color-error-100); margin-bottom: var(--spacing-md);">
                    <span class="warning-box__icon" style="color: var(--color-error);">"⚠"</span>
                    <span class="warning-box__text" style="color: var(--color-error);">{e}</span>
                </div>
            })}

            <div class="picker__content">
                <div class="picker__grid">
                    {move || items.get().into_iter().map(|item| {
                        let item_id = item.id.clone();
                        let is_selected = move || {
                            selected_id.get().as_ref() == Some(&item_id)
                        };

                        view! {
                            <div
                                class="picker__item"
                                class:picker__item--selected={is_selected}
                                on:click={
                                    let id = item.id.clone();
                                    move |_| set_selected_id.set(Some(id.clone()))
                                }
                                on:dblclick={
                                    let on_selected = on_selected.clone();
                                    let item = item.clone();
                                    move |_| on_selected(Some(item.clone()))
                                }
                            >
                                <div class="picker__item-logo">
                                    {
                                        if let Some(logo) = &item.logo_path {
                                            view! {
                                                <img class="picker__item-image" src={logo.clone()} alt={item.description.clone()} />
                                            }.into_any()
                                        } else {
                                            view! {
                                                <div class="picker__item-icon">{icon("store")}</div>
                                            }.into_any()
                                        }
                                    }
                                </div>
                                <div class="picker__item-description">
                                    {item.description.clone()}
                                </div>
                                <div class="picker__item-code">
                                    {item.code.clone()}
                                </div>
                            </div>
                        }
                    }).collect_view()}
                </div>
            </div>

            <div class="picker__actions">
                <button
                    class="button button--primary"
                    on:click=handle_select
                    disabled={move || selected_id.get().is_none()}
                >
                    {icon("check")}
                    {"Выбрать"}
                </button>
                <button
                    class="button button--secondary"
                    on:click=move |_| on_cancel(())
                >
                    {"Отмена"}
                </button>
            </div>
        </div>
    }
}

async fn fetch_marketplaces() -> Result<Vec<Marketplace>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let api_base = || {
        let window = web_sys::window().expect("no window");
        let location = window.location();
        let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
        let hostname = location
            .hostname()
            .unwrap_or_else(|_| "127.0.0.1".to_string());
        format!("{}//{}:3000", protocol, hostname)
    };

    let url = format!("{}/api/marketplace", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let data: Vec<Marketplace> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}
