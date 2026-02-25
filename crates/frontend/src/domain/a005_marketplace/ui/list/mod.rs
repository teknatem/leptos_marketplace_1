use crate::domain::a005_marketplace::ui::details::MarketplaceDetails;
use crate::shared::api_utils::api_base;
use crate::shared::components::table::TableCrosshairHighlight;
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, sort_list, Sortable};
use crate::shared::modal_stack::ModalStackService;
use crate::shared::table_utils::init_column_resize;
use contracts::domain::a005_marketplace::aggregate::Marketplace;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cmp::Ordering;
use thaw::*;

#[derive(Clone, Debug)]
pub struct MarketplaceRow {
    pub id: String,
    pub code: String,
    pub description: String,
    pub url: String,
    pub logo_path: Option<String>,
    pub comment: String,
    pub created_at: String,
}

impl From<Marketplace> for MarketplaceRow {
    fn from(m: Marketplace) -> Self {
        use contracts::domain::common::AggregateId;

        Self {
            id: m.base.id.as_string(),
            code: m.base.code,
            description: m.base.description,
            url: m.url,
            logo_path: m.logo_path,
            comment: m.base.comment.unwrap_or_else(|| "-".to_string()),
            created_at: format_timestamp(m.base.metadata.created_at),
        }
    }
}

fn format_timestamp(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

impl Sortable for MarketplaceRow {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "description" => self
                .description
                .to_lowercase()
                .cmp(&other.description.to_lowercase()),
            "url" => self.url.to_lowercase().cmp(&other.url.to_lowercase()),
            "comment" => self
                .comment
                .to_lowercase()
                .cmp(&other.comment.to_lowercase()),
            "created_at" => self.created_at.cmp(&other.created_at),
            _ => Ordering::Equal,
        }
    }
}

const TABLE_ID: &str = "a005-marketplace-table";
const COLUMN_WIDTHS_KEY: &str = "a005_marketplace_column_widths";

#[component]
#[allow(non_snake_case)]
pub fn MarketplaceList() -> impl IntoView {
    let modal_stack =
        use_context::<ModalStackService>().expect("ModalStackService not found in context");
    let (items, set_items) = signal::<Vec<MarketplaceRow>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (sort_field, set_sort_field) = signal::<String>("description".to_string());
    let (sort_ascending, set_sort_ascending) = signal::<bool>(true);

    let fetch = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_marketplaces().await {
                Ok(v) => {
                    let rows: Vec<MarketplaceRow> = v.into_iter().map(Into::into).collect();
                    set_items.set(rows);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let toggle_sort = move |field: &'static str| {
        move |_| {
            if sort_field.get() == field {
                set_sort_ascending.update(|v| *v = !*v);
            } else {
                set_sort_field.set(field.to_string());
                set_sort_ascending.set(true);
            }
        }
    };

    let sorted_items = move || {
        let mut v = items.get();
        sort_list(&mut v, &sort_field.get(), sort_ascending.get());
        v
    };

    let open_details_modal = move |id: Option<String>| {
        modal_stack.clear();
        modal_stack.push_with_frame(
            Some("max-width: min(1100px, 95vw); width: min(1100px, 95vw);".to_string()),
            Some("marketplace-modal".to_string()),
            move |handle| {
                let handle_saved = handle.clone();
                let handle_cancel = handle.clone();
                view! {
                    <MarketplaceDetails
                        id=id.clone()
                        on_saved=Callback::new(move |_| {
                            handle_saved.close();
                            fetch();
                        })
                        on_cancel=Callback::new(move |_| handle_cancel.close())
                    />
                }
                .into_any()
            },
        );
    };

    let handle_edit = move |id: String| {
        let items_clone = items.get();
        if items_clone.iter().any(|item| item.id == id) {
            open_details_modal(Some(id));
        }
    };

    fetch();

    let resize_initialized = leptos::prelude::StoredValue::new(false);
    Effect::new(move |_| {
        if !resize_initialized.get_value() {
            resize_initialized.set_value(true);
            spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(100).await;
                init_column_resize(TABLE_ID, COLUMN_WIDTHS_KEY);
            });
        }
    });

    view! {
        <div class="page">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">{"Маркетплейсы"}</h1>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| fetch()
                    >
                        {icon("refresh")}
                        " Обновить"
                    </Button>
                </div>
            </div>

            <div class="page__content">
                {move || error.get().map(|e| view! {
                    <div class="warning-box warning-box--error">
                        <span class="warning-box__icon">"⚠"</span>
                        <span class="warning-box__text">{e}</span>
                    </div>
                })}

                <div class="table-wrapper">
                    <TableCrosshairHighlight table_id=TABLE_ID.to_string() />

                    <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 700px;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell
                                    resizable=false
                                    attr:style="width: 50px; min-width: 50px; max-width: 50px;"
                                >
                                    {"Лого"}
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=200.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=toggle_sort("description")>
                                        "Наименование"
                                        <span class=move || get_sort_class(&sort_field.get(), "description")>
                                            {move || get_sort_indicator(&sort_field.get(), "description", sort_ascending.get())}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=200.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=toggle_sort("url")>
                                        "URL"
                                        <span class=move || get_sort_class(&sort_field.get(), "url")>
                                            {move || get_sort_indicator(&sort_field.get(), "url", sort_ascending.get())}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=150.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=toggle_sort("comment")>
                                        "Комментарий"
                                        <span class=move || get_sort_class(&sort_field.get(), "comment")>
                                            {move || get_sort_indicator(&sort_field.get(), "comment", sort_ascending.get())}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=150.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=toggle_sort("created_at")>
                                        "Создано"
                                        <span class=move || get_sort_class(&sort_field.get(), "created_at")>
                                            {move || get_sort_indicator(&sort_field.get(), "created_at", sort_ascending.get())}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            {move || sorted_items().into_iter().map(|row| {
                                let id = row.id.clone();
                                let id_for_link = id.clone();
                                let logo_path = row.logo_path.clone();
                                view! {
                                    <TableRow on:click=move |_| handle_edit(id.clone())>
                                        <TableCell attr:style="width: 60px; max-width: 60px; padding: 4px 8px;">
                                            {if let Some(path) = logo_path {
                                                view! {
                                                    <img src={path} alt="logo" style="max-width: 40px; max-height: 24px; display: block; object-fit: contain;" />
                                                }.into_any()
                                            } else {
                                                view! { <span style="color: var(--color-text-tertiary);">{"—"}</span> }.into_any()
                                            }}
                                        </TableCell>
                                        <TableCell>
                                            <TableCellLayout truncate=true>
                                                <a
                                                    href="#"
                                                    style="color: var(--colorBrandForeground1); text-decoration: none; cursor: pointer;"
                                                    on:click=move |e| {
                                                        e.prevent_default();
                                                        e.stop_propagation();
                                                        handle_edit(id_for_link.clone());
                                                    }
                                                >
                                                    {row.description}
                                                </a>
                                            </TableCellLayout>
                                        </TableCell>
                                        <TableCell><TableCellLayout truncate=true>{row.url}</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout truncate=true>{row.comment}</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout truncate=true>{row.created_at}</TableCellLayout></TableCell>
                                    </TableRow>
                                }
                            }).collect_view()}
                        </TableBody>
                    </Table>
                </div>
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
