use crate::domain::a002_organization::ui::details::OrganizationDetails;
use crate::shared::api_utils::api_base;
use crate::shared::icons::icon;
use crate::shared::modal_stack::ModalStackService;
use contracts::domain::a002_organization::aggregate::Organization;
use leptos::prelude::*;
use std::collections::HashSet;
use std::rc::Rc;
use thaw::*;

#[derive(Clone, Debug)]
pub struct OrganizationRow {
    pub id: String,
    pub code: String,
    pub description: String,
    pub full_name: String,
    pub inn: String,
    pub kpp: String,
    pub comment: String,
    pub created_at: String,
}

impl From<Organization> for OrganizationRow {
    fn from(o: Organization) -> Self {
        use contracts::domain::common::AggregateId;

        Self {
            id: o.base.id.as_string(),
            code: o.base.code,
            description: o.base.description,
            full_name: o.full_name,
            inn: o.inn,
            kpp: o.kpp,
            comment: o.base.comment.unwrap_or_else(|| "-".to_string()),
            created_at: format_timestamp(o.base.metadata.created_at),
        }
    }
}

fn format_timestamp(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

#[component]
#[allow(non_snake_case)]
pub fn OrganizationList() -> impl IntoView {
    let (items, set_items) = signal::<Vec<OrganizationRow>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (show_modal, set_show_modal) = signal(false);
    let (editing_id, set_editing_id) = signal::<Option<String>>(None);
    let (selected, set_selected) = signal::<HashSet<String>>(HashSet::new());
    let modal_stack =
        use_context::<ModalStackService>().expect("ModalStackService not found in context");

    let fetch = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_organizations().await {
                Ok(v) => {
                    let rows: Vec<OrganizationRow> = v.into_iter().map(Into::into).collect();
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

    // Open modal via centralized stack (ModalFrame) so Details stays a full screen (its own header/actions).
    let open_details_modal = move |id: Option<String>| {
        // close any previous modal opened from this list
        modal_stack.clear();
        modal_stack.push_with_frame(
            // surface sizing is controlled here; Details component renders its own compact header.
            Some("max-width: min(1100px, 95vw); width: min(1100px, 95vw);".to_string()),
            Some("organization-details-modal".to_string()),
            move |handle| {
                let on_saved = Rc::new({
                    let handle = handle.clone();
                    move |_| {
                        handle.close();
                        fetch();
                    }
                });
                let on_cancel = Rc::new({
                    let handle = handle.clone();
                    move |_| handle.close()
                });

                view! {
                    <OrganizationDetails
                        id=id.clone()
                        on_saved=on_saved
                        on_cancel=on_cancel
                    />
                }
                .into_any()
            },
        );
    };

    let toggle_select = move |id: String, checked: bool| {
        set_selected.update(|s| {
            if checked {
                s.insert(id.clone());
            } else {
                s.remove(&id);
            }
        });
    };

    let clear_selection = move || set_selected.set(HashSet::new());

    let delete_selected = move || {
        let ids: Vec<String> = selected.get().into_iter().collect();
        if ids.is_empty() {
            return;
        }

        let count = ids.len();
        // Simple confirm dialog via browser
        let confirmed = {
            if let Some(win) = web_sys::window() {
                win.confirm_with_message(&format!(
                    "Удалить выбранные элементы? Количество: {}",
                    count
                ))
                .unwrap_or(false)
            } else {
                false
            }
        };
        if !confirmed {
            return;
        }

        wasm_bindgen_futures::spawn_local(async move {
            let mut all_ok = true;
            for id in ids {
                if let Err(_) = delete_organization(&id).await {
                    all_ok = false;
                }
            }
            if all_ok {
                // refresh list and clear selection
                // Use window setTimeout microtask to avoid borrowing issues
                let _ = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(
                    &wasm_bindgen::JsValue::UNDEFINED,
                ))
                .await;
            }
        });
        // Immediately refetch and clear selection (optimistic)
        fetch();
        clear_selection();
    };

    fetch();

    view! {
        <div class="page">
            // Page header with title and action buttons
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">{"Организации"}</h1>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=move |_| handle_create_new()
                    >
                        {icon("plus")}
                        " Новая организация"
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| fetch()
                    >
                        {icon("refresh")}
                        " Обновить"
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| delete_selected()
                        disabled=Signal::derive(move || selected.get().is_empty())
                    >
                        {icon("delete")}
                        {move || format!(" Удалить ({})", selected.get().len())}
                    </Button>
                </div>
            </div>

                    {move || error.get().map(|e| view! {
                        <div class="warning-box warning-box--error">
                            <span class="warning-box__icon">"⚠"</span>
                            <span class="warning-box__text">{e}</span>
                        </div>
                    })}

                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell resizable=false class="fixed-checkbox-column">
                                    <input
                                        type="checkbox"
                                        class="table__checkbox"
                                        prop:checked=move || {
                                            let current = items.get();
                                            let sel = selected.get();
                                            !current.is_empty() && current.iter().all(|item| sel.contains(&item.id))
                                        }
                                        on:change=move |ev| {
                                            let checked = event_target_checked(&ev);
                                            let current_items = items.get();
                                            if checked {
                                                set_selected.update(|s| {
                                                    for item in current_items.iter() {
                                                        s.insert(item.id.clone());
                                                    }
                                                });
                                            } else {
                                                set_selected.set(HashSet::new());
                                            }
                                        }
                                    />
                                </TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=200.0>{"Наименование"}</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=120.0>{"ИНН"}</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=100.0>{"КПП"}</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=150.0>{"Комментарий"}</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=150.0>{"Создано"}</TableHeaderCell>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            {move || items.get().into_iter().map(|row| {
                                let id = row.id.clone();
                                let id_for_click = id.clone();
                                let id_for_checkbox = id.clone();
                                let id_for_toggle = id.clone();
                                let is_selected = selected.get().contains(&id);
                                view! {
                                    <TableRow
                                        class:table__row--selected=is_selected
                                        on:click=move |_| handle_edit(id_for_click.clone())
                                    >
                                        <TableCell class="fixed-checkbox-column" on:click=|e| e.stop_propagation()>
                                            <input
                                                type="checkbox"
                                                class="table__checkbox"
                                                prop:checked=move || selected.get().contains(&id_for_checkbox)
                                                on:change=move |ev| {
                                                    let checked = event_target_checked(&ev);
                                                    toggle_select(id_for_toggle.clone(), checked);
                                                }
                                            />
                                        </TableCell>
                                        <TableCell><TableCellLayout>{row.description}</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout>{row.inn}</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout>{row.kpp}</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout>{row.comment}</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout>{row.created_at}</TableCellLayout></TableCell>
                                    </TableRow>
                                }
                            }).collect_view()}
                        </TableBody>
                    </Table>

                    <Show when=move || show_modal.get()>
                        {move || {
                            // Bridge old local state to new modal stack mechanism (keeps list code minimal).
                            open_details_modal(editing_id.get());
                            set_show_modal.set(false);
                            view! { <></> }
                        }}
                    </Show>
        </div>
    }
}

async fn fetch_organizations() -> Result<Vec<Organization>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/organization", api_base());
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
    let data: Vec<Organization> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

async fn delete_organization(id: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("DELETE");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/organization/{}", api_base(), id);
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
    Ok(())
}
