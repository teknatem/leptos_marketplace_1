use crate::shared::icons::icon;
use contracts::domain::a004_nomenclature::aggregate::Nomenclature;
use contracts::domain::common::AggregateId;
use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct NomenclaturePickerItem {
    pub id: String,
    pub code: String,
    pub description: String,
    pub article: String,
}

impl From<Nomenclature> for NomenclaturePickerItem {
    fn from(n: Nomenclature) -> Self {
        Self {
            id: n.base.id.as_string(),
            code: n.base.code,
            description: n.base.description,
            article: n.article,
        }
    }
}

#[component]
pub fn NomenclaturePicker<F, G>(
    initial_selected_id: Option<String>,
    #[prop(optional)] prefiltered_items: Option<Vec<Nomenclature>>,
    #[prop(optional, into)] title: Option<String>,
    #[prop(optional, into)] subtitle: Option<String>,
    #[prop(optional, into)] search_placeholder: Option<String>,
    #[prop(optional, into)] empty_state_text: Option<String>,
    on_selected: F,
    on_cancel: G,
) -> impl IntoView
where
    F: Fn(Option<NomenclaturePickerItem>) + 'static + Clone + Send,
    G: Fn(()) + 'static + Clone + Send,
{
    let (items, set_items) = signal::<Vec<NomenclaturePickerItem>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (selected_id, set_selected_id) = signal::<Option<String>>(initial_selected_id);
    let (search_filter, set_search_filter) = signal::<String>(String::new());

    let title = title.unwrap_or_else(|| "Выбор номенклатуры 1С УТ".to_string());
    let subtitle = subtitle.unwrap_or_else(|| {
        "Найдите и выберите позицию 1С, которую нужно связать с текущим товаром маркетплейса."
            .to_string()
    });
    let search_placeholder = search_placeholder
        .unwrap_or_else(|| "Поиск по артикулу, коду или наименованию".to_string());
    let empty_state_text =
        empty_state_text.unwrap_or_else(|| "Подходящие позиции не найдены".to_string());

    if let Some(prefilt) = prefiltered_items {
        let rows: Vec<NomenclaturePickerItem> = prefilt.into_iter().map(Into::into).collect();
        set_items.set(rows);
    } else {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_nomenclature().await {
                Ok(v) => {
                    let rows: Vec<NomenclaturePickerItem> = v.into_iter().map(Into::into).collect();
                    set_items.set(rows);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    }

    let filtered_items = move || {
        let filter = search_filter.get().to_lowercase();
        items
            .get()
            .into_iter()
            .filter(|item| !item.article.trim().is_empty())
            .filter(|item| {
                if filter.is_empty() {
                    true
                } else {
                    item.description.to_lowercase().contains(&filter)
                        || item.code.to_lowercase().contains(&filter)
                        || item.article.to_lowercase().contains(&filter)
                }
            })
            .collect::<Vec<_>>()
    };

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
        <div class="nomenclature-picker">
            <div class="nomenclature-picker__header">
                <div>
                    <h3 class="nomenclature-picker__title">{title}</h3>
                    <p class="nomenclature-picker__subtitle">{subtitle}</p>
                </div>
                <div class="nomenclature-picker__summary">
                    {move || format!("Доступно: {}", filtered_items().len())}
                </div>
            </div>

            {move || error.get().map(|e| view! {
                <div class="warning-box warning-box--error nomenclature-picker__error">
                    <span class="warning-box__icon">"⚠"</span>
                    <span class="warning-box__text">{e}</span>
                </div>
            })}

            <div class="nomenclature-picker__toolbar">
                <input
                    type="text"
                    class="form__input nomenclature-picker__search"
                    placeholder=search_placeholder.clone()
                    prop:value={move || search_filter.get()}
                    on:input=move |ev| {
                        set_search_filter.set(event_target_value(&ev));
                    }
                />
                <div class="nomenclature-picker__hint">
                    "Двойной клик по строке сразу выбирает позицию."
                </div>
            </div>

            <div class="nomenclature-picker__content">
                {move || {
                    let filtered = filtered_items();
                    if filtered.is_empty() {
                        view! {
                            <div class="nomenclature-picker__empty">{empty_state_text.clone()}</div>
                        }.into_any()
                    } else {
                        view! {
                            <table class="table__data nomenclature-picker__table">
                                <thead class="table__head">
                                    <tr>
                                        <th class="table__header-cell nomenclature-picker__head-cell nomenclature-picker__head-cell--article">"Артикул"</th>
                                        <th class="table__header-cell nomenclature-picker__head-cell nomenclature-picker__head-cell--code">"Код 1С"</th>
                                        <th class="table__header-cell nomenclature-picker__head-cell">"Наименование"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {filtered.into_iter().map(|item| {
                                        let item_id = item.id.clone();
                                        let is_selected = move || selected_id.get().as_ref() == Some(&item_id);

                                        view! {
                                            <tr
                                                class="table__row nomenclature-picker__row"
                                                class:nomenclature-picker__row--selected=move || is_selected()
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
                                                <td class="table__cell nomenclature-picker__cell nomenclature-picker__cell--mono">{item.article.clone()}</td>
                                                <td class="table__cell nomenclature-picker__cell">{item.code.clone()}</td>
                                                <td class="table__cell nomenclature-picker__cell">{item.description.clone()}</td>
                                            </tr>
                                        }
                                    }).collect_view()}
                                </tbody>
                            </table>
                        }.into_any()
                    }
                }}
            </div>

            <div class="nomenclature-picker__footer">
                <button class="button button--secondary" on:click=move |_| on_cancel(())>
                    {"Отмена"}
                </button>
                <button
                    class="button button--primary"
                    on:click=handle_select
                    disabled={move || selected_id.get().is_none()}
                >
                    {icon("check")}
                    {"Выбрать позицию"}
                </button>
            </div>
        </div>
    }
}

async fn fetch_nomenclature() -> Result<Vec<Nomenclature>, String> {
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

    let url = format!("{}/api/nomenclature", api_base());
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
    let data: Vec<Nomenclature> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;

    Ok(data
        .into_iter()
        .filter(|n| !n.is_folder && !n.base.metadata.is_deleted)
        .collect())
}
