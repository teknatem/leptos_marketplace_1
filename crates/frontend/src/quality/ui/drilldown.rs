//! Переиспользуемые панели drill-down для подсистемы контроля качества.
//!
//! `RegistratorGroupsPanel` (список регистраторов с пагинацией, сортировкой,
//! перепроведением/очисткой) и `ProjectionRowsPanel` (строки одного регистратора)
//! используются страницей детализации правила `quality_check_details`.

use crate::shared::api_utils::api_base;
use crate::shared::icons::icon;
use contracts::quality::{
    NipCleanupResult, NipGroupsResponse, NipProjectionRow, NipRegistratorGroup, NipRepostResult,
};
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn document_id_from_registrator_ref(registrator_ref: &str) -> String {
    registrator_ref
        .split_once(':')
        .map(|(_, id)| id.to_string())
        .unwrap_or_else(|| registrator_ref.to_string())
}

pub fn is_cleanup_check(check_id: &str) -> bool {
    check_id == "projection_orphan_registrators"
}

// ---------------------------------------------------------------------------
// RegistratorGroupsPanel — список регистраторов с пагинацией и сортировкой
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
struct GroupsSortState {
    field: String,
    desc: bool,
}

impl Default for GroupsSortState {
    fn default() -> Self {
        Self {
            field: "missing_count".to_string(),
            desc: true,
        }
    }
}

