use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use contracts::domain::a023_purchase_of_goods::aggregate::{PurchaseOfGoods, PurchaseOfGoodsLine};
use contracts::projections::p912_nomenclature_costs::dto::{
    NomenclatureCostDto, NomenclatureCostListResponse,
};
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

fn format_date(s: &str) -> String {
    let date_part = s.split('T').next().unwrap_or(s);
    if let Some((year, rest)) = date_part.split_once('-') {
        if let Some((month, day)) = rest.split_once('-') {
            return format!("{}.{}.{}", day, month, year);
        }
    }
    s.to_string()
}

fn format_money(v: f64) -> String {
    format!("{:.2}", v)
}

fn format_optional_money(v: Option<f64>) -> String {
    v.map(format_money).unwrap_or_else(|| "—".to_string())
}

#[component]
pub fn PurchaseOfGoodsDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let stored_id = StoredValue::new(id.clone());

    let (doc, set_doc) = signal(None::<PurchaseOfGoods>);
    let (projection_rows, set_projection_rows) = signal(Vec::<NomenclatureCostDto>::new());
    let (projection_error, set_projection_error) = signal(None::<String>);
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
            set_projection_rows.set(Vec::new());
            set_projection_error.set(None);
            spawn_local(async move {
                let url = format!("{}/api/a023/purchase-of-goods/{}", api_base(), id_val);
                match Request::get(&url).send().await {
                    Ok(response) if response.ok() => match response.json::<PurchaseOfGoods>().await
                    {
                        Ok(data) => {
                            let tab_key = format!("a023_purchase_of_goods_details_{}", id_val);
                            let tab_title = format!("Приобр. {}", data.document_no);
                            tabs_store.update_tab_title(&tab_key, &tab_title);

                            let projection_url = format!(
                                "{}/api/p912/nomenclature-costs?registrator_type=a023_purchase_of_goods&registrator_ref={}&limit=500",
                                api_base(),
                                id_val
                            );
                            match Request::get(&projection_url).send().await {
                                Ok(resp) if resp.ok() => {
                                    match resp.json::<NomenclatureCostListResponse>().await {
                                        Ok(payload) => set_projection_rows.set(payload.items),
                                        Err(e) => set_projection_error
                                            .set(Some(format!("Ошибка загрузки p912: {}", e))),
                                    }
                                }
                                Ok(resp) => set_projection_error.set(Some(format!(
                                    "Ошибка загрузки p912: HTTP {}",
                                    resp.status()
                                ))),
                                Err(e) => set_projection_error
                                    .set(Some(format!("Ошибка сети p912: {}", e))),
                            }

                            set_doc.set(Some(data));
                        }
                        Err(e) => set_error.set(Some(format!("Ошибка парсинга: {}", e))),
                    },
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
                let url = format!("{}/api/a023/purchase-of-goods/{}/post", api_base(), id_val);
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
                let url = format!(
                    "{}/api/a023/purchase-of-goods/{}/unpost",
                    api_base(),
                    id_val
                );
                match Request::post(&url).send().await {
                    Ok(r) if r.ok() => load_doc(),
                    Ok(r) => set_error.set(Some(format!(
                        "Ошибка отмены проведения: HTTP {}",
                        r.status()
                    ))),
                    Err(e) => set_error.set(Some(format!("Ошибка сети: {}", e))),
                }
                set_posting.set(false);
            });
        }
    };

    view! {
        <PageFrame page_id="a023_purchase_of_goods--detail" category="detail">
            {move || {
                let title = doc
                    .get()
                    .map(|d| format!("Приобр. {} от {}", d.document_no, format_date(&d.document_date)))
                    .unwrap_or_else(|| "Приобретение товаров и услуг".to_string());
                let is_posted = doc.get().map(|d| d.base.metadata.is_posted).unwrap_or(false);
                let doc_loaded = doc.get().is_some();
                let post_doc = post_doc.clone();
                let unpost_doc = unpost_doc.clone();
                view! {
                    <div class="page__header">
                        <div class="page__header-left">
                            <h1 class="page__title">{title}</h1>
                            {if doc_loaded {
                                if is_posted {
                                    view! { <span class="badge badge--success">"Проведён"</span> }.into_any()
                                } else {
                                    view! { <span class="badge badge--secondary">"Не проведён"</span> }.into_any()
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
                                    }
                                    .into_any()
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
                                    }
                                    .into_any()
                                }
                            } else {
                                view! { <span></span> }.into_any()
                            }}
                            <Button appearance=ButtonAppearance::Subtle on_click=move |_| on_close.run(())>
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
                        }
                        .into_any();
                    }
                    if let Some(err) = error.get() {
                        return view! {
                            <div style="padding:var(--spacing-lg);background:var(--color-error-50);border:1px solid var(--color-error-100);border-radius:var(--radius-sm);color:var(--color-error);margin:var(--spacing-lg);">
                                <strong>"Ошибка: "</strong>{err}
                            </div>
                        }
                        .into_any();
                    }
                    if let Some(d) = doc.get() {
                        let lines: Vec<PurchaseOfGoodsLine> = d.parse_lines();
                        let lines_count = lines.len();
                        let total_amount: f64 = lines.iter().map(|l| l.amount_with_vat).sum();
                        let total_vat: f64 = lines.iter().map(|l| l.vat_amount).sum();

                        view! {
                            <div class="detail-grid">
                                <div class="detail-grid__col">
                                    <CardAnimated delay_ms=0 nav_id="a023_purchase_of_goods_details_summary">
                                        <div style="padding:var(--spacing-md);display:grid;grid-template-columns:max-content 1fr;gap:var(--spacing-sm) var(--spacing-xl);align-items:baseline;">
                                            <span class="form__label">"Номер документа:"</span>
                                            <strong style="font-size:var(--font-size-lg);">{d.document_no.clone()}</strong>

                                            <span class="form__label">"Дата документа:"</span>
                                            <span>{format_date(&d.document_date)}</span>

                                            <span class="form__label">"Контрагент (UUID):"</span>
                                            <code style="font-family:monospace;font-size:var(--font-size-sm);">
                                                {d.counterparty_key.clone()}
                                            </code>

                                            <span class="form__label">"Строк товаров:"</span>
                                            <span>{lines_count}</span>

                                            <span class="form__label">"Итого с НДС:"</span>
                                            <strong>{format_money(total_amount)}</strong>

                                            <span class="form__label">"Итого НДС:"</span>
                                            <span>{format_money(total_vat)}</span>
                                        </div>
                                    </CardAnimated>
                                </div>

                                <div class="detail-grid__col">
                                    <CardAnimated delay_ms=40 nav_id="a023_purchase_of_goods_details_lines">
                                        {if !lines.is_empty() {
                                            view! {
                                                <div style="padding:var(--spacing-md);">
                                                    <h3 style="margin:0 0 var(--spacing-md) 0;font-size:var(--font-size-md);">"Табличная часть «Товары»"</h3>
                                                    <div class="table-wrapper">
                                                        <Table attr:style="width:100%;">
                                                            <TableHeader>
                                                                <TableRow>
                                                                    <TableHeaderCell>"Номенклатура (UUID)"</TableHeaderCell>
                                                                    <TableHeaderCell>"Кол-во"</TableHeaderCell>
                                                                    <TableHeaderCell>"Цена"</TableHeaderCell>
                                                                    <TableHeaderCell>"Сумма с НДС"</TableHeaderCell>
                                                                    <TableHeaderCell>"НДС"</TableHeaderCell>
                                                                </TableRow>
                                                            </TableHeader>
                                                            <TableBody>
                                                                <For
                                                                    each=move || lines.clone()
                                                                    key=|line| format!(
                                                                        "{}:{}:{}",
                                                                        line.nomenclature_key, line.quantity, line.price
                                                                    )
                                                                    children=move |line| {
                                                                        view! {
                                                                            <TableRow>
                                                                                <TableCell>
                                                                                    <TableCellLayout truncate=true>
                                                                                        <code style="font-family:monospace;font-size:var(--font-size-xs);">
                                                                                            {line.nomenclature_key.clone()}
                                                                                        </code>
                                                                                    </TableCellLayout>
                                                                                </TableCell>
                                                                                <TableCell>
                                                                                    <TableCellLayout>
                                                                                        <span style="font-variant-numeric:tabular-nums;">
                                                                                            {format!("{:.3}", line.quantity)}
                                                                                        </span>
                                                                                    </TableCellLayout>
                                                                                </TableCell>
                                                                                <TableCell>
                                                                                    <TableCellLayout>
                                                                                        <span style="font-variant-numeric:tabular-nums;">
                                                                                            {format_money(line.price)}
                                                                                        </span>
                                                                                    </TableCellLayout>
                                                                                </TableCell>
                                                                                <TableCell>
                                                                                    <TableCellLayout>
                                                                                        <strong style="font-variant-numeric:tabular-nums;">
                                                                                            {format_money(line.amount_with_vat)}
                                                                                        </strong>
                                                                                    </TableCellLayout>
                                                                                </TableCell>
                                                                                <TableCell>
                                                                                    <TableCellLayout>
                                                                                        <span style="font-variant-numeric:tabular-nums;color:var(--color-text-secondary);">
                                                                                            {format_money(line.vat_amount)}
                                                                                        </span>
                                                                                    </TableCellLayout>
                                                                                </TableCell>
                                                                            </TableRow>
                                                                        }
                                                                    }
                                                                />
                                                            </TableBody>
                                                        </Table>
                                                    </div>
                                                </div>
                                            }
                                            .into_any()
                                        } else {
                                            view! {
                                                <div style="padding:var(--spacing-md);color:var(--color-text-secondary);">
                                                    "Строки табличной части отсутствуют"
                                                </div>
                                            }
                                            .into_any()
                                        }}
                                    </CardAnimated>
                                </div>

                                <div class="detail-grid__col">
                                    <CardAnimated delay_ms=80 nav_id="a023_purchase_of_goods_details_projection">
                                        <div style="padding:var(--spacing-md);display:flex;flex-direction:column;gap:var(--spacing-md);">
                                            <div style="display:flex;justify-content:space-between;align-items:center;gap:var(--spacing-md);flex-wrap:wrap;">
                                                <h3 style="margin:0;font-size:var(--font-size-md);">"Проекция себестоимости p912"</h3>
                                                <span class="badge badge--secondary">
                                                    {format!("Строк: {}", projection_rows.get().len())}
                                                </span>
                                            </div>

                                            {move || {
                                                if let Some(err) = projection_error.get() {
                                                    view! {
                                                        <div style="padding:var(--spacing-sm);background:var(--color-error-50);border:1px solid var(--color-error-100);border-radius:var(--radius-sm);color:var(--color-error);">
                                                            {err}
                                                        </div>
                                                    }
                                                    .into_any()
                                                } else if projection_rows.get().is_empty() {
                                                    view! {
                                                        <div style="color:var(--color-text-secondary);">
                                                            "Для документа строки p912 пока не сформированы"
                                                        </div>
                                                    }
                                                    .into_any()
                                                } else {
                                                    view! {
                                                        <div class="table-wrapper">
                                                            <Table attr:style="width:100%;">
                                                                <TableHeader>
                                                                    <TableRow>
                                                                        <TableHeaderCell>"Номенклатура"</TableHeaderCell>
                                                                        <TableHeaderCell>"Период"</TableHeaderCell>
                                                                        <TableHeaderCell>"С/с"</TableHeaderCell>
                                                                        <TableHeaderCell>"Кол-во"</TableHeaderCell>
                                                                        <TableHeaderCell>"Сумма"</TableHeaderCell>
                                                                    </TableRow>
                                                                </TableHeader>
                                                                <TableBody>
                                                                    <For
                                                                        each=move || projection_rows.get()
                                                                        key=|row| row.id.clone()
                                                                        children=move |row| {
                                                                            let nomenclature_label = row
                                                                                .nomenclature_name
                                                                                .clone()
                                                                                .unwrap_or_else(|| row.nomenclature_ref.clone());
                                                                            let article = row.nomenclature_article.clone().unwrap_or_default();
                                                                            view! {
                                                                                <TableRow>
                                                                                    <TableCell>
                                                                                        <TableCellLayout truncate=true>
                                                                                            <div style="display:flex;flex-direction:column;gap:2px;">
                                                                                                <span>{nomenclature_label}</span>
                                                                                                <span style="font-size:var(--font-size-xs);color:var(--color-text-secondary);font-family:monospace;">
                                                                                                    {if article.is_empty() {
                                                                                                        row.nomenclature_ref.clone()
                                                                                                    } else {
                                                                                                        format!("{} | {}", article, row.nomenclature_ref.clone())
                                                                                                    }}
                                                                                                </span>
                                                                                            </div>
                                                                                        </TableCellLayout>
                                                                                    </TableCell>
                                                                                    <TableCell>
                                                                                        <TableCellLayout>{format_date(&row.period)}</TableCellLayout>
                                                                                    </TableCell>
                                                                                    <TableCell>
                                                                                        <TableCellLayout>{format_money(row.cost)}</TableCellLayout>
                                                                                    </TableCell>
                                                                                    <TableCell>
                                                                                        <TableCellLayout>{format_optional_money(row.quantity)}</TableCellLayout>
                                                                                    </TableCell>
                                                                                    <TableCell>
                                                                                        <TableCellLayout>{format_optional_money(row.amount)}</TableCellLayout>
                                                                                    </TableCell>
                                                                                </TableRow>
                                                                            }
                                                                        }
                                                                    />
                                                                </TableBody>
                                                            </Table>
                                                        </div>
                                                    }
                                                    .into_any()
                                                }
                                            }}
                                        </div>
                                    </CardAnimated>
                                </div>
                            </div>
                        }
                        .into_any()
                    } else {
                        view! { <div>"Нет данных"</div> }.into_any()
                    }
                }}
            </div>
        </PageFrame>
    }
}
