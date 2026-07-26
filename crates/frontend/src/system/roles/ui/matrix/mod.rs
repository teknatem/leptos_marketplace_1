use contracts::system::access::ScopeDescriptorDto;
use contracts::system::roles::{Role, RoleScopeAccess};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;
use thaw::*;
use wasm_bindgen::JsCast;

use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_SYSTEM;
use crate::system::auth::guard::RequireAdmin;
use crate::system::roles::api;

#[component]
pub fn RoleMatrixPage() -> impl IntoView {
    view! {
        <RequireAdmin>
            <RoleMatrix />
        </RequireAdmin>
    }
}

/// Map: role_id → Vec<RoleScopeAccess>
type PermissionsMap = HashMap<String, Vec<RoleScopeAccess>>;
/// Map: (role_id, scope_id) → pending mode override
type PendingChanges = HashMap<(String, String), String>;

fn get_mode(map: &PermissionsMap, role_id: &str, scope_id: &str) -> String {
    map.get(role_id)
        .and_then(|perms| perms.iter().find(|p| p.scope_id == scope_id))
        .map(|p| p.access_mode.clone())
        .unwrap_or_else(|| "-".to_string())
}

fn next_mode(current: &str) -> &'static str {
    match current {
        "-" => "read",
        "read" => "all",
        "all" => "-",
        _ => "-",
    }
}

fn mode_badge(mode: &str) -> &'static str {
    match mode {
        "all" => "ВСЕ",
        "read" => "чтение",
        _ => "—",
    }
}

/// CSS class for the role cell <td>. Padding/borders come from `.data-matrix td`;
/// this adds centering and (for custom roles) the edit affordance.
fn cell_td_class(is_system: bool) -> &'static str {
    if is_system {
        "roles-matrix__cell"
    } else {
        "roles-matrix__cell roles-matrix__cell--editable"
    }
}

/// CSS class for the pill badge rendered inside a cell — standard `.badge .badge--*`.
/// System roles are dimmed.
fn mode_pill_class(mode: &str, is_system: bool) -> String {
    let variant = match mode {
        "all" => "badge--success",
        "read" => "badge--primary",
        _ => "badge--neutral",
    };
    let mut class = format!("badge {variant}");
    if is_system {
        class.push_str(" roles-matrix__pill--dim");
    }
    class
}

fn category_label(cat: &str) -> &'static str {
    match cat {
        "references" => "Справочники",
        "marketplace_data" => "Торговые данные",
        "production" => "Производство и закупки",
        "analytics" => "Аналитика",
        "ai" => "AI / LLM",
        "imports" => "Импорт данных",
        "system" => "Системные функции",
        _ => "Прочее",
    }
}

fn escape_csv(s: &str) -> String {
    s.replace('"', "\"\"")
}

fn build_csv(
    roles: &[Role],
    scopes: &[ScopeDescriptorDto],
    permissions: &PermissionsMap,
) -> String {
    let mut csv = String::new();
    // UTF-8 BOM for correct Excel opening
    csv.push('\u{FEFF}');

    // Header
    csv.push_str("\"Категория\",\"Название\",\"Scope ID\"");
    for role in roles {
        csv.push_str(&format!(
            ",\"{} ({})\"",
            escape_csv(&role.name),
            escape_csv(&role.code)
        ));
    }
    csv.push('\n');

    for scope in scopes {
        let cat = category_label(&scope.category);
        csv.push_str(&format!(
            "\"{}\",\"{}\",\"{}\"",
            escape_csv(cat),
            escape_csv(&scope.label),
            escape_csv(&scope.scope_id),
        ));
        for role in roles {
            let mode = get_mode(permissions, &role.id, &scope.scope_id);
            let display = match mode.as_str() {
                "all" => "all",
                "read" => "read",
                _ => "-",
            };
            csv.push_str(&format!(",\"{}\"", display));
        }
        csv.push('\n');
    }
    csv
}

fn trigger_csv_download(filename: &str, content: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };

    let array = js_sys::Array::new();
    array.push(&wasm_bindgen::JsValue::from_str(content));

    let options = {
        let o = web_sys::BlobPropertyBag::new();
        // js_sys::Reflect is not needed — set via Object property
        let _ = js_sys::Reflect::set(&o, &"type".into(), &"text/csv;charset=utf-8".into());
        o
    };

    let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&array, &options) else {
        return;
    };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
        return;
    };

    let Ok(el) = document.create_element("a") else {
        return;
    };
    let Ok(a) = el.dyn_into::<web_sys::HtmlAnchorElement>() else {
        return;
    };
    a.set_href(&url);
    a.set_download(filename);
    // Append → click → remove to ensure Firefox compatibility
    if let Some(body) = document.body() {
        let _ = body.append_child(&a);
        a.click();
        let _ = body.remove_child(&a);
    } else {
        a.click();
    }
    let _ = web_sys::Url::revoke_object_url(&url);
}