#[component]
#[allow(non_snake_case)]
pub fn RegistratorGroupsPanel(
    check_id: String,
    projection_table: String,
    projection_label: String,
    #[prop(into)] on_back: Callback<()>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_open_doc: Callback<(String, String, String)>,
    #[prop(into)] on_open_rows: Callback<(String, String, String)>,
) -> impl IntoView {
    let (groups_resp, set_groups_resp) = signal::<Option<NipGroupsResponse>>(None);
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (sort, set_sort) = signal(GroupsSortState::default());
    let (page, set_page) = signal::<i64>(0);
    let page_size: i64 = 500;
    let (checked, set_checked) = signal::<std::collections::HashSet<String>>(Default::default());
    let (repost_loading, set_repost_loading) = signal(false);
    let (repost_msg, set_repost_msg) = signal::<Option<String>>(None);
    let (reload, set_reload) = signal(0u32);

    {
        let cid = check_id.clone();
        let ptable = projection_table.clone();
        Effect::new(move |_| {
            let sv = sort.get();
            let pv = page.get();
            let _ = reload.get();
            let cid = cid.clone();
            let ptable = ptable.clone();
            set_loading.set(true);
            set_error.set(None);
            spawn_local(async move {
                let url = format!(
                    "{}/api/quality/checks/{}/groups?projection_table={}&page={}&page_size={}&sort_by={}&sort_desc={}",
                    api_base(), cid, ptable, pv, page_size, sv.field, sv.desc
                );
                match Request::get(&url).send().await {
                    Ok(resp) if resp.status() == 200 => {
                        match resp.json::<NipGroupsResponse>().await {
                            Ok(data) => {
                                set_groups_resp.set(Some(data));
                                set_loading.set(false);
                            }
                            Err(e) => {
                                set_error.set(Some(format!("Ошибка разбора: {e}")));
                                set_loading.set(false);
                            }
                        }
                    }
                    Ok(resp) => {
                        set_error.set(Some(format!("HTTP {}", resp.status())));
                        set_loading.set(false);
                    }
                    Err(e) => {
                        set_error.set(Some(format!("Ошибка запроса: {e}")));
                        set_loading.set(false);
                    }
                }
            });
        });
    }

    let toggle_sort = move |field: &str| {
        let field = field.to_string();
        set_sort.update(|s| {
            if s.field == field {
                s.desc = !s.desc;
            } else {
                s.field = field;
                s.desc = true;
            }
        });
        set_page.set(0);
        set_checked.update(|c| c.clear());
    };

    let sort_icon = move |field: &str| {
        let s = sort.get();
        if s.field == field {
            if s.desc {
                " ↓"
            } else {
                " ↑"
            }
        } else {
            ""
        }
    };

    let total_pages = move || {
        groups_resp
            .get()
            .map(|r| (r.total + page_size - 1) / page_size)
            .unwrap_or(1)
    };
    let cleanup_mode = is_cleanup_check(&check_id);

    let do_bulk_repost = {
        let cid = check_id.clone();
        let ptable = projection_table.clone();
        move || {
            let cleanup_mode = is_cleanup_check(&cid);
            let checked_list: Vec<String> = checked.get().iter().cloned().collect();
            if checked_list.is_empty() {
                return;
            }

            let selected_docs: Vec<(String, String)> = groups_resp
                .get()
                .map(|r| {
                    r.items
                        .iter()
                        .filter(|g| {
                            let can_act = if cleanup_mode {
                                g.can_cleanup
                            } else {
                                g.can_post
                            };
                            can_act && checked_list.contains(&g.registrator_ref)
                        })
                        .map(|g| (g.registrator_type.clone(), g.registrator_ref.clone()))
                        .collect()
                })
                .unwrap_or_default();

            if selected_docs.is_empty() {
                set_repost_msg.set(Some(
                    "Не удалось определить доступные строки для действия".to_string(),
                ));
                return;
            }

            let total_requested = selected_docs.len();
            set_repost_loading.set(true);
            set_repost_msg.set(Some(format!(
                "{}: 0/{}",
                if cleanup_mode {
                    "Очистка"
                } else {
                    "Перепроведение"
                },
                total_requested
            )));

            let cid = cid.clone();
            let ptable = ptable.clone();
            spawn_local(async move {
                let url = format!(
                    "{}/api/quality/checks/{}/{}",
                    api_base(),
                    cid,
                    if cleanup_mode { "cleanup" } else { "repost" }
                );
                let mut completed = 0usize;
                let mut affected = 0usize;
                let mut errors: Vec<String> = Vec::new();

                for (reg_type, reg_ref) in selected_docs {
                    let body = if cleanup_mode {
                        json!({
                            "projection_table": ptable.clone(),
                            "registrator_refs": [reg_ref.clone()],
                        })
                    } else {
                        json!({
                            "projection_table": ptable.clone(),
                            "registrator_type": reg_type,
                            "registrator_refs": [reg_ref.clone()],
                        })
                    };

                    match Request::post(&url)
                        .header("Content-Type", "application/json")
                        .body(body.to_string())
                        .unwrap()
                        .send()
                        .await
                    {
                        Ok(resp) if resp.status() == 200 => {
                            if cleanup_mode {
                                if let Ok(result) = resp.json::<NipCleanupResult>().await {
                                    affected += result.deleted_rows;
                                    errors.extend(result.errors);
                                } else {
                                    errors.push(format!("{reg_ref}: ошибка разбора ответа"));
                                }
                            } else if let Ok(result) = resp.json::<NipRepostResult>().await {
                                affected += result.reposted;
                                errors.extend(result.errors);
                            } else {
                                errors.push(format!("{reg_ref}: ошибка разбора ответа"));
                            }
                        }
                        Ok(resp) => {
                            errors.push(format!("{reg_ref}: HTTP {}", resp.status()));
                        }
                        Err(e) => {
                            errors.push(format!("{reg_ref}: {e}"));
                        }
                    }

                    completed += 1;
                    set_repost_msg.set(Some(format!(
                        "{}: {}/{}, {}: {}, ошибок: {}",
                        if cleanup_mode {
                            "Очистка"
                        } else {
                            "Перепроведение"
                        },
                        completed,
                        total_requested,
                        if cleanup_mode {
                            "строк удалено"
                        } else {
                            "успешно"
                        },
                        affected,
                        errors.len()
                    )));
                }

                set_repost_loading.set(false);
                set_checked.update(|c| c.clear());
                set_reload.update(|t| *t += 1);
                let suffix = if errors.is_empty() {
                    String::new()
                } else {
                    format!(" (ошибок: {})", errors.len())
                };
                let final_msg = if cleanup_mode {
                    format!(
                        "Удалено строк: {} (групп: {}){}",
                        affected, total_requested, suffix
                    )
                } else {
                    format!("Перепроведено: {}/{}{}", affected, total_requested, suffix)
                };
                set_repost_msg.set(Some(final_msg));
            });
        }
    };

    view! {
        <div style="margin-top: 16px; border: 1px solid var(--color-border); border-radius: 6px; background: var(--color-surface); padding: 16px;">
            <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 12px; flex-wrap: wrap;">
                <thaw::Button appearance=thaw::ButtonAppearance::Subtle size=thaw::ButtonSize::Small
                    on_click=move |_| on_back.run(())
                >
                    {icon("arrow-left")} " Назад"
                </thaw::Button>
                <span style="font-weight: 600; font-size: 0.9rem;">"Нарушения по регистраторам"</span>
                <span class="badge badge--secondary" style="font-size: 0.75rem;">{projection_label.clone()}</span>
                {move || groups_resp.get().map(|r| view! {
                    <span class="badge badge--error" style="font-size: 0.75rem;">{format!("Всего: {}", r.total)}</span>
                })}
                <div style="margin-left: auto; display: flex; gap: 6px; align-items: center;">
                    <thaw::Button
                        appearance=thaw::ButtonAppearance::Primary
                        size=thaw::ButtonSize::Small
                        disabled=Signal::derive(move || {
                            let n = checked.get().len();
                            let can = groups_resp.get()
                                .map(|r| {
                                    let ch = checked.get();
                                    r.items.iter().any(|g| {
                                        let can_act = if cleanup_mode { g.can_cleanup } else { g.can_post };
                                        can_act && ch.contains(&g.registrator_ref)
                                    })
                                })
                                .unwrap_or(false);
                            repost_loading.get() || n == 0 || !can
                        })
                        on_click=move |_| do_bulk_repost()
                    >
                        {if cleanup_mode { icon("trash") } else { icon("refresh-cw") }}
                        {move || format!(
                            " {} ({})",
                            if cleanup_mode { "Очистить" } else { "Перепровести" },
                            checked.get().len()
                        )}
                    </thaw::Button>
                    {move || repost_msg.get().map(|msg| view! {
                        <span style="font-size: 0.8rem; color: var(--color-text-secondary);">{msg}</span>
                    })}
                    <thaw::Button appearance=thaw::ButtonAppearance::Secondary size=thaw::ButtonSize::Small
                        on_click=move |_| { set_page.set(0); set_checked.update(|c| c.clear()); set_reload.update(|t| *t += 1); }
                        disabled=loading.get()
                    >
                        {icon("refresh")}
                    </thaw::Button>
                    <thaw::Button appearance=thaw::ButtonAppearance::Subtle size=thaw::ButtonSize::Small
                        on_click=move |_| on_close.run(())
                    >
                        {icon("x")}
                    </thaw::Button>
                </div>
            </div>

            {move || error.get().map(|e| view! {
                <div class="warning-box" style="margin-bottom: 8px;">{e}</div>
            })}

            {move || if loading.get() && groups_resp.get().is_none() {
                view! { <div style="padding: 12px; color: var(--color-text-secondary);">"Загрузка..."</div> }.into_any()
            } else if let Some(resp) = groups_resp.get() {
                let items = resp.items.clone();
                let total = resp.total;
                let current_page = resp.page;
                let tpages = total_pages();
                view! {
                    <div>
                        {
                            // Группируем по типу регистратора, сохраняя порядок появления.
                            let mut sections: Vec<(String, Vec<NipRegistratorGroup>)> = Vec::new();
                            for g in items.into_iter() {
                                if let Some(s) = sections.iter_mut().find(|(t, _)| *t == g.registrator_type) {
                                    s.1.push(g);
                                } else {
                                    let t = g.registrator_type.clone();
                                    sections.push((t, vec![g]));
                                }
                            }

                            sections.into_iter().map(|(_rtype, sec_items)| {
                                let type_label = sec_items.first().map(|g| g.registrator_type_label.clone()).unwrap_or_default();
                                let src_headers: Vec<String> = sec_items.first()
                                    .map(|g| g.source_columns.iter().map(|c| c.label.clone()).collect())
                                    .unwrap_or_default();
                                let has_src = !src_headers.is_empty();
                                let sec_count = sec_items.len();
                                let actionable: Vec<String> = sec_items.iter()
                                    .filter(|g| if cleanup_mode { g.can_cleanup } else { g.can_post })
                                    .map(|g| g.registrator_ref.clone())
                                    .collect();
                                let src_headers_head = src_headers.clone();
                                view! {
                                    <div style="margin-bottom: 18px;">
                                        <div style="display: flex; align-items: center; gap: 8px; margin: 10px 0 4px;">
                                            <span class="badge badge--secondary" style="font-size: 0.78rem;">{type_label}</span>
                                            <span style="font-size: 0.78rem; color: var(--color-text-secondary);">{format!("{} док.", sec_count)}</span>
                                        </div>
                                        <table class="table__data table--striped" style="font-size: 0.82rem;">
                                            <thead class="table__head">
                                                <tr>
                                                    <th class="table__header-cell" style="width: 32px;">
                                                        {
                                                            let actionable_h = actionable.clone();
                                                            move || {
                                                                let ap = actionable_h.clone();
                                                                let ch = checked.get();
                                                                let all_checked = !ap.is_empty() && ap.iter().all(|r| ch.contains(r.as_str()));
                                                                view! {
                                                                    <input type="checkbox" checked=all_checked
                                                                        on:change=move |_| {
                                                                            let ap = ap.clone();
                                                                            set_checked.update(|c| {
                                                                                if all_checked { for r in &ap { c.remove(r); } }
                                                                                else { for r in &ap { c.insert(r.clone()); } }
                                                                            });
                                                                        }
                                                                    />
                                                                }
                                                            }
                                                        }
                                                    </th>
                                                    {if has_src {
                                                        src_headers_head.iter().map(|h| view! {
                                                            <th class="table__header-cell">{h.clone()}</th>
                                                        }).collect_view().into_any()
                                                    } else {
                                                        view! {
                                                            <th class="table__header-cell">"ID документа"</th>
                                                            <th class="table__header-cell">"Дата (мин)"</th>
                                                            <th class="table__header-cell">"Дата (макс)"</th>
                                                        }.into_any()
                                                    }}
                                                    <th class="table__header-cell" style="cursor: pointer; text-align: right;"
                                                        on:click=move |_| toggle_sort("missing_count")
                                                    >
                                                        "Нарушений" {move || sort_icon("missing_count")}
                                                    </th>
                                                    <th class="table__header-cell table__header-cell--center" style="width: 180px;">"Действия"</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                            {sec_items.into_iter().map(|g| {
                                                let rref = g.registrator_ref.clone();
                                                let rref_cb = rref.clone();
                                                let rref_rows = rref.clone();
                                                let rref_repost = rref.clone();
                                                let rtype_rows = g.registrator_type.clone();
                                                let rtype_repost = g.registrator_type.clone();
                                                let tab_prefix = g.tab_key_prefix.clone();
                                                let rlabel = format!("{}: {}", g.registrator_type_label, g.display_short);
                                                let rlabel_rows = rlabel.clone();
                                                let doc_id_open = document_id_from_registrator_ref(&rref);
                                                let doc_label_open = rlabel.clone();
                                                let can_post = g.can_post;
                                                let can_cleanup = g.can_cleanup;
                                                let can_act = if cleanup_mode { can_cleanup } else { can_post };
                                                let is_checked = checked.get().contains(&rref);
                                                let n_headers = src_headers.len();
                                                let source_cells: Vec<(String, bool)> = if g.source_columns.is_empty() {
                                                    (0..n_headers).map(|_| (String::new(), false)).collect()
                                                } else {
                                                    g.source_columns.iter().map(|c| (c.value.clone(), c.align_right)).collect()
                                                };
                                                view! {
                                                    <tr class="table__row">
                                                        <td class="table__cell" style="text-align: center;">
                                                            {if can_act {
                                                                view! {
                                                                    <input type="checkbox" checked=is_checked
                                                                        on:change=move |_| {
                                                                            let r = rref_cb.clone();
                                                                            set_checked.update(|c| {
                                                                                if c.contains(&r) { c.remove(&r); } else { c.insert(r); }
                                                                            });
                                                                        }
                                                                    />
                                                                }.into_any()
                                                            } else { view! { <span /> }.into_any() }}
                                                        </td>
                                                        {if has_src {
                                                            source_cells.into_iter().map(|(v, ar)| {
                                                                let style = if ar { "text-align: right; font-variant-numeric: tabular-nums;" } else { "" };
                                                                view! { <td class="table__cell" style=style>{v}</td> }
                                                            }).collect_view().into_any()
                                                        } else {
                                                            let min_d = g.min_entry_date.clone().unwrap_or_default();
                                                            let max_d = g.max_entry_date.clone().unwrap_or_default();
                                                            view! {
                                                                <td class="table__cell" style="font-family: monospace; font-size: 0.78rem;">{g.display_short.clone()}</td>
                                                                <td class="table__cell" style="color: var(--color-text-secondary);">{min_d}</td>
                                                                <td class="table__cell" style="color: var(--color-text-secondary);">{max_d}</td>
                                                            }.into_any()
                                                        }}
                                                        <td class="table__cell" style="text-align: right; color: var(--color-error); font-weight: 600;">
                                                            {g.missing_count.to_string()}
                                                        </td>
                                                        <td class="table__cell table__cell--center">
                                                            <div style="display: flex; gap: 4px; justify-content: center;">
                                                                {if let Some(prefix) = tab_prefix {
                                                                    view! {
                                                                        <thaw::Button
                                                                            appearance=thaw::ButtonAppearance::Subtle
                                                                            size=thaw::ButtonSize::Small
                                                                            on_click=move |_| on_open_doc.run((prefix.clone(), doc_id_open.clone(), doc_label_open.clone()))
                                                                        >
                                                                            {icon("arrow-right")} " Открыть"
                                                                        </thaw::Button>
                                                                    }.into_any()
                                                                } else { view! { <span /> }.into_any() }}
                                                                <thaw::Button
                                                                    appearance=thaw::ButtonAppearance::Subtle
                                                                    size=thaw::ButtonSize::Small
                                                                    on_click=move |_| on_open_rows.run((rtype_rows.clone(), rref_rows.clone(), rlabel_rows.clone()))
                                                                >
                                                                    {icon("eye")} " Строки"
                                                                </thaw::Button>
                                                                {if can_act {
                                                                    let rtype_r = rtype_repost.clone();
                                                                    let rref_r = rref_repost.clone();
                                                                    let cid_r = check_id.clone();
                                                                    let ptable_r = projection_table.clone();
                                                                    view! {
                                                                        <thaw::Button
                                                                            appearance=thaw::ButtonAppearance::Subtle
                                                                            size=thaw::ButtonSize::Small
                                                                            disabled=repost_loading.get()
                                                                            on_click=move |_| {
                                                                                let rtype_a = rtype_r.clone();
                                                                                let rref_a = rref_r.clone();
                                                                                let cid_a = cid_r.clone();
                                                                                let ptable_a = ptable_r.clone();
                                                                                set_repost_loading.set(true);
                                                                                set_repost_msg.set(None);
                                                                                spawn_local(async move {
                                                                                    let cleanup_mode = is_cleanup_check(&cid_a);
                                                                                    let body = if cleanup_mode {
                                                                                        json!({
                                                                                            "projection_table": ptable_a,
                                                                                            "registrator_refs": [rref_a],
                                                                                        })
                                                                                    } else {
                                                                                        json!({
                                                                                            "projection_table": ptable_a,
                                                                                            "registrator_type": rtype_a,
                                                                                            "registrator_refs": [rref_a],
                                                                                        })
                                                                                    };
                                                                                    let url = format!(
                                                                                        "{}/api/quality/checks/{}/{}",
                                                                                        api_base(),
                                                                                        cid_a,
                                                                                        if cleanup_mode { "cleanup" } else { "repost" }
                                                                                    );
                                                                                    match Request::post(&url)
                                                                                        .header("Content-Type", "application/json")
                                                                                        .body(body.to_string())
                                                                                        .unwrap()
                                                                                        .send()
                                                                                        .await
                                                                                    {
                                                                                        Ok(resp) if resp.status() == 200 => {
                                                                                            if cleanup_mode {
                                                                                                if let Ok(r) = resp.json::<NipCleanupResult>().await {
                                                                                                    set_repost_msg.set(Some(format!("Удалено строк: {}", r.deleted_rows)));
                                                                                                }
                                                                                            } else if let Ok(r) = resp.json::<NipRepostResult>().await {
                                                                                                set_repost_msg.set(Some(format!("Перепроведено: {}/{}", r.reposted, r.requested)));
                                                                                            }
                                                                                            set_repost_loading.set(false);
                                                                                            set_reload.update(|t| *t += 1);
                                                                                        }
                                                                                        Ok(resp) => {
                                                                                            set_repost_msg.set(Some(format!("HTTP {}", resp.status())));
                                                                                            set_repost_loading.set(false);
                                                                                        }
                                                                                        Err(e) => {
                                                                                            set_repost_msg.set(Some(format!("Ошибка: {e}")));
                                                                                            set_repost_loading.set(false);
                                                                                        }
                                                                                    }
                                                                                });
                                                                            }
                                                                        >
                                                                            {if cleanup_mode { icon("trash") } else { icon("refresh-cw") }}
                                                                        </thaw::Button>
                                                                    }.into_any()
                                                                } else { view! { <span /> }.into_any() }}
                                                            </div>
                                                        </td>
                                                    </tr>
                                                }
                                            }).collect_view()}
                                            </tbody>
                                        </table>
                                    </div>
                                }
                            }).collect_view()
                        }

                        {if tpages > 1 {
                            let is_first = current_page == 0;
                            let is_last = current_page >= tpages - 1;
                            let tp2 = tpages;
                            view! {
                                <div style="display: flex; align-items: center; gap: 8px; padding: 8px 0; font-size: 0.82rem;">
                                    <thaw::Button
                                        appearance=thaw::ButtonAppearance::Subtle
                                        size=thaw::ButtonSize::Small
                                        disabled=Signal::derive(move || is_first)
                                        on_click=move |_| { set_page.update(|p| *p = (*p - 1).max(0)); set_checked.update(|c| c.clear()); }
                                    >
                                        {icon("chevron-left")}
                                    </thaw::Button>
                                    <span style="color: var(--color-text-secondary);">
                                        {format!("Стр. {} из {} (всего {})", current_page + 1, tp2, total)}
                                    </span>
                                    <thaw::Button
                                        appearance=thaw::ButtonAppearance::Subtle
                                        size=thaw::ButtonSize::Small
                                        disabled=Signal::derive(move || is_last)
                                        on_click=move |_| { set_page.update(|p| *p = (*p + 1).min(tp2 - 1)); set_checked.update(|c| c.clear()); }
                                    >
                                        {icon("chevron-right")}
                                    </thaw::Button>
                                </div>
                            }.into_any()
                        } else {
                            view! { <div style="padding: 6px 0; font-size: 0.78rem; color: var(--color-text-tertiary);">{format!("Всего: {}", total)}</div> }.into_any()
                        }}
                    </div>
                }.into_any()
            } else {
                view! { <div style="padding: 12px; color: var(--color-text-secondary);">"Нет данных"</div> }.into_any()
            }}
        </div>
    }
}

