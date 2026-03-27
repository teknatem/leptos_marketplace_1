use contracts::system::roles::{Role, RoleScopeAccess, ScopeInfo};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;
use thaw::*;

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

fn get_mode(map: &PermissionsMap, role_id: &str, scope_id: &str) -> &'static str {
    map.get(role_id)
        .and_then(|perms| perms.iter().find(|p| p.scope_id == scope_id))
        .map(|p| match p.access_mode.as_str() {
            "all" => "all",
            "read" => "read",
            _ => "-",
        })
        .unwrap_or("-")
}

fn mode_cell_style(mode: &str) -> &'static str {
    match mode {
        "all"  => "background: var(--color-primary-100, #dbeafe); color: var(--color-primary-700, #1d4ed8); font-weight: 600;",
        "read" => "background: var(--color-info-100, #e0f2fe); color: var(--color-info-700, #0369a1);",
        _      => "color: var(--color-text-disabled, #9ca3af);",
    }
}

#[component]
fn RoleMatrix() -> impl IntoView {
    let roles: RwSignal<Vec<Role>> = RwSignal::new(Vec::new());
    let scopes: RwSignal<Vec<ScopeInfo>> = RwSignal::new(Vec::new());
    let permissions: RwSignal<PermissionsMap> = RwSignal::new(HashMap::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (loading, set_loading) = signal(true);

    let load_data = move || {
        set_loading.set(true);
        set_error.set(None);

        spawn_local(async move {
            let roles_res = api::fetch_roles().await;
            let scopes_res = api::fetch_scopes().await;

            match (roles_res, scopes_res) {
                (Ok(roles_data), Ok(scopes_data)) => {
                    // Load permissions for all roles in parallel (sequential for now)
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
                    roles.set(roles_data);
                    scopes.set(scopes_data);
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

    view! {
        <PageFrame page_id="sys_roles_matrix" category=PAGE_CAT_SYSTEM>
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Матрица ролей"</h1>
                    {move || {
                        let r = roles.get().len();
                        let s = scopes.get().len();
                        view! {
                            <Badge>{format!("{} ролей × {} scope", r, s)}</Badge>
                        }
                    }}
                </div>
                <div class="page__header-right">
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

                {move || if loading.get() {
                    view! { <div class="page__loading">"Загрузка матрицы..."</div> }.into_any()
                } else {
                    let roles_snapshot = roles.get();
                    let scopes_snapshot = scopes.get();
                    let perm_snapshot = permissions.get();

                    view! {
                        <div class="table-wrapper" style="overflow-x: auto;">
                            <table class="roles-matrix" style="border-collapse: collapse; font-size: 0.82em; min-width: max-content;">
                                <thead>
                                    <tr>
                                        <th style="text-align: left; padding: 6px 10px; border-bottom: 2px solid var(--color-border); min-width: 200px; position: sticky; left: 0; background: var(--color-surface);">
                                            "Scope ID"
                                        </th>
                                        {roles_snapshot.iter().map(|role| {
                                            let is_system = role.is_system;
                                            let name = role.name.clone();
                                            let code = role.code.clone();
                                            view! {
                                                <th style="padding: 6px 8px; border-bottom: 2px solid var(--color-border); text-align: center; min-width: 90px;">
                                                    <div style="font-weight: 600;">{name}</div>
                                                    <div style="font-size: 0.85em; font-family: monospace; color: var(--color-text-secondary);">{code}</div>
                                                    {if is_system {
                                                        view! { <span class="badge badge--info" style="font-size: 0.7em;">"sys"</span> }.into_any()
                                                    } else {
                                                        view! { <></> }.into_any()
                                                    }}
                                                </th>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </tr>
                                </thead>
                                <tbody>
                                    {scopes_snapshot.iter().enumerate().map(|(i, scope)| {
                                        let row_bg = if i % 2 == 0 { "background: var(--color-surface);" } else { "background: var(--color-surface-alt, var(--color-surface));" };
                                        let scope_id = scope.scope_id.clone();
                                        view! {
                                            <tr style=row_bg>
                                                <td style=format!("padding: 4px 10px; border-bottom: 1px solid var(--color-border); font-family: monospace; position: sticky; left: 0; {}", row_bg)>
                                                    {scope_id.clone()}
                                                </td>
                                                {roles_snapshot.iter().map(|role| {
                                                    let mode = get_mode(&perm_snapshot, &role.id, &scope_id);
                                                    let cell_style = mode_cell_style(mode);
                                                    view! {
                                                        <td style=format!("padding: 4px 8px; border-bottom: 1px solid var(--color-border); text-align: center; {}", cell_style)>
                                                            {mode}
                                                        </td>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </tr>
                                        }
                                    }).collect::<Vec<_>>()}
                                </tbody>
                            </table>
                        </div>
                    }.into_any()
                }}
            </div>
        </PageFrame>
    }
}
