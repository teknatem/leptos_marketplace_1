use contracts::system::roles::RoleScopeAccess;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_SYSTEM;
use crate::system::auth::guard::RequireAdmin;
use crate::system::roles::api;

#[component]
pub fn RoleDetailsPage(role_id: String) -> impl IntoView {
    let stored_id = StoredValue::new(role_id);
    view! {
        <RequireAdmin>
            <RoleDetails role_id=stored_id.get_value() />
        </RequireAdmin>
    }
}

fn access_mode_badge(mode: &str) -> (&'static str, &'static str) {
    match mode {
        "all" => ("all", "badge badge--primary"),
        "read" => ("read", "badge badge--info"),
        _ => ("-", "badge badge--neutral"),
    }
}

#[component]
fn RoleDetails(role_id: String) -> impl IntoView {
    let permissions: RwSignal<Vec<RoleScopeAccess>> = RwSignal::new(Vec::new());
    let role_name: RwSignal<String> = RwSignal::new(String::new());
    let is_system = RwSignal::new(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (loading, set_loading) = signal(true);

    let stored_id = StoredValue::new(role_id);

    let load_data = move || {
        let id = stored_id.get_value();
        set_loading.set(true);
        set_error.set(None);
        spawn_local(async move {
            let perms_result = api::fetch_role_permissions(&id).await;
            let roles_result = api::fetch_roles().await;

            match (perms_result, roles_result) {
                (Ok(perms), Ok(roles)) => {
                    permissions.set(perms);
                    if let Some(role) = roles.iter().find(|r| r.id == id) {
                        role_name.set(role.name.clone());
                        is_system.set(role.is_system);
                    }
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
        <PageFrame page_id="sys_role_details" category=PAGE_CAT_SYSTEM>
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">
                        {move || format!("Роль: {}", role_name.get())}
                    </h1>
                    {move || if is_system.get() {
                        view! { <span class="badge badge--info">"Системная"</span> }.into_any()
                    } else {
                        view! { <span class="badge badge--neutral">"Пользовательская"</span> }.into_any()
                    }}
                    <Badge>
                        {move || format!("{} прав", permissions.get().len())}
                    </Badge>
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
                    view! { <div class="page__loading">"Загрузка..."</div> }.into_any()
                } else {
                    view! {
                        <div class="table-wrapper">
                            <Table attr:style="width: 100%;">
                                <TableHeader>
                                    <TableRow>
                                        <TableHeaderCell min_width=220.0>"Scope ID"</TableHeaderCell>
                                        <TableHeaderCell min_width=80.0>"Доступ"</TableHeaderCell>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    <For
                                        each=move || permissions.get()
                                        key=|p| p.scope_id.clone()
                                        children=move |perm| {
                                            let (label, class) = access_mode_badge(&perm.access_mode);
                                            view! {
                                                <TableRow>
                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            <span style="font-family: monospace; font-size: 0.85em;">
                                                                {perm.scope_id.clone()}
                                                            </span>
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCell>
                                                        <TableCellLayout>
                                                            <span class=class>{label}</span>
                                                        </TableCellLayout>
                                                    </TableCell>
                                                </TableRow>
                                            }
                                        }
                                    />
                                </TableBody>
                            </Table>
                        </div>
                    }.into_any()
                }}
            </div>
        </PageFrame>
    }
}