// ---------------------------------------------------------------------------
// ProjectionRowsPanel — строки проекции для одного регистратора
// ---------------------------------------------------------------------------

#[component]
#[allow(non_snake_case)]
pub fn ProjectionRowsPanel(
    check_id: String,
    projection_table: String,
    registrator_ref: String,
    registrator_label: String,
    #[prop(into)] on_back: Callback<()>,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let (rows, set_rows) = signal::<Vec<NipProjectionRow>>(Vec::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);

    let cid = check_id.clone();
    let ptable = projection_table.clone();
    let rref = registrator_ref.clone();

    Effect::new(move |_| {
        let url = format!(
            "{}/api/quality/checks/{}/rows?projection_table={}&registrator_ref={}",
            api_base(),
            cid,
            ptable,
            rref
        );
        set_loading.set(true);
        set_error.set(None);
        spawn_local(async move {
            match Request::get(&url).send().await {
                Ok(resp) if resp.status() == 200 => {
                    match resp.json::<Vec<NipProjectionRow>>().await {
                        Ok(data) => {
                            set_rows.set(data);
                            set_loading.set(false);
                        }
                        Err(e) => {
                            set_error.set(Some(format!("Ошибка разбора: {e}")));
                            set_loading.set(false);
                        }
                    }
                }
                Ok(resp) => {
                    set_error.set(Some(format!("HTTP {}", resp.status())));
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(format!("Ошибка запроса: {e}")));
                    set_loading.set(false);
                }
            }
        });
    });

    view! {
        <div style="margin-top: 16px; border: 1px solid var(--color-border); border-radius: 6px; background: var(--color-surface); padding: 16px;">
            <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 12px; flex-wrap: wrap;">
                <thaw::Button appearance=thaw::ButtonAppearance::Subtle size=thaw::ButtonSize::Small
                    on_click=move |_| on_back.run(())
                >
                    {icon("arrow-left")} " Назад"
                </thaw::Button>
                <span style="font-weight: 600; font-size: 0.9rem;">
                    {if is_cleanup_check(&check_id) { "Осиротевшие строки проекции" } else { "Строки без номенклатуры" }}
                </span>
                <span class="badge badge--secondary" style="font-size: 0.75rem;">{registrator_label.clone()}</span>
                {move || {
                    let cnt = rows.get().len();
                    if cnt > 0 {
                        view! { <span class="badge badge--error" style="font-size: 0.75rem;">{format!("{} строк", cnt)}</span> }.into_any()
                    } else { view! { <span /> }.into_any() }
                }}
                <div style="margin-left: auto;">
                    <thaw::Button appearance=thaw::ButtonAppearance::Subtle size=thaw::ButtonSize::Small
                        on_click=move |_| on_close.run(())
                    >
                        {icon("x")}
                    </thaw::Button>
                </div>
            </div>

            {move || error.get().map(|e| view! {
                <div class="warning-box" style="margin-bottom: 8px;">{e}</div>
            })}

            {move || if loading.get() {
                view! { <div style="padding: 12px; color: var(--color-text-secondary);">"Загрузка..."</div> }.into_any()
            } else if rows.get().is_empty() {
                view! { <div style="padding: 12px; color: var(--color-text-secondary);">"Нет строк"</div> }.into_any()
            } else {
                let items = rows.get();
                let has_context = items.first().map(|r| r.context_label.is_some()).unwrap_or(false);
                view! {
                    <table class="table__data table--striped" style="font-size: 0.82rem;">
                        <thead class="table__head">
                            <tr>
                                <th class="table__header-cell">"ID строки"</th>
                                <th class="table__header-cell">"Дата"</th>
                                <th class="table__header-cell">"Оборот"</th>
                                <th class="table__header-cell" style="text-align: right;">"Сумма"</th>
                                {if has_context { view! { <th class="table__header-cell">"Контекст"</th> }.into_any() } else { view! { <span /> }.into_any() }}
                            </tr>
                        </thead>
                        <tbody>
                        {items.into_iter().map(|row| {
                            let short_id = if row.id.len() > 12 { format!("{}…", &row.id[..12]) } else { row.id.clone() };
                            let context = row.context_label.as_deref().unwrap_or("").to_string()
                                + row.context_value.as_deref().map(|v| format!(": {}", v)).as_deref().unwrap_or("");
                            view! {
                                <tr class="table__row">
                                    <td class="table__cell" style="font-family: monospace; font-size: 0.75rem;" title=row.id.clone()>{short_id}</td>
                                    <td class="table__cell">{row.entry_date.clone()}</td>
                                    <td class="table__cell" style="font-size: 0.78rem; color: var(--color-text-secondary);">{row.turnover_code.clone()}</td>
                                    <td class="table__cell" style="text-align: right; font-variant-numeric: tabular-nums;">
                                        {format!("{:.2}", row.amount)}
                                    </td>
                                    {if has_context {
                                        view! { <td class="table__cell" style="font-size: 0.78rem; color: var(--color-text-secondary);">{context}</td> }.into_any()
                                    } else { view! { <span /> }.into_any() }}
                                </tr>
                            }
                        }).collect_view()}
                        </tbody>
                    </table>
                }.into_any()
            }}
        </div>
    }
}
