use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use crate::shared::list_utils::format_number;
use crate::shared::page_frame::PageFrame;
use contracts::projections::p910_mp_unlinked_turnovers::dto::MpUnlinkedTurnoverDto;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

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
fn KvFieldWide(
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
        <div class="proj-detail__kv proj-detail__kv--wide">
            <div class="proj-detail__label">{label}</div>
            <div class=class>{value}</div>
        </div>
    }
}

fn open_registrator_tab(tabs: &AppGlobalContext, registrator_ref: &str) {
    let _ = (tabs, registrator_ref);
}

fn open_general_ledger_tab(tabs: &AppGlobalContext, general_ledger_ref: &str) {
    tabs.open_tab(
        &format!("general_ledger_details_{}", general_ledger_ref),
        &format!(
            "General Ledger {}",
            &general_ledger_ref[..general_ledger_ref.len().min(8)]
        ),
    );
}

#[component]
pub fn MpUnlinkedTurnoverDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let tabs_store = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let (data, set_data) = signal::<Option<MpUnlinkedTurnoverDto>>(None);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);

    let id_clone = id.clone();
    Effect::new(move |_| {
        let current_id = id_clone.clone();
        spawn_local(async move {
            match fetch_detail(&current_id).await {
                Ok(detail) => {
                    set_data.set(Some(detail));
                    set_loading.set(false);
                }
                Err(err) => {
                    log!("Failed to fetch p910 detail: {}", err);
                    set_error.set(Some(err));
                    set_loading.set(false);
                }
            }
        });
    });

    view! {
        <PageFrame page_id="p910_mp_unlinked_turnovers--detail" category="detail" class="page--wide">
            <div class="page__header">
                <div class="page__header-left">
                    {icon("database")}
                    <h1 class="page__title">"P910 Unlinked Turnover"</h1>
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

                    let Some(item) = data.get() else {
                        return view! { <div class="text-muted">"No data"</div> }.into_any();
                    };

                    let registrator_ref = item.registrator_ref.clone();
                    let general_ledger_ref = item.general_ledger_ref.clone().unwrap_or_default();
                    let has_general_ledger_ref = !general_ledger_ref.is_empty();
                    let tabs_for_registrator = tabs_store.clone();
                    let tabs_for_general_ledger = tabs_store.clone();

                    let general_ledger_source = if has_general_ledger_ref {
                        let general_ledger_ref_for_click = general_ledger_ref.clone();
                        let general_ledger_ref_label = general_ledger_ref.clone();
                        view! {
                            <div class="proj-detail__source-item">
                                <div class="proj-detail__source-role">"General Ledger"</div>
                                <div class="proj-detail__source-title">"Accounting entry"</div>
                                <a
                                    href="#"
                                    class="table__link proj-detail__source-ref"
                                    on:click=move |ev: web_sys::MouseEvent| {
                                        ev.prevent_default();
                                        open_general_ledger_tab(&tabs_for_general_ledger, &general_ledger_ref_for_click);
                                    }
                                >
                                    {general_ledger_ref_label}
                                </a>
                            </div>
                        }
                        .into_any()
                    } else {
                        view! { <></> }.into_any()
                    };

                    view! {
                        <div class="proj-detail">
                            <section class="proj-detail__hero">
                                <div class="proj-detail__hero-top">
                                    <div class="proj-detail__hero-title">
                                        <div class="proj-detail__eyebrow">"Projection P910"</div>
                                        <h2 class="proj-detail__title">{item.turnover_name.clone()}</h2>
                                        <div class="proj-detail__subtitle">{item.turnover_description.clone()}</div>
                                    </div>
                                </div>

                                <div class="proj-detail__hero-meta">
                                    <div class="proj-detail__chip">
                                        <span class="proj-detail__chip-label">"Date"</span>
                                        <span>{item.entry_date.clone()}</span>
                                    </div>
                                    <div class="proj-detail__chip">
                                        <span class="proj-detail__chip-label">"Layer"</span>
                                        <span>{item.layer.as_str().to_string()}</span>
                                    </div>
                                    <div class="proj-detail__chip">
                                        <span class="proj-detail__chip-label">"Connection"</span>
                                        <span>{item.connection_mp_ref.clone()}</span>
                                    </div>
                                </div>

                                <div class="proj-detail__source-list">
                                    <div class="proj-detail__source-item">
                                        <div class="proj-detail__source-role">"Registrator"</div>
                                        <div class="proj-detail__source-title">{item.registrator_type.clone()}</div>
                                        <a
                                            href="#"
                                            class="table__link proj-detail__source-ref"
                                            on:click=move |ev: web_sys::MouseEvent| {
                                                ev.prevent_default();
                                                open_registrator_tab(&tabs_for_registrator, &registrator_ref);
                                            }
                                        >
                                            {item.registrator_ref.clone()}
                                        </a>
                                    </div>
                                    {general_ledger_source}
                                </div>
                            </section>

                            <div class="proj-detail__sections">
                                <section class="proj-detail__section">
                                    <h3 class="proj-detail__section-title">"General"</h3>
                                    <div class="proj-detail__kv-grid">
                                        <KvField label="Turnover Code" value=item.turnover_code.clone() mono=true />
                                        <KvField label="Connection" value=item.connection_mp_ref.clone() mono=true />
                                        <KvField label="Registrator Type" value=item.registrator_type.clone() />
                                        <KvField
                                            label="Nomenclature Ref"
                                            value=item.nomenclature_ref.clone().unwrap_or_else(|| "-".to_string())
                                            mono=true
                                        />
                                    </div>
                                </section>

                                <section class="proj-detail__section">
                                    <h3 class="proj-detail__section-title">"Amount"</h3>
                                    <div class="proj-detail__kv-grid">
                                        <KvField label="Amount" value=format_number(item.amount) />
                                        <KvField label="Value Kind" value=item.value_kind.as_str().to_string() />
                                        <KvField label="Agg Kind" value=item.agg_kind.as_str().to_string() />
                                    </div>
                                </section>

                                <section class="proj-detail__section">
                                    <h3 class="proj-detail__section-title">"Classifier"</h3>
                                    <div class="proj-detail__kv-grid">
                                        <KvField label="Selection Rule" value=item.selection_rule.as_str().to_string() />
                                        <KvField label="Report Group" value=item.report_group.as_str().to_string() />
                                        <KvFieldWide label="LLM Description" value=item.turnover_llm_description.clone() />
                                    </div>
                                </section>

                                <section class="proj-detail__section">
                                    <h3 class="proj-detail__section-title">"Technical"</h3>
                                    <div class="proj-detail__kv-grid">
                                        <KvField label="ID" value=item.id.clone() mono=true />
                                        <KvField label="Created At" value=item.created_at.clone() mono=true />
                                        <KvField label="Updated At" value=item.updated_at.clone() mono=true />
                                        <KvField
                                            label="General Ledger Ref"
                                            value=item.general_ledger_ref.clone().unwrap_or_else(|| "-".to_string())
                                            mono=true
                                        />
                                        <KvFieldWide label="Comment" value=item.comment.clone().unwrap_or_else(|| "-".to_string()) />
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

async fn fetch_detail(id: &str) -> Result<MpUnlinkedTurnoverDto, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("/api/p910/unlinked-turnovers/{}", urlencoding::encode(id));
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