fn category_order(cat: &str) -> usize {
    match cat {
        "references" => 0,
        "marketplace_data" => 1,
        "production" => 2,
        "analytics" => 3,
        "ai" => 4,
        "imports" => 5,
        "system" => 6,
        _ => 7,
    }
}

#[component]
fn RoleMatrix() -> impl IntoView {
    let roles: RwSignal<Vec<Role>> = RwSignal::new(Vec::new());
    let scopes: RwSignal<Vec<ScopeDescriptorDto>> = RwSignal::new(Vec::new());
    let permissions: RwSignal<PermissionsMap> = RwSignal::new(HashMap::new());
    let pending: RwSignal<PendingChanges> = RwSignal::new(HashMap::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (save_error, set_save_error) = signal::<Option<String>>(None);
    let (loading, set_loading) = signal(true);
    let (saving, set_saving) = signal(false);
    // Быстрый фильтр строк (по названию, scope_id, описанию).
    let search = RwSignal::new(String::new());

    let load_data = move || {
        set_loading.set(true);
        set_error.set(None);
        pending.set(HashMap::new());

        spawn_local(async move {
            let roles_res = api::fetch_roles().await;
            let scopes_res = api::fetch_scopes().await;

            match (roles_res, scopes_res) {
                (Ok(roles_data), Ok(scopes_data)) => {
                    let mut perm_map: PermissionsMap = HashMap::new();
                    for role in &roles_data {
                        match api::fetch_role_permissions(&role.id).await {
                            Ok(perms) => {
                                perm_map.insert(role.id.clone(), perms);
                            }
                            Err(_) => {
                                perm_map.insert(role.id.clone(), vec![]);
                            }
                        }
                    }
                    // Sort scopes by category order then by scope_id
                    let mut sorted = scopes_data;
                    sorted.sort_by(|a, b| {
                        let ca = category_order(&a.category);
                        let cb = category_order(&b.category);
                        ca.cmp(&cb).then(a.scope_id.cmp(&b.scope_id))
                    });
                    roles.set(roles_data);
                    scopes.set(sorted);
                    permissions.set(perm_map);
                    set_loading.set(false);
                }
                (Err(e), _) | (_, Err(e)) => {
                    set_error.set(Some(format!("Ошибка загрузки: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    Effect::new(move |_| {
        load_data();
    });

    let has_pending = move || !pending.get().is_empty();

    let export_csv = move |_| {
        let roles_snap = roles.get();
        let scopes_snap = scopes.get();
        let perms_snap = permissions.get();
        if roles_snap.is_empty() || scopes_snap.is_empty() {
            return;
        }
        let content = build_csv(&roles_snap, &scopes_snap, &perms_snap);
        trigger_csv_download("roles_matrix.csv", &content);
    };

    let save_changes = move |_| {
        let pending_snap = pending.get();
        let roles_snap = roles.get();
        let perm_snap = permissions.get();

        if pending_snap.is_empty() {
            return;
        }

        set_saving.set(true);
        set_save_error.set(None);

        // Group pending changes by role_id
        let mut by_role: HashMap<String, HashMap<String, String>> = HashMap::new();
        for ((role_id, scope_id), mode) in &pending_snap {
            by_role
                .entry(role_id.clone())
                .or_default()
                .insert(scope_id.clone(), mode.clone());
        }

        spawn_local(async move {
            let mut any_error = false;

            for (role_id, changes) in &by_role {
                // Find the role (only custom roles are editable)
                let role = roles_snap.iter().find(|r| &r.id == role_id);
                if role.map(|r| r.is_system).unwrap_or(true) {
                    continue;
                }

                // Merge pending into existing permissions
                let existing = perm_snap.get(role_id).cloned().unwrap_or_default();
                let mut new_grants: HashMap<String, String> = existing
                    .into_iter()
                    .map(|p| (p.scope_id, p.access_mode))
                    .collect();

                for (scope_id, mode) in changes {
                    if mode == "-" {
                        new_grants.remove(scope_id);
                    } else {
                        new_grants.insert(scope_id.clone(), mode.clone());
                    }
                }

                let grants: Vec<RoleScopeAccess> = new_grants
                    .into_iter()
                    .map(|(scope_id, access_mode)| RoleScopeAccess {
                        scope_id,
                        access_mode,
                    })
                    .collect();

                if let Err(e) = api::update_role_permissions(role_id, grants).await {
                    set_save_error.set(Some(format!("Ошибка сохранения роли {}: {}", role_id, e)));
                    any_error = true;
                    break;
                }
            }

            if !any_error {
                // Reload data
                pending.set(HashMap::new());
                load_data();
            }
            set_saving.set(false);
        });
    };

    view! {
        <PageFrame page_id="sys_roles_matrix" category=PAGE_CAT_SYSTEM>
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Матрица прав"</h1>
                    {move || {
                        let r = roles.get().len();
                        let s = scopes.get().len();
                        let p = pending.get().len();
                        view! {
                            <Badge>{format!("{} ролей × {} scope", r, s)}</Badge>
                            {if p > 0 { view! {
                                <Badge color=BadgeColor::Warning>{format!("{} изменений", p)}</Badge>
                            }.into_any() } else { view! {<></>}.into_any() }}
                        }
                    }}
                </div>
                <div class="page__header-right" style="display:flex;gap:8px;align-items:center;">
                    {move || if has_pending() {
                        view! {
                            <Button
                                appearance=ButtonAppearance::Primary
                                on_click=save_changes
                                disabled=Signal::derive(move || saving.get())
                            >
                                {icon("save")}
                                " Сохранить"
                            </Button>
                            <Button
                                appearance=ButtonAppearance::Secondary
                                on_click=move |_| { pending.set(HashMap::new()); }
                                disabled=Signal::derive(move || saving.get())
                            >
                                "Отмена"
                            </Button>
                        }.into_any()
                    } else {
                        view! {<></>}.into_any()
                    }}
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=export_csv
                        disabled=Signal::derive(move || loading.get())
                    >
                        {icon("download")}
                        "Excel (csv)"
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| load_data()
                        disabled=Signal::derive(move || loading.get())
                    >
                        {icon("refresh")}
                        " Обновить"
                    </Button>
                </div>
            </div>

            <div class="page__content">
                {move || error.get().map(|e| view! { <div class="alert alert--error">{e}</div> })}
                {move || save_error.get().map(|e| view! { <div class="alert alert--error">{e}</div> })}

                {move || if loading.get() {
                    view! { <div class="page__loading">"Загрузка матрицы..."</div> }.into_any()
                } else {
                    let roles_snap = roles.get();
                    let scopes_snap = scopes.get();
                    let total = scopes_snap.len();

                    // Быстрый фильтр строк по названию / scope_id / описанию.
                    let needle = search.get().trim().to_lowercase();
                    let filtered: Vec<ScopeDescriptorDto> = scopes_snap
                        .into_iter()
                        .filter(|s| {
                            needle.is_empty()
                                || format!("{} {} {}", s.label, s.scope_id, s.description)
                                    .to_lowercase()
                                    .contains(&needle)
                        })
                        .collect();
                    let shown = filtered.len();

                    // Group filtered scopes by category (пустые категории не появляются).
                    let mut grouped: Vec<(String, Vec<ScopeDescriptorDto>)> = Vec::new();
                    for scope in filtered.into_iter() {
                        let cat = scope.category.clone();
                        if let Some(last) = grouped.last_mut() {
                            if last.0 == cat {
                                last.1.push(scope);
                                continue;
                            }
                        }
                        grouped.push((cat, vec![scope]));
                    }

                    view! {
                        <div class="roles-matrix__toolbar">
                            <input
                                class="form__input roles-matrix__search"
                                placeholder="Поиск: название, scope, описание…"
                                prop:value=move || search.get()
                                on:input=move |ev| search.set(event_target_value(&ev))
                            />
                            <span class="badge badge--neutral">{format!("{shown} из {total}")}</span>
                        </div>
                        <div class="table-wrapper">
                            <table class="data-matrix roles-matrix">
                                <thead>
                                    <tr>
                                        <th class="data-matrix__corner roles-matrix__head roles-matrix__name">
                                            "Название"
                                        </th>
                                        <th class="data-matrix__corner roles-matrix__head roles-matrix__scope">
                                            "Scope ID"
                                        </th>
                                        {roles_snap.iter().map(|role| {
                                            let name = role.name.clone();
                                            let code = role.code.clone();
                                            view! {
                                                <th class="data-matrix__sticky-head roles-matrix__head roles-matrix__role-col">
                                                    <div class="roles-matrix__role-name">{name}</div>
                                                    <code class="roles-matrix__role-code">{code}</code>
                                                </th>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </tr>
                                </thead>
                                <tbody>
                                    {grouped.into_iter().map(|(cat, cat_scopes)| {
                                        let cat_label = category_label(&cat);
                                        let roles_inner = roles_snap.clone();
                                        // +2 for the two sticky columns
                                        let col_count = roles_inner.len() + 2;
                                        view! {
                                            // Category header row — accent divider
                                            <tr>
                                                <td colspan=col_count class="roles-matrix__cat">
                                                    {cat_label}
                                                    <span class="roles-matrix__cat-count">
                                                        {format!("({} scope)", cat_scopes.len())}
                                                    </span>
                                                </td>
                                            </tr>
                                            // Scope rows
                                            {cat_scopes.into_iter().map(|scope| {
                                                let scope_id = scope.scope_id.clone();
                                                let label = scope.label.clone();
                                                let description = scope.description.clone();
                                                let read_label = scope.read_label.clone();
                                                let all_label = scope.all_label.clone();
                                                let roles_row = roles_inner.clone();

                                                view! {
                                                    <tr title=description.clone()>
                                                        <td class="data-matrix__rowhead roles-matrix__name">
                                                            {label.clone()}
                                                        </td>
                                                        <td class="data-matrix__rowhead roles-matrix__scope">
                                                            {scope_id.clone()}
                                                        </td>
                                                        // Role cells
                                                        {roles_row.iter().map(|role| {
                                                            let role_id = role.id.clone();
                                                            let scope_id2 = scope_id.clone();
                                                            let is_system = role.is_system;
                                                            let rl = read_label.clone();
                                                            let al = all_label.clone();

                                                            // Extra clones needed because role_id/scope_id2
                                                            // are moved into eff_mode but also used in style closure
                                                            let role_id_style = role_id.clone();
                                                            let scope_id2_style = scope_id2.clone();

                                                            let eff_mode = Signal::derive(move || {
                                                                let p = pending.get();
                                                                let key = (role_id.clone(), scope_id2.clone());
                                                                if let Some(m) = p.get(&key) {
                                                                    m.clone()
                                                                } else {
                                                                    let perms = permissions.get();
                                                                    get_mode(&perms, &role_id, &scope_id2)
                                                                }
                                                            });

                                                            let role_id_click = role.id.clone();
                                                            let scope_id_click = scope_id.clone();

                                                            let on_click = move |_| {
                                                                if is_system { return; }
                                                                let current = {
                                                                    let p = pending.get();
                                                                    let key = (role_id_click.clone(), scope_id_click.clone());
                                                                    if let Some(m) = p.get(&key) {
                                                                        m.clone()
                                                                    } else {
                                                                        let perms = permissions.get();
                                                                        get_mode(&perms, &role_id_click, &scope_id_click)
                                                                    }
                                                                };
                                                                let next = next_mode(&current).to_string();
                                                                pending.update(|p| {
                                                                    p.insert((role_id_click.clone(), scope_id_click.clone()), next);
                                                                });
                                                            };

                                                            view! {
                                                                <td
                                                                    class=move || {
                                                                        let mut c = cell_td_class(is_system).to_string();
                                                                        let has_pending_change = {
                                                                            let p = pending.get();
                                                                            p.contains_key(&(role_id_style.clone(), scope_id2_style.clone()))
                                                                        };
                                                                        if has_pending_change {
                                                                            c.push_str(" roles-matrix__cell--pending");
                                                                        }
                                                                        c
                                                                    }
                                                                    title=move || {
                                                                        let m = eff_mode.get();
                                                                        match m.as_str() {
                                                                            "read" => rl.clone(),
                                                                            "all"  => al.clone(),
                                                                            _      => if is_system { "Нет доступа".to_string() } else { "Нажмите для изменения".to_string() },
                                                                        }
                                                                    }
                                                                    on:click=on_click
                                                                >
                                                                    <span class=move || mode_pill_class(&eff_mode.get(), is_system)>
                                                                        {move || { let m = eff_mode.get(); mode_badge(&m) }}
                                                                    </span>
                                                                </td>
                                                            }
                                                        }).collect::<Vec<_>>()}
                                                    </tr>
                                                }
                                            }).collect::<Vec<_>>()}
                                        }
                                    }).collect::<Vec<_>>()}
                                </tbody>
                            </table>
                        </div>
                        <div class="roles-matrix__hint">
                            "Кликните по ячейке custom-роли для изменения: — → чтение → ВСЕ → —. Нажмите «Сохранить» для применения."
                        </div>
                    }.into_any()
                }}
            </div>
        </PageFrame>
    }
}
