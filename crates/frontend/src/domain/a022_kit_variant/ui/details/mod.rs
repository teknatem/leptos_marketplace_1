use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::page_frame::PageFrame;
use contracts::domain::a022_kit_variant::aggregate::KitVariant;
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use thaw::*;

fn format_datetime(s: &str) -> String {
    if let Some((date, time)) = s.split_once('T') {
        let time_clean = time
            .split('Z')
            .next()
            .unwrap_or(time)
            .split('+')
            .next()
            .unwrap_or(time);
        let hms = time_clean.split('.').next().unwrap_or(time_clean);
        let date_formatted = {
            let parts: Vec<&str> = date.split('-').collect();
            if parts.len() == 3 {
                format!("{}.{}.{}", parts[2], parts[1], parts[0])
            } else {
                date.to_string()
            }
        };
        return format!("{} {}", date_formatted, hms);
    }
    s.to_string()
}

#[derive(Debug, Clone, Deserialize)]
struct NomInfo {
    pub id: String,
    pub code: String,
    pub description: String,
    pub article: String,
}

#[derive(Debug, Clone)]
struct ResolvedGoodsItem {
    pub nomenclature_ref: String,
    pub quantity: f64,
    pub nom_info: Option<NomInfo>,
}

#[component]
pub fn KitVariantDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let stored_id = StoredValue::new(id.clone());

    let (item, set_item) = signal(None::<KitVariant>);
    let (owner_info, set_owner_info) = signal(None::<NomInfo>);
    let (goods_items, set_goods_items) = signal(Vec::<ResolvedGoodsItem>::new());
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);

    Effect::new(move || {
        let id_val = stored_id.get_value();
        let tabs_store = tabs_store.clone();
        set_loading.set(true);
        set_error.set(None);

        spawn_local(async move {
            let url = format!("{}/api/a022/kit-variant/{}", api_base(), id_val);
            match Request::get(&url).send().await {
                Ok(response) if response.ok() => {
                    match response.json::<KitVariant>().await {
                        Ok(data) => {
                            let tab_key = format!("a022_kit_variant_detail_{}", id_val);
                            let tab_title = format!("Комплект {}", data.base.description);
                            tabs_store.update_tab_title(&tab_key, &tab_title);

                            // Загрузить владельца (номенклатуру)
                            if let Some(ref owner_id) = data.owner_ref {
                                let nom_url =
                                    format!("{}/api/nomenclature/{}", api_base(), owner_id);
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
                                            set_owner_info.set(Some(NomInfo {
                                                id: n.id,
                                                code: n.code,
                                                description: n.description,
                                                article: n.article,
                                            }));
                                        }
                                    }
                                }
                            }

                            // Загрузить номенклатуру для каждой позиции состава
                            let goods = data.parse_goods();
                            let mut resolved = Vec::new();
                            for g in &goods {
                                let nom_url = format!(
                                    "{}/api/nomenclature/{}",
                                    api_base(),
                                    g.nomenclature_ref
                                );
                                let nom_info = if let Ok(nom_resp) =
                                    Request::get(&nom_url).send().await
                                {
                                    if nom_resp.ok() {
                                        #[derive(Deserialize)]
                                        struct NomFull {
                                            pub id: String,
                                            pub code: String,
                                            pub description: String,
                                            pub article: String,
                                        }
                                        nom_resp.json::<NomFull>().await.ok().map(|n| NomInfo {
                                            id: n.id,
                                            code: n.code,
                                            description: n.description,
                                            article: n.article,
                                        })
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };
                                resolved.push(ResolvedGoodsItem {
                                    nomenclature_ref: g.nomenclature_ref.clone(),
                                    quantity: g.quantity,
                                    nom_info,
                                });
                            }
                            set_goods_items.set(resolved);
                            set_item.set(Some(data));
                        }
                        Err(e) => set_error.set(Some(format!("Ошибка парсинга: {}", e))),
                    }
                }
                Ok(r) => set_error.set(Some(format!("HTTP {}", r.status()))),
                Err(e) => set_error.set(Some(format!("Ошибка сети: {}", e))),
            }
            set_loading.set(false);
        });
    });

    view! {
        <PageFrame page_id="a022_kit_variant--detail" category="detail">
            {move || {
                let title = item.get()
                    .map(|d| d.base.description.clone())
                    .unwrap_or_else(|| "Вариант комплектации".to_string());
                view! {
                    <div class="page__header">
                        <div class="page__header-left">
                            <h1 class="page__title">{title}</h1>
                        </div>
                        <div class="page__header-right">
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
                    if let Some(d) = item.get() {
                        let tabs_store_owner = leptos::context::use_context::<AppGlobalContext>()
                            .expect("AppGlobalContext not found");
                        let owner_ref_clone = d.owner_ref.clone();

                        view! {
                            <div style="padding:var(--spacing-lg);display:flex;flex-direction:column;gap:var(--spacing-lg);">
                                // Основные реквизиты
                                <Card>
                                    <div style="padding:var(--spacing-md);display:grid;grid-template-columns:max-content 1fr;gap:var(--spacing-sm) var(--spacing-xl);align-items:baseline;">
                                        <span class="form__label">"Код:"</span>
                                        <code style="font-family:monospace;">{d.base.code.clone()}</code>

                                        <span class="form__label">"Наименование:"</span>
                                        <strong>{d.base.description.clone()}</strong>

                                        <span class="form__label">"Номенклатура (владелец):"</span>
                                        <span style="display:flex;align-items:center;gap:var(--spacing-sm);flex-wrap:wrap;">
                                            {move || {
                                                let nom = owner_info.get();
                                                let owner_ref = owner_ref_clone.clone();
                                                if let Some(n) = nom {
                                                    let nom_id = n.id.clone();
                                                    let nom_title = format!("{} ({})", n.description, n.article);
                                                    let nom_title_open = nom_title.clone();
                                                    let tabs_store = tabs_store_owner.clone();
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
                                                    }.into_any()
                                                } else if let Some(ref_id) = owner_ref {
                                                    view! {
                                                        <span style="font-family:monospace;font-size:var(--font-size-xs);color:var(--color-text-tertiary);">
                                                            {ref_id}
                                                        </span>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <span class="badge badge--warning">"Не указана"</span>
                                                    }.into_any()
                                                }
                                            }}
                                        </span>

                                        <span class="form__label">"Загружено:"</span>
                                        <span>{format_datetime(&d.fetched_at.to_rfc3339())}</span>

                                        <span class="form__label">"Подключение 1С:"</span>
                                        <span style="font-family:monospace;font-size:var(--font-size-sm);">
                                            {d.connection_id.clone()}
                                        </span>
                                    </div>
                                </Card>

                                // Таблица состава набора
                                <Card>
                                    <div style="padding:var(--spacing-md);">
                                        <h3 style="margin:0 0 var(--spacing-md);font-size:var(--font-size-md);font-weight:600;">
                                            "Состав набора"
                                            {move || {
                                                let cnt = goods_items.get().len();
                                                if cnt > 0 {
                                                    view! {
                                                        <span class="badge badge--primary" style="margin-left:var(--spacing-sm);">
                                                            {format!("{} поз.", cnt)}
                                                        </span>
                                                    }.into_any()
                                                } else {
                                                    view! { <></> }.into_any()
                                                }
                                            }}
                                        </h3>

                                        {move || {
                                            let items = goods_items.get();
                                            if items.is_empty() {
                                                return view! {
                                                    <div style="color:var(--color-text-secondary);padding:var(--spacing-md) 0;">
                                                        "Состав не загружен или пуст"
                                                    </div>
                                                }.into_any();
                                            }

                                            let tabs_store_goods = leptos::context::use_context::<AppGlobalContext>()
                                                .expect("AppGlobalContext not found");

                                            view! {
                                                <table style="width:100%;border-collapse:collapse;">
                                                    <thead>
                                                        <tr style="border-bottom:2px solid var(--color-border);">
                                                            <th style="text-align:left;padding:var(--spacing-sm) var(--spacing-md);font-weight:600;color:var(--color-text-secondary);font-size:var(--font-size-xs);width:40px;">
                                                                "№"
                                                            </th>
                                                            <th style="text-align:left;padding:var(--spacing-sm) var(--spacing-md);font-weight:600;color:var(--color-text-secondary);font-size:var(--font-size-xs);">
                                                                "Номенклатура"
                                                            </th>
                                                            <th style="text-align:left;padding:var(--spacing-sm) var(--spacing-md);font-weight:600;color:var(--color-text-secondary);font-size:var(--font-size-xs);width:120px;">
                                                                "Артикул"
                                                            </th>
                                                            <th style="text-align:right;padding:var(--spacing-sm) var(--spacing-md);font-weight:600;color:var(--color-text-secondary);font-size:var(--font-size-xs);width:100px;">
                                                                "Количество"
                                                            </th>
                                                        </tr>
                                                    </thead>
                                                    <tbody>
                                                        {items.into_iter().enumerate().map(|(idx, g)| {
                                                            let tabs_store = tabs_store_goods.clone();
                                                            let nom_ref = g.nomenclature_ref.clone();

                                                            let (nom_id_opt, nom_name, nom_article) = if let Some(ref n) = g.nom_info {
                                                                (Some(n.id.clone()), n.description.clone(), n.article.clone())
                                                            } else {
                                                                (None, nom_ref.clone(), String::new())
                                                            };

                                                            let qty_str = if g.quantity == g.quantity.floor() {
                                                                format!("{}", g.quantity as i64)
                                                            } else {
                                                                format!("{:.3}", g.quantity)
                                                            };

                                                            let row_bg = if idx % 2 == 0 {
                                                                "background:var(--color-bg-primary);"
                                                            } else {
                                                                "background:var(--color-bg-secondary);"
                                                            };

                                                            view! {
                                                                <tr style=row_bg>
                                                                    <td style="padding:var(--spacing-sm) var(--spacing-md);color:var(--color-text-tertiary);font-size:var(--font-size-xs);text-align:left;">
                                                                        {idx + 1}
                                                                    </td>
                                                                    <td style="padding:var(--spacing-sm) var(--spacing-md);">
                                                                        {if let Some(n_id) = nom_id_opt {
                                                                            let title_open = nom_name.clone();
                                                                            view! {
                                                                                <a
                                                                                    href="#"
                                                                                    style="color:var(--color-primary);text-decoration:none;"
                                                                                    on:click=move |e| {
                                                                                        e.prevent_default();
                                                                                        tabs_store.open_tab(
                                                                                            &format!("a004_nomenclature_detail_{}", n_id),
                                                                                            &title_open,
                                                                                        );
                                                                                    }
                                                                                >
                                                                                    {nom_name}
                                                                                </a>
                                                                            }.into_any()
                                                                        } else {
                                                                            view! {
                                                                                <span style="font-family:monospace;font-size:var(--font-size-xs);color:var(--color-text-tertiary);">
                                                                                    {nom_name}
                                                                                </span>
                                                                            }.into_any()
                                                                        }}
                                                                    </td>
                                                                    <td style="padding:var(--spacing-sm) var(--spacing-md);">
                                                                        <span style="font-family:monospace;font-size:var(--font-size-xs);color:var(--color-text-secondary);">
                                                                            {if nom_article.is_empty() { "—".to_string() } else { nom_article }}
                                                                        </span>
                                                                    </td>
                                                                    <td style="padding:var(--spacing-sm) var(--spacing-md);text-align:right;font-variant-numeric:tabular-nums;font-weight:500;">
                                                                        {qty_str}
                                                                    </td>
                                                                </tr>
                                                            }
                                                        }).collect::<Vec<_>>()}
                                                    </tbody>
                                                </table>
                                            }.into_any()
                                        }}
                                    </div>
                                </Card>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div style="padding:var(--spacing-lg);">"Нет данных"</div> }.into_any()
                    }
                }}
            </div>
        </PageFrame>
    }
}
