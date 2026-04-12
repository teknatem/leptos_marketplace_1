use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::date_utils::format_datetime;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use contracts::domain::a004_nomenclature::aggregate::Nomenclature;
use contracts::domain::a028_missing_cost_registry::aggregate::{
    MissingCostRegistry, MissingCostRegistryLine, MissingCostRegistryUpdateDto,
};
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::{HashMap, HashSet};
use thaw::*;

#[derive(Debug, Clone)]
struct NomenclatureInfo {
    code: String,
    description: String,
    article: String,
}

fn format_date(iso_date: &str) -> String {
    let date_part = iso_date.split('T').next().unwrap_or(iso_date);
    if let Some((year, rest)) = date_part.split_once('-') {
        if let Some((month, day)) = rest.split_once('-') {
            return format!("{}.{}.{}", day, month, year);
        }
    }
    iso_date.to_string()
}

fn format_cost(value: Option<f64>) -> String {
    value.map(|v| format!("{:.2}", v)).unwrap_or_default()
}

fn parse_cost_input(value: &str) -> Option<f64> {
    let normalized = value.trim().replace(',', ".");
    if normalized.is_empty() {
        None
    } else {
        normalized.parse::<f64>().ok()
    }
}

#[component]
pub fn MissingCostRegistryDetail(
    id: String,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    let (doc, set_doc) = signal(None::<MissingCostRegistry>);
    let (lines, set_lines) = signal(Vec::<MissingCostRegistryLine>::new());
    let (nomenclature_map, set_nomenclature_map) =
        signal(HashMap::<String, NomenclatureInfo>::new());
    let document_comment = RwSignal::new(String::new());
    let (loading, set_loading) = signal(true);
    let (saving, set_saving) = signal(false);
    let (posting, set_posting) = signal(false);
    let (error, set_error) = signal(None::<String>);
    let (success, set_success) = signal(None::<String>);

    let load_doc = {
        let id = id.clone();
        let tabs_store = tabs_store.clone();
        Callback::new(move |_| {
            let id_val = id.clone();
            set_loading.set(true);
            set_error.set(None);
            set_success.set(None);

            spawn_local(async move {
                let url = format!("{}/api/a028/missing-cost-registry/{}", api_base(), id_val);
                match Request::get(&url).send().await {
                    Ok(response) if response.ok() => match response
                        .json::<MissingCostRegistry>()
                        .await
                    {
                        Ok(data) => {
                            let tab_key = format!("a028_missing_cost_registry_details_{}", id_val);
                            let tab_title = format!("Реестр цен {}", data.document_no);
                            tabs_store.update_tab_title(&tab_key, &tab_title);

                            let parsed_lines = data.parse_lines();
                            document_comment.set(data.base.comment.clone().unwrap_or_default());
                            set_lines.set(parsed_lines.clone());
                            set_doc.set(Some(data));

                            let unique_refs = parsed_lines
                                .into_iter()
                                .map(|line| line.nomenclature_ref)
                                .collect::<HashSet<_>>();

                            let mut resolved = HashMap::new();
                            for nomenclature_ref in unique_refs {
                                let nom_url =
                                    format!("{}/api/nomenclature/{}", api_base(), nomenclature_ref);
                                if let Ok(resp) = Request::get(&nom_url).send().await {
                                    if resp.ok() {
                                        if let Ok(nom) = resp.json::<Nomenclature>().await {
                                            resolved.insert(
                                                nom.base.id.0.to_string(),
                                                NomenclatureInfo {
                                                    code: nom.base.code,
                                                    description: nom.base.description,
                                                    article: nom.article,
                                                },
                                            );
                                        }
                                    }
                                }
                            }

                            set_nomenclature_map.set(resolved);
                        }
                        Err(err) => set_error.set(Some(format!("Ошибка парсинга: {}", err))),
                    },
                    Ok(response) => {
                        set_error.set(Some(format!("Ошибка сервера: HTTP {}", response.status())))
                    }
                    Err(err) => set_error.set(Some(format!("Ошибка сети: {}", err))),
                }

                set_loading.set(false);
            });
        })
    };

    let load_doc_effect = load_doc.clone();
    Effect::new(move |_| load_doc_effect.run(()));

    let save_doc = {
        let id = id.clone();
        let load_doc = load_doc.clone();
        Callback::new(move |_| {
            set_saving.set(true);
            set_error.set(None);
            set_success.set(None);
            let id = id.clone();

            let dto = MissingCostRegistryUpdateDto {
                comment: {
                    let value = document_comment.get_untracked().trim().to_string();
                    if value.is_empty() {
                        None
                    } else {
                        Some(value)
                    }
                },
                lines: lines.get_untracked(),
            };

            spawn_local(async move {
                let url = format!("{}/api/a028/missing-cost-registry/{}", api_base(), id);
                match Request::put(&url).json(&dto) {
                    Ok(builder) => match builder.send().await {
                        Ok(response) if response.ok() => {
                            set_success.set(Some("Документ сохранен".to_string()));
                            load_doc.run(());
                        }
                        Ok(response) => set_error.set(Some(format!(
                            "Ошибка сохранения: HTTP {}",
                            response.status()
                        ))),
                        Err(err) => set_error.set(Some(format!("Ошибка сети: {}", err))),
                    },
                    Err(err) => set_error.set(Some(format!("Ошибка сериализации: {}", err))),
                }

                set_saving.set(false);
            });
        })
    };

    let post_doc = {
        let id = id.clone();
        let load_doc = load_doc.clone();
        Callback::new(move |_| {
            set_posting.set(true);
            set_error.set(None);
            let id = id.clone();

            spawn_local(async move {
                let url = format!("{}/api/a028/missing-cost-registry/{}/post", api_base(), id);
                match Request::post(&url).send().await {
                    Ok(response) if response.ok() => {
                        set_success.set(Some("Документ проведен".to_string()));
                        load_doc.run(());
                    }
                    Ok(response) => set_error.set(Some(format!(
                        "Ошибка проведения: HTTP {}",
                        response.status()
                    ))),
                    Err(err) => set_error.set(Some(format!("Ошибка сети: {}", err))),
                }

                set_posting.set(false);
            });
        })
    };

    let unpost_doc = {
        let id = id.clone();
        let load_doc = load_doc.clone();
        Callback::new(move |_| {
            set_posting.set(true);
            set_error.set(None);
            let id = id.clone();

            spawn_local(async move {
                let url = format!(
                    "{}/api/a028/missing-cost-registry/{}/unpost",
                    api_base(),
                    id
                );
                match Request::post(&url).send().await {
                    Ok(response) if response.ok() => {
                        set_success.set(Some("Проведение отменено".to_string()));
                        load_doc.run(());
                    }
                    Ok(response) => set_error.set(Some(format!(
                        "Ошибка отмены проведения: HTTP {}",
                        response.status()
                    ))),
                    Err(err) => set_error.set(Some(format!("Ошибка сети: {}", err))),
                }

                set_posting.set(false);
            });
        })
    };

    view! {
        <PageFrame page_id="a028_missing_cost_registry--detail" category="detail">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">
                        {move || {
                            doc.get()
                                .map(|d| {
                                    format!(
                                        "Реестр отсутствующих цен от {}",
                                        format_date(&d.document_date)
                                    )
                                })
                                .unwrap_or_else(|| "Реестр отсутствующих цен".to_string())
                        }}
                    </h1>
                    {move || {
                        if doc.get().map(|d| d.base.metadata.is_posted).unwrap_or(false) {
                            view! { <span class="badge badge--success">"Проведен"</span> }.into_any()
                        } else {
                            view! { <span class="badge badge--secondary">"Черновик"</span> }.into_any()
                        }
                    }}
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=move |_| save_doc.run(())
                        disabled=Signal::derive(move || saving.get() || loading.get())
                    >
                        {icon("save")}
                        " Сохранить"
                    </Button>
                    <Show
                        when=move || doc.get().map(|d| !d.base.metadata.is_posted).unwrap_or(false)
                        fallback=move || view! {
                            <Button
                                appearance=ButtonAppearance::Secondary
                                on_click=move |_| unpost_doc.run(())
                                disabled=Signal::derive(move || posting.get() || loading.get())
                            >
                                "Отменить проведение"
                            </Button>
                        }
                    >
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=move |_| post_doc.run(())
                            disabled=Signal::derive(move || posting.get() || loading.get())
                        >
                            "Провести"
                        </Button>
                    </Show>
                    <Button appearance=ButtonAppearance::Subtle on_click=move |_| on_close.run(())>
                        "Закрыть"
                    </Button>
                </div>
            </div>

            <div class="page__content">
                {move || error.get().map(|e| view! {
                    <div class="warning-box warning-box--error" style="margin-bottom: 16px;">
                        <span class="warning-box__text">{e}</span>
                    </div>
                })}

                {move || success.get().map(|e| view! {
                    <div class="info-box" style="margin-bottom: 16px;">
                        {e}
                    </div>
                })}

                <Show
                    when=move || !loading.get()
                    fallback=move || view! {
                        <div style="display:flex; align-items:center; gap:8px; padding:32px 0;">
                            <Spinner />
                            <span>"Загрузка..."</span>
                        </div>
                    }
                >
                    <Card>
                        <Flex vertical=true gap=FlexGap::Medium>
                            <div>
                                <label class="form__label">"Комментарий документа"</label>
                                <Input value=document_comment placeholder="Комментарий" />
                            </div>
                            <div style="display:flex; gap:24px; flex-wrap:wrap; color: var(--color-text-tertiary);">
                                <span>{move || format!("Строк всего: {}", lines.get().len())}</span>
                                <span>{move || {
                                    let filled = lines.get()
                                        .iter()
                                        .filter(|line| line.cost.is_some_and(|cost| cost > 0.0))
                                        .count();
                                    format!("Заполнено: {}", filled)
                                }}</span>
                                <span>{move || {
                                    let missing = lines.get().iter().filter(|line| line.cost.is_none()).count();
                                    format!("Без цены: {}", missing)
                                }}</span>
                            </div>
                        </Flex>
                    </Card>

                    <div style="margin-top: 16px;">
                        <table class="table">
                            <thead>
                                <tr>
                                    <th>"Номенклатура"</th>
                                    <th>"Артикул"</th>
                                    <th>"Себестоимость"</th>
                                    <th>"Комментарий"</th>
                                    <th>"Обнаружено"</th>
                                    <th>"Действие"</th>
                                </tr>
                            </thead>
                            <tbody>
                                <For
                                    each=move || lines.get()
                                    key=|line| line.nomenclature_ref.clone()
                                    children=move |line| {
                                        let nomenclature_ref = line.nomenclature_ref.clone();
                                        let nomenclature_ref_for_name = nomenclature_ref.clone();
                                        let nomenclature_ref_for_article = nomenclature_ref.clone();
                                        let nomenclature_ref_for_tab = nomenclature_ref.clone();
                                        let nomenclature_ref_for_cost = nomenclature_ref.clone();
                                        let nomenclature_ref_for_comment = nomenclature_ref.clone();
                                        let nomenclature_ref_for_remove = nomenclature_ref.clone();
                                        let line_comment = line.comment.clone().unwrap_or_default();
                                        let detected_at = line.detected_at.clone();
                                        let cost_value = format_cost(line.cost);
                                        let tabs_store = tabs_store.clone();

                                        view! {
                                            <tr>
                                                <td>
                                                    <a
                                                        href="#"
                                                        class="table__link"
                                                        style="color: var(--color-primary); text-decoration: underline; font-weight: 500;"
                                                        on:click=move |ev: web_sys::MouseEvent| {
                                                            ev.prevent_default();
                                                            tabs_store.open_tab(
                                                                &format!("a004_nomenclature_details_{}", nomenclature_ref_for_tab),
                                                                "Номенклатура",
                                                            );
                                                        }
                                                    >
                                                        {move || {
                                                            nomenclature_map
                                                                .get()
                                                                .get(&nomenclature_ref_for_name)
                                                                .map(|info| format!("{} {}", info.code, info.description))
                                                                .unwrap_or_else(|| nomenclature_ref_for_name.clone())
                                                        }}
                                                    </a>
                                                </td>
                                                <td>
                                                    {move || {
                                                        nomenclature_map
                                                            .get()
                                                            .get(&nomenclature_ref_for_article)
                                                            .map(|info| info.article.clone())
                                                            .unwrap_or_else(|| "—".to_string())
                                                    }}
                                                </td>
                                                <td style="min-width: 170px;">
                                                    <input
                                                        type="text"
                                                        inputmode="decimal"
                                                        prop:value=cost_value
                                                        placeholder="0.00"
                                                        style="width: 100%; padding: 8px 10px; border: 1px solid var(--color-border); border-radius: 8px; background: var(--color-bg-base); text-align: right; font-variant-numeric: tabular-nums;"
                                                        on:input=move |ev| {
                                                            let value = event_target_value(&ev);
                                                            set_lines.update(|items| {
                                                                if let Some(item) = items
                                                                    .iter_mut()
                                                                    .find(|item| item.nomenclature_ref == nomenclature_ref_for_cost)
                                                                {
                                                                    item.cost = parse_cost_input(&value);
                                                                }
                                                            });
                                                        }
                                                    />
                                                </td>
                                                <td style="min-width: 260px;">
                                                    <input
                                                        type="text"
                                                        prop:value=line_comment
                                                        placeholder="Комментарий"
                                                        style="width: 100%; padding: 8px 10px; border: 1px solid var(--color-border); border-radius: 8px; background: var(--color-bg-base);"
                                                        on:input=move |ev| {
                                                            let value = event_target_value(&ev).trim().to_string();
                                                            set_lines.update(|items| {
                                                                if let Some(item) = items
                                                                    .iter_mut()
                                                                    .find(|item| item.nomenclature_ref == nomenclature_ref_for_comment)
                                                                {
                                                                    item.comment = if value.is_empty() {
                                                                        None
                                                                    } else {
                                                                        Some(value)
                                                                    };
                                                                }
                                                            });
                                                        }
                                                    />
                                                </td>
                                                <td>{format_datetime(&detected_at)}</td>
                                                <td>
                                                    <Button
                                                        appearance=ButtonAppearance::Subtle
                                                        size=ButtonSize::Small
                                                        on_click=move |_| {
                                                            set_lines.update(|items| {
                                                                items.retain(|item| {
                                                                    item.nomenclature_ref != nomenclature_ref_for_remove
                                                                });
                                                            });
                                                        }
                                                    >
                                                        "Удалить"
                                                    </Button>
                                                </td>
                                            </tr>
                                        }
                                    }
                                />
                            </tbody>
                        </table>
                    </div>
                </Show>
            </div>
        </PageFrame>
    }
}
