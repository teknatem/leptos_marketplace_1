use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use crate::shared::list_utils::format_number;
use crate::shared::page_frame::PageFrame;
use contracts::projections::p911_wb_advert_by_items::dto::WbAdvertByItemDetailDto;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_string()
    } else {
        let short: String = value.chars().take(max_chars).collect();
        format!("{short}...")
    }
}

#[component]
fn KvField(
    label: &'static str,
    value: String,
    #[prop(default = false)] mono: bool,
) -> impl IntoView {
    let class = if mono {
        "proj-detail__value proj-detail__value--mono"
    } else {
        "proj-detail__value"
    };

    view! {
        <div class="proj-detail__kv">
            <div class="proj-detail__label">{label}</div>
            <div class=class>{value}</div>
        </div>
    }
}

#[component]
pub fn WbAdvertByItemDetail(
    general_ledger_ref: String,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let tabs_store = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let (data, set_data) = signal::<Option<WbAdvertByItemDetailDto>>(None);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);

    let general_ledger_ref_clone = general_ledger_ref.clone();
    Effect::new(move |_| {
        let current_id = general_ledger_ref_clone.clone();
        spawn_local(async move {
            match fetch_detail(&current_id).await {
                Ok(detail) => {
                    set_data.set(Some(detail));
                    set_loading.set(false);
                }
                Err(err) => {
                    log!("Failed to fetch p911 detail: {}", err);
                    set_error.set(Some(err));
                    set_loading.set(false);
                }
            }
        });
    });

    view! {
        <PageFrame page_id="p911_wb_advert_by_items--detail" category="detail" class="page--wide">
            <div class="page__header">
                <div class="page__header-left">
                    {icon("database")}
                    <h1 class="page__title">"P911 WB Advert By Items"</h1>
                </div>
                <div class="page__header-right">
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| on_close.run(())>
                        "Close"
                    </Button>
                </div>
            </div>

            <div class="page__content">
                {move || {
                    if loading.get() {
                        return view! { <div class="text-muted">"Loading..."</div> }.into_any();
                    }
                    if let Some(err) = error.get() {
                        return view! {
                            <div class="warning-box warning-box--error">
                                <span class="warning-box__icon">"!"</span>
                                <span class="warning-box__text">{err}</span>
                            </div>
                        }.into_any();
                    }

                    let Some(detail) = data.get() else {
                        return view! { <div class="text-muted">"No data"</div> }.into_any();
                    };
                    let problem_count = detail.items.iter().filter(|item| item.is_problem).count();

                    let summary = detail.items.first().cloned().map(|item| {
                        view! {
                            <section class="proj-detail__section">
                                <h3 class="proj-detail__section-title">"Summary"</h3>
                                <div class="proj-detail__kv-grid">
                                    <KvField label="Turnover Code" value=item.turnover_code.clone() mono=true />
                                    <KvField label="Layer" value=item.layer.as_str().to_string() />
                                    <KvField label="Connection" value=item.connection_mp_ref.clone() mono=true />
                                    <KvField label="Rows" value=detail.items.len().to_string() />
                                    <KvField label="Problem Rows" value=problem_count.to_string() />
                                    <KvField label="Total Amount" value=format_number(detail.total_amount) />
                                </div>
                            </section>
                        }.into_any()
                    }).unwrap_or_else(|| view! { <></> }.into_any());

                    let general_ledger_block = detail.general_ledger_entry.clone().map(|item| {
                        view! {
                            <section class="proj-detail__section">
                                <h3 class="proj-detail__section-title">"General Ledger Entry"</h3>
                                <div class="proj-detail__kv-grid">
                                    <KvField label="ID" value=item.id.clone() mono=true />
                                    <KvField label="Entry Date" value=item.entry_date.clone() />
                                    <KvField label="Debit" value=item.debit_account.clone() />
                                    <KvField label="Credit" value=item.credit_account.clone() />
                                    <KvField label="Amount" value=format_number(item.amount) />
                                    <KvField label="Turnover" value=item.turnover_code.clone() mono=true />
                                </div>
                            </section>
                        }.into_any()
                    }).unwrap_or_else(|| view! { <></> }.into_any());

                    let first_registrator = detail
                        .items
                        .first()
                        .map(|item| item.registrator_ref.clone())
                        .unwrap_or_default();
                    let current_general_ledger_ref = detail.general_ledger_ref.clone();
                    let tabs_for_general_ledger = tabs_store.clone();
                    let tabs_for_registrator = tabs_store.clone();

                    view! {
                        <div class="proj-detail">
                            <section class="proj-detail__hero">
                                <div class="proj-detail__hero-top">
                                    <div class="proj-detail__hero-title">
                                        <div class="proj-detail__eyebrow">"Projection P911"</div>
                                        <h2 class="proj-detail__title">"WB Advert By Items Turnovers"</h2>
                                        <div class="proj-detail__subtitle">
                                            {format!("General ledger {}", truncate(&detail.general_ledger_ref, 16))}
                                        </div>
                                    </div>
                                </div>

                                <div style="display:flex;gap:12px;flex-wrap:wrap;">
                                    <Button
                                        appearance=ButtonAppearance::Secondary
                                        on_click=move |_| {
                                            tabs_for_general_ledger.open_tab(
                                                &format!("general_ledger_details_{}", current_general_ledger_ref),
                                                &format!("General Ledger {}", truncate(&current_general_ledger_ref, 8)),
                                            );
                                        }
                                    >
                                        "Open general ledger"
                                    </Button>
                                    <Button
                                        appearance=ButtonAppearance::Secondary
                                        on_click=move |_| {
                                            if let Some(rest) = first_registrator.strip_prefix("a026:") {
                                                tabs_for_registrator.open_tab(
                                                    &format!("a026_wb_advert_daily_details_{}", rest),
                                                    &format!("WB Ads {}", truncate(rest, 8)),
                                                );
                                            }
                                        }
                                    >
                                        "Open document"
                                    </Button>
                                </div>
                            </section>

                            <div class="proj-detail__sections">
                                {summary}
                                {general_ledger_block}
                                <section class="proj-detail__section">
                                    <h3 class="proj-detail__section-title">"Rows"</h3>
                                    <div class="table-wrapper" style="width:100%;overflow-x:auto;">
                                        <Table attr:style="width:100%;">
                                            <TableHeader>
                                                <TableRow>
                                                    <TableHeaderCell min_width=120.0>"Entry Date"</TableHeaderCell>
                                                    <TableHeaderCell min_width=220.0>"Nomenclature"</TableHeaderCell>
                                                    <TableHeaderCell min_width=120.0>"Amount"</TableHeaderCell>
                                                    <TableHeaderCell min_width=120.0>"Problem"</TableHeaderCell>
                                                    <TableHeaderCell min_width=260.0>"Registrator"</TableHeaderCell>
                                                </TableRow>
                                            </TableHeader>
                                            <TableBody>
                                                <For each=move || detail.items.clone() key=|item| item.id.clone() children=move |item| {
                                                    view! {
                                                        <TableRow>
                                                            <TableCell><TableCellLayout>{item.entry_date}</TableCellLayout></TableCell>
                                                            <TableCell><TableCellLayout truncate=true>{item.nomenclature_ref.unwrap_or_else(|| "-".to_string())}</TableCellLayout></TableCell>
                                                            <TableCell class="table__cell--right"><TableCellLayout>{format_number(item.amount)}</TableCellLayout></TableCell>
                                                            <TableCell>
                                                                <TableCellLayout>
                                                                    {if item.is_problem {
                                                                        view! {
                                                                            <Badge appearance=BadgeAppearance::Filled color=BadgeColor::Danger>
                                                                                <span>"Problem"</span>
                                                                            </Badge>
                                                                        }.into_any()
                                                                    } else {
                                                                        view! { <span class="text-muted">"—"</span> }.into_any()
                                                                    }}
                                                                </TableCellLayout>
                                                            </TableCell>
                                                            <TableCell><TableCellLayout truncate=true>{item.registrator_ref}</TableCellLayout></TableCell>
                                                        </TableRow>
                                                    }
                                                }/>
                                            </TableBody>
                                        </Table>
                                    </div>
                                </section>
                            </div>
                        </div>
                    }.into_any()
                }}
            </div>
        </PageFrame>
    }
}

async fn fetch_detail(general_ledger_ref: &str) -> Result<WbAdvertByItemDetailDto, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!(
        "/api/p911/wb-advert-by-items/{}",
        urlencoding::encode(general_ledger_ref)
    );
    let request =
        Request::new_with_str_and_init(&url, &opts).map_err(|error| format!("{error:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|error| format!("{error:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let response_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|error| format!("{error:?}"))?;
    let response: Response = response_value
        .dyn_into()
        .map_err(|error| format!("{error:?}"))?;
    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }
    let text = JsFuture::from(response.text().map_err(|error| format!("{error:?}"))?)
        .await
        .map_err(|error| format!("{error:?}"))?;
    let text = text.as_string().ok_or_else(|| "bad text".to_string())?;
    serde_json::from_str(&text).map_err(|error| format!("{error}"))
}
