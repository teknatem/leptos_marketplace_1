use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::page_frame::PageFrame;
use contracts::domain::a023_purchase_of_goods::aggregate::{PurchaseOfGoods, PurchaseOfGoodsLine};
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

#[component]
pub fn PurchaseOfGoodsDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let stored_id = StoredValue::new(id.clone());

    let (doc, set_doc) = signal(None::<PurchaseOfGoods>);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);

    let load_doc = {
        let stored_id = stored_id;
        let tabs_store = tabs_store.clone();
        move || {
            let id_val = stored_id.get_value();
            set_loading.set(true);
            set_error.set(None);
            spawn_local(async move {
                let url = format!("{}/api/a023/purchase-of-goods/{}", api_base(), id_val);
                match Request::get(&url).send().await {
                    Ok(response) if response.ok() => {
                        match response.json::<PurchaseOfGoods>().await {
                            Ok(data) => {
                                let tab_key =
                                    format!("a023_purchase_of_goods_detail_{}", id_val);
                                let tab_title =
                                    format!("Приобр. {}", data.document_no);
                                tabs_store.update_tab_title(&tab_key, &tab_title);
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

    view! {
        <PageFrame page_id="a023_purchase_of_goods--detail" category="detail">
            <div class="page__header">
                <div class="page__header-left">
                    {move || {
                        let title = doc.get()
                            .map(|d| format!("Приобр. {} от {}", d.document_no, format_date(&d.document_date)))
                            .unwrap_or_else(|| "Приобретение товаров и услуг".to_string());
                        view! { <h1 class="page__title">{title}</h1> }
                    }}
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
                        let lines: Vec<PurchaseOfGoodsLine> = d.parse_lines();
                        let lines_count = lines.len();

                        let total_amount: f64 = lines.iter().map(|l| l.amount_with_vat).sum();
                        let total_vat: f64 = lines.iter().map(|l| l.vat_amount).sum();

                        view! {
                            <div style="padding:var(--spacing-lg);display:flex;flex-direction:column;gap:var(--spacing-lg);">
                                <Card>
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
                                </Card>

                                {if !lines.is_empty() {
                                    view! {
                                        <Card>
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
                                                                key=|line| line.nomenclature_key.clone()
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
                                        </Card>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div style="padding:var(--spacing-md);color:var(--color-text-secondary);">
                                            "Строки табличной части отсутствуют"
                                        </div>
                                    }.into_any()
                                }}
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
