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
    /// Optional: pre-filtered items (e.g., from search results)
    #[prop(optional)]
    prefiltered_items: Option<Vec<Nomenclature>>,
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

    // Загрузка списка номенклатуры при монтировании
    if let Some(prefilt) = prefiltered_items {
        // Использовать предварительно отфильтрованный список
        let rows: Vec<NomenclaturePickerItem> = prefilt.into_iter().map(Into::into).collect();
        set_items.set(rows);
    } else {
        // Загрузить все элементы номенклатуры
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

    // Фильтрованный список (исключаем позиции без артикула)
    let filtered_items = move || {
        let filter = search_filter.get().to_lowercase();
        let all_items = items.get();

        let filtered: Vec<_> = all_items
            .into_iter()
            .filter(|item| !item.article.trim().is_empty()) // Исключаем без артикула
            .filter(|item| {
                if filter.is_empty() {
                    true
                } else {
                    item.description.to_lowercase().contains(&filter)
                        || item.code.to_lowercase().contains(&filter)
                        || item.article.to_lowercase().contains(&filter)
                }
            })
            .collect();

        filtered
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
        <div class="picker-container" style="width: 80%; max-width: 1000px; height: 80vh; max-height: 700px; display: flex; flex-direction: column; background: white; border-radius: 8px; box-shadow: 0 4px 12px rgba(0,0,0,0.15);">
            <div class="picker-header" style="padding: 20px; border-bottom: 2px solid var(--color-primary, #4a90e2); background: linear-gradient(to bottom, #fff, #f9f9f9);">
                <h3 style="margin: 0; color: var(--color-primary, #4a90e2); font-size: 1.3rem;">{"Выбор номенклатуры"}</h3>
            </div>

            {move || error.get().map(|e| view! { <div class="error">{e}</div> })}

            <div style="padding: 12px 20px; background: #f9f9f9; border-bottom: 1px solid #e0e0e0;">
                <input
                    type="text"
                    placeholder="🔍 Поиск по артикулу, коду или названию..."
                    prop:value={move || search_filter.get()}
                    on:input=move |ev| {
                        set_search_filter.set(event_target_value(&ev));
                    }
                    style="width: 100%; padding: 10px 12px; border: 1px solid #ddd; border-radius: 4px; font-size: 14px;"
                />
            </div>

            <div class="picker-content" style="overflow: auto; max-height: 500px;">
                {move || {
                    let filtered = filtered_items();
                    if filtered.is_empty() {
                        view! {
                            <div style="padding: 40px; text-align: center; color: #666;">
                                {"Номенклатура не найдена"}
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <table style="width: 100%; border-collapse: collapse; background: white;">
                                <thead style="position: sticky; top: 0; background: #f5f5f5; z-index: 1;">
                                    <tr style="border-bottom: 2px solid #ddd;">
                                        <th style="padding: 12px 8px; text-align: left; font-weight: 600; color: #333; width: 120px;">{"Артикул"}</th>
                                        <th style="padding: 12px 8px; text-align: left; font-weight: 600; color: #333; width: 120px;">{"Код"}</th>
                                        <th style="padding: 12px 8px; text-align: left; font-weight: 600; color: #333;">{"Наименование"}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {filtered.into_iter().map(|item| {
                                        let item_id = item.id.clone();
                                        let is_selected = move || {
                                            selected_id.get().as_ref() == Some(&item_id)
                                        };

                                        view! {
                                            <tr
                                                style={move || {
                                                    if is_selected() {
                                                        "background: #e3f2fd; cursor: pointer; border-bottom: 1px solid #ddd;"
                                                    } else {
                                                        "cursor: pointer; border-bottom: 1px solid #eee;"
                                                    }
                                                }}
                                                style:hover="background: #f5f5f5"
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
                                                <td style="padding: 10px 8px; color: #555; font-family: monospace;">{item.article.clone()}</td>
                                                <td style="padding: 10px 8px; color: #555;">{item.code.clone()}</td>
                                                <td style="padding: 10px 8px; color: #333;">{item.description.clone()}</td>
                                            </tr>
                                        }
                                    }).collect_view()}
                                </tbody>
                            </table>
                        }.into_any()
                    }
                }}
            </div>

            <div style="padding: 16px 20px; border-top: 1px solid #e0e0e0; display: flex; justify-content: flex-end; gap: 12px; background: #f9f9f9;">
                <button
                    class="btn btn-primary"
                    on:click=handle_select
                    disabled={move || selected_id.get().is_none()}
                    style="padding: 10px 24px; font-size: 14px;"
                >
                    {icon("check")}
                    {"Выбрать"}
                </button>
                <button
                    class="btn btn-secondary"
                    on:click=move |_| on_cancel(())
                    style="padding: 10px 24px; font-size: 14px;"
                >
                    {"Отмена"}
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

    // Фильтруем только элементы (не папки) и не удаленные
    let filtered: Vec<Nomenclature> = data
        .into_iter()
        .filter(|n| !n.is_folder && !n.base.metadata.is_deleted)
        .collect();

    Ok(filtered)
}
