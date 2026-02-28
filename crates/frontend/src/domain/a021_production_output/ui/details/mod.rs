use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use contracts::domain::a021_production_output::aggregate::ProductionOutput;
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use thaw::*;

fn format_date(s: &str) -> String {
    if let Some((year, rest)) = s.split_once('-') {
        if let Some((month, day)) = rest.split_once('-') {
            return format!("{}.{}.{}", day, month, year);
        }
    }
    s.to_string()
}

fn format_datetime(s: &str) -> String {
    if let Some((date, time)) = s.split_once('T') {
        let time_clean = time
            .split('Z').next()
            .unwrap_or(time)
            .split('+').next()
            .unwrap_or(time);
        let hms = time_clean.split('.').next().unwrap_or(time_clean);
        return format!("{} {}", format_date(date), hms);
    }
    format_date(s)
}

fn format_money(v: f64) -> String {
    format!("{:.2}", v)
}

#[derive(Debug, Clone, Deserialize)]
struct NomInfo {
    pub id: String,
    pub code: String,
    pub description: String,
    pub article: String,
}

#[component]
pub fn ProductionOutputDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let stored_id = StoredValue::new(id.clone());

    let (doc, set_doc) = signal(None::<ProductionOutput>);
    let (nom_info, set_nom_info) = signal(None::<NomInfo>);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);
    let (posting, set_posting) = signal(false);

    let load_doc = {
        let stored_id = stored_id;
        let tabs_store = tabs_store.clone();
        move || {
            let id_val = stored_id.get_value();
            set_loading.set(true);
            set_error.set(None);
            set_nom_info.set(None);
            spawn_local(async move {
                let url = format!("{}/api/a021/production-output/{}", api_base(), id_val);
                match Request::get(&url).send().await {
                    Ok(response) if response.ok() => {
                        match response.json::<ProductionOutput>().await {
                            Ok(data) => {
                                let tab_key = format!("a021_production_output_detail_{}", id_val);
                                let tab_title = format!("Выпуск {}", data.document_no);
                                tabs_store.update_tab_title(&tab_key, &tab_title);

                                // Загружаем номенклатуру, если есть ссылка
                                if let Some(ref nom_id) = data.nomenclature_ref {
                                    let nom_url = format!("{}/api/nomenclature/{}", api_base(), nom_id);
                                    if let Ok(nom_resp) = Request::get(&nom_url).send().await {
                                        if nom_resp.ok() {
                                            #[derive(Deserialize)]
                                            struct NomFull {
                                                pub id: String,
                                                pub code: String,
                                                pub description: String,
                                                pub article: String,
                                            }
                                            if let Ok(n) = nom_resp.json::<NomFull>().await {
                                                set_nom_info.set(Some(NomInfo {
                                                    id: n.id,
                                                    code: n.code,
                                                    description: n.description,
                                                    article: n.article,
                                                }));
                                            }
                                        }
                                    }
                                }

                                set_doc.set(Some(data));
                            }
                            Err(e) => set_error.set(Some(format!("Ошибка парсинга: {}", e))),
                        }
                    }
                    Ok(r) => set_error.set(Some(format!("HTTP {}", r.status()))),
                    Err(e) => set_error.set(Some(format!("Ошибка сети: {}", e))),
                }
                set_loading.set(false);
            });
        }
    };

    let load_doc_clone = load_doc.clone();
    Effect::new(move || {
        load_doc_clone();
    });

    let post_doc = {
        let stored_id = stored_id;
        let load_doc = load_doc.clone();
        move || {
            let id_val = stored_id.get_value();
            set_posting.set(true);
            set_error.set(None);
            let load_doc = load_doc.clone();
            spawn_local(async move {
                let url = format!("{}/api/a021/production-output/{}/post", api_base(), id_val);
                match Request::post(&url).send().await {
                    Ok(r) if r.ok() => load_doc(),
                    Ok(r) => set_error.set(Some(format!("Ошибка проведения: HTTP {}", r.status()))),
                    Err(e) => set_error.set(Some(format!("Ошибка сети: {}", e))),
                }
                set_posting.set(false);
            });
        }
    };

    let unpost_doc = {
        let stored_id = stored_id;
        let load_doc = load_doc.clone();
        move || {
            let id_val = stored_id.get_value();
            set_posting.set(true);
            set_error.set(None);
            let load_doc = load_doc.clone();
            spawn_local(async move {
                let url = format!("{}/api/a021/production-output/{}/unpost", api_base(), id_val);
                match Request::post(&url).send().await {
                    Ok(r) if r.ok() => load_doc(),
                    Ok(r) => set_error.set(Some(format!("Ошибка отмены проведения: HTTP {}", r.status()))),
                    Err(e) => set_error.set(Some(format!("Ошибка сети: {}", e))),
                }
                set_posting.set(false);
            });
        }
    };

    view! {
        <PageFrame page_id="a021_production_output--detail" category="detail">
            {move || {
                let doc_title = doc.get()
                    .map(|d| format!("Выпуск {} от {}", d.document_no, format_date(&d.document_date)))
                    .unwrap_or_else(|| "Выпуск продукции".to_string());
                let is_posted = doc.get().map(|d| d.base.metadata.is_posted).unwrap_or(false);
                let doc_loaded = doc.get().is_some();
                let post_doc = post_doc.clone();
                let unpost_doc = unpost_doc.clone();
                view! {
                    <div class="page__header">
                        <div class="page__header-left">
                            <h1 class="page__title">{doc_title}</h1>
                            {if doc_loaded {
                                if is_posted {
                                    view! {
                                        <span class="badge badge--success">"Проведён"</span>
                                    }.into_any()
                                } else {
                                    view! {
                                        <span class="badge badge--secondary">"Не проведён"</span>
                                    }.into_any()
                                }
                            } else {
                                view! { <span></span> }.into_any()
                            }}
                        </div>
                        <div class="page__header-right">
                            {if doc_loaded {
                                if is_posted {
                                    view! {
                                        <Button
                                            appearance=ButtonAppearance::Secondary
                                            on_click=move |_| unpost_doc()
                                            disabled=Signal::derive(move || posting.get())
                                        >
                                            {icon("x-circle")}
                                            " Отменить проведение"
                                        </Button>
                                    }.into_any()
                                } else {
                                    view! {
                                        <Button
                                            appearance=ButtonAppearance::Primary
                                            on_click=move |_| post_doc()
                                            disabled=Signal::derive(move || posting.get())
                                        >
                                            {icon("check-circle")}
                                            " Провести"
                                        </Button>
                                    }.into_any()
                                }
                            } else {
                                view! { <span></span> }.into_any()
                            }}
                            <Button
                                appearance=ButtonAppearance::Subtle
                                on_click=move |_| on_close.run(())
                            >
                                "✕ Закрыть"
                            </Button>
                        </div>
                    </div>
                }
            }}

            <div class="page__content">
                {move || {
                    if loading.get() {
                        return view! {
                            <Flex gap=FlexGap::Small style="align-items:center;padding:var(--spacing-4xl);justify-content:center;">
                                <Spinner />
                                <span>"Загрузка..."</span>
                            </Flex>
                        }.into_any();
                    }
                    if let Some(err) = error.get() {
                        return view! {
                            <div style="padding:var(--spacing-lg);background:var(--color-error-50);border:1px solid var(--color-error-100);border-radius:var(--radius-sm);color:var(--color-error);margin:var(--spacing-lg);">
                                <strong>"Ошибка: "</strong>{err}
                            </div>
                        }.into_any();
                    }
                    if let Some(d) = doc.get() {
                        let nom_status = if d.nomenclature_ref.is_some() {
                            ("badge badge--success", "Привязана")
                        } else {
                            ("badge badge--warning", "Не привязана")
                        };

                        let nom_ref_clone = d.nomenclature_ref.clone();
                        let tabs_store_nom = tabs_store.clone();

                        view! {
                            <div style="padding:var(--spacing-lg);display:flex;flex-direction:column;gap:var(--spacing-lg);">
                                <Card>
                                    <div style="padding:var(--spacing-md);display:grid;grid-template-columns:max-content 1fr;gap:var(--spacing-sm) var(--spacing-xl);align-items:baseline;">
                                        <span class="form__label">"Номер документа:"</span>
                                        <strong style="font-size:var(--font-size-lg);">{d.document_no.clone()}</strong>

                                        <span class="form__label">"Дата производства:"</span>
                                        <span>{format_date(&d.document_date)}</span>

                                        <span class="form__label">"Наименование:"</span>
                                        <span>{d.base.description.clone()}</span>

                                        <span class="form__label">"Артикул:"</span>
                                        <code style="font-family:monospace;">{d.article.clone()}</code>

                                        <span class="form__label">"Количество:"</span>
                                        <strong>{d.count}</strong>

                                        <span class="form__label">"Сумма себестоимости:"</span>
                                        <span>{format_money(d.amount)}</span>

                                        <span class="form__label">"С/с на 1 шт:"</span>
                                        <span>
                                            {d.cost_of_production.map(format_money).unwrap_or_else(|| "—".to_string())}
                                        </span>
                                    </div>
                                </Card>

                                <Card>
                                    <div style="padding:var(--spacing-md);display:grid;grid-template-columns:max-content 1fr;gap:var(--spacing-sm) var(--spacing-xl);align-items:baseline;">
                                        <span class="form__label">"Номенклатура 1С:"</span>
                                        <span style="display:flex;align-items:center;gap:var(--spacing-sm);flex-wrap:wrap;">
                                            <span class=nom_status.0>{nom_status.1}</span>
                                            {move || {
                                                let nom = nom_info.get();
                                                let nom_ref = nom_ref_clone.clone();
                                                if let Some(n) = nom {
                                                    let nom_id = n.id.clone();
                                                    let nom_title = format!("{} ({})", n.description.clone(), n.article.clone());
                                                    let nom_title_open = nom_title.clone();
                                                    let tabs_store = tabs_store_nom.clone();
                                                    view! {
                                                        <a
                                                            href="#"
                                                            style="color:var(--color-primary);text-decoration:none;font-weight:500;"
                                                            on:click=move |e| {
                                                                e.prevent_default();
                                                                tabs_store.open_tab(
                                                                    &format!("a004_nomenclature_detail_{}", nom_id),
                                                                    &nom_title_open,
                                                                );
                                                            }
                                                        >
                                                            {nom_title}
                                                        </a>
                                                        {if !n.code.is_empty() {
                                                            view! {
                                                                <span style="font-family:monospace;font-size:var(--font-size-xs);color:var(--color-text-tertiary);">
                                                                    {format!("(код: {})", n.code)}
                                                                </span>
                                                            }.into_any()
                                                        } else {
                                                            view! { <span></span> }.into_any()
                                                        }}
                                                    }.into_any()
                                                } else if let Some(ref_id) = nom_ref {
                                                    view! {
                                                        <span style="font-family:monospace;font-size:var(--font-size-xs);color:var(--color-text-tertiary);">
                                                            {ref_id}
                                                        </span>
                                                    }.into_any()
                                                } else {
                                                    view! { <span></span> }.into_any()
                                                }
                                            }}
                                        </span>

                                        <span class="form__label">"Подключение 1С:"</span>
                                        <span style="font-family:monospace;font-size:var(--font-size-sm);">
                                            {d.connection_id.clone()}
                                        </span>

                                        <span class="form__label">"Загружено:"</span>
                                        <span>{format_datetime(&d.fetched_at.to_rfc3339())}</span>
                                    </div>
                                </Card>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div>"Нет данных"</div> }.into_any()
                    }
                }}
            </div>
        </PageFrame>
    }
}
