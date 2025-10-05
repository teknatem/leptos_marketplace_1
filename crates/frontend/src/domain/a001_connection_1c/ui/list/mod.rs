use crate::domain::a001_connection_1c::ui::details::Connection1CDetails;
use contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase;
use leptos::prelude::*;
use leptos_struct_table::TableDataProvider;
use leptos_struct_table::{TableRow, TailwindClassesPreset};
use std::fmt;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct DisplayableUrl(String);

impl fmt::Display for DisplayableUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.len() > 64 {
            write!(f, "{}...", &self.0[..61])
        } else {
            write!(f, "{}", self.0)
        }
    }
}

#[derive(TableRow, Clone, Debug)]
#[table(impl_vec_data_provider, classes_provider = "TailwindClassesPreset")]
pub struct Connection1CDatabaseRow {
    pub id: String,
    pub description: String,
    pub url: String,
    pub login: String,
    pub comment: String,
    pub password: String,
    pub is_primary: bool,
    pub is_deleted: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Connection1CDatabase> for Connection1CDatabaseRow {
    fn from(c: Connection1CDatabase) -> Self {
        use contracts::domain::common::AggregateId;

        Self {
            id: c.base.id.as_string(),
            description: c.base.description,
            url: {
                let mut s = c.url;
                if s.len() > 64 {
                    s.truncate(64);
                }
                s
            },
            login: c.login,
            comment: c.base.comment.unwrap_or_else(|| "-".to_string()),
            password: "••••".to_string(),
            is_primary: c.is_primary,
            is_deleted: c.base.metadata.is_deleted,
            created_at: format_timestamp(c.base.metadata.created_at),
            updated_at: format_timestamp(c.base.metadata.updated_at),
        }
    }
}

fn format_timestamp(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

#[component]
#[allow(non_snake_case)]
pub fn Connection1CList() -> impl IntoView {
    let (items, set_items) = signal::<Vec<Connection1CDatabaseRow>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (show_modal, set_show_modal) = signal(false);
    let (editing_id, set_editing_id) = signal::<Option<String>>(None);

    let fetch = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_connections().await {
                Ok(v) => {
                    let rows: Vec<Connection1CDatabaseRow> =
                        v.into_iter().map(Into::into).collect();
                    set_items.set(rows);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let handle_create_new = move || {
        set_editing_id.set(None);
        set_show_modal.set(true);
    };

    let handle_edit = move |id: String| {
        let items_clone = items.get();
        if items_clone.iter().any(|item| item.id == id) {
            set_editing_id.set(Some(id));
            set_show_modal.set(true);
        }
    };

    // Save happens inside details; list only refreshes on success

    let handle_cancel = move |_| {
        set_show_modal.set(false);
        set_editing_id.set(None);
    };

    fetch();

    view! {
        <div class="content">
            <div class="header">
                <h2>{"1C Database Connections"}</h2>
                <div class="header-actions">
                    <button class="btn btn-primary" on:click=move |_| handle_create_new()>
                        {"➕ New Connection"}
                    </button>
                    <button class="btn btn-secondary" on:click=move |_| fetch()>
                        {"🔄 Refresh"}
                    </button>
                </div>
            </div>

            {move || error.get().map(|e| view! { <div class="error">{e}</div> })}

            <div class="table-container">
                <table>
                    <thead>
                        <tr>
                            <th>{"Description"}</th>
                            <th>{"URL"}</th>
                            <th>{"Login"}</th>
                            <th>{"Comment"}</th>
                            <th>{"Primary"}</th>
                            <th>{"Created"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || items.get().into_iter().map(|row| {
                            let id = row.id.clone();
                            view! {
                                <tr on:click=move |_| handle_edit(id.clone())>
                                    <td>{row.description}</td>
                                    <td>{row.url}</td>
                                    <td>{row.login}</td>
                                    <td>{row.comment}</td>
                                    <td>{if row.is_primary { "✓" } else { "" }}</td>
                                    <td>{row.created_at}</td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            </div>

            {move || if show_modal.get() {
                view! {
                    <div class="modal-overlay">
                        <div class="modal-content">
                            <Connection1CDetails
                                id=editing_id.get()
                                on_saved=Rc::new(move |_| { set_show_modal.set(false); set_editing_id.set(None); fetch(); })
                                on_cancel=Rc::new(move |_| handle_cancel(()))
                            />
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }}
        </div>
    }
}

// Build API base URL. Always use port 3000 for the backend API.
fn api_base() -> String {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return String::new(),
    };
    let location = window.location();
    let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    format!("{}//{}:3000", protocol, hostname)
}

async fn fetch_connections() -> Result<Vec<Connection1CDatabase>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/connection_1c", api_base());
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
    let data: Vec<Connection1CDatabase> =
        serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}
