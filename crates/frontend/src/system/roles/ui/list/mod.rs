use contracts::system::roles::{CreateRoleDto, Role, UpdateRoleDto};
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_SYSTEM;
use crate::system::auth::guard::RequireAdmin;
use crate::system::roles::api;

#[component]
pub fn RolesListPage() -> impl IntoView {
    view! {
        <RequireAdmin>
            <RolesList />
        </RequireAdmin>
    }
}

#[derive(Clone, Default)]
struct RolesListState {
    items: Vec<Role>,
    all_items: Vec<Role>,
    total_count: usize,
    page: usize,
    page_size: usize,
    total_pages: usize,
    search_query: String,
    is_loaded: bool,
}

fn recalc(state: &mut RolesListState) {
    let query = state.search_query.to_lowercase();
    let mut data = state.all_items.clone();
    if !query.is_empty() {
        data.retain(|r| {
            r.code.to_lowercase().contains(&query) || r.name.to_lowercase().contains(&query)
        });
    }
    state.total_count = data.len();
    let total_pages = if state.total_count == 0 {
        1
    } else {
        (state.total_count + state.page_size - 1) / state.page_size
    };
    state.total_pages = total_pages;
    if state.page >= total_pages {
        state.page = total_pages.saturating_sub(1);
    }
    let start = state.page * state.page_size;
    let end = (start + state.page_size).min(data.len());
    state.items = data.get(start..end).unwrap_or(&[]).to_vec();
}

#[component]
fn RolesList() -> impl IntoView {
    let state = RwSignal::new(RolesListState {
        page_size: 25,
        ..Default::default()
    });
    let (error, set_error) = signal::<Option<String>>(None);
    let (loading, set_loading) = signal(false);
    let (show_create_form, set_show_create_form) = signal(false);
    let editing_role: RwSignal<Option<Role>> = RwSignal::new(None);

    let load_data = move || {
        set_loading.set(true);
        set_error.set(None);
        spawn_local(async move {
            match api::fetch_roles().await {
                Ok(data) => {
                    state.update(|s| {
                        s.all_items = data;
                        s.page = 0;
                        s.is_loaded = true;
                        recalc(s);
                    });
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(format!("Не удалось загрузить роли: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            load_data();
        }
    });

    let search_signal = RwSignal::new(String::new());

    let apply_search = move || {
        state.update(|s| {
            s.search_query = search_signal.get_untracked();
            s.page = 0;
            recalc(s);
        });
    };

    let go_to_page = move |page: usize| {
        state.update(|s| {
            s.page = page;
            recalc(s);
        });
    };

    let change_page_size = move |size: usize| {
        state.update(|s| {
            s.page_size = size;
            s.page = 0;
            recalc(s);
        });
    };

    let global_ctx = use_context::<AppGlobalContext>();

    let open_role_details = move |role_id: String, role_name: String| {
        let tab_key = format!("sys_role_details_{}", role_id);
        if let Some(ctx) = global_ctx {
            ctx.open_tab(&tab_key, &role_name);
        }
    };

    view! {
        <PageFrame page_id="sys_roles--list" category=PAGE_CAT_SYSTEM>
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Роли"</h1>
                    <Badge>
                        {move || state.get().total_count.to_string()}
                    </Badge>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=move |_| set_show_create_form.set(true)
                    >
                        {icon("plus")}
                        " Новая роль"
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| load_data()
                        disabled=Signal::derive(move || loading.get())
                    >
                        {icon("refresh")}
                        {move || if loading.get() { " Загрузка..." } else { " Обновить" }}
                    </Button>
                </div>
            </div>

            <div class="page__content">
                {move || error.get().map(|e| view! { <div class="alert alert--error">{e}</div> })}

                <div class="filter-panel">
                    <div class="filter-panel-header">
                        <div class="filter-panel-header__left">
                            {icon("filter")}
                            <span class="filter-panel__title">"Поиск"</span>
                        </div>
                        <div class="filter-panel-header__center">
                            <PaginationControls
                                current_page=Signal::derive(move || state.get().page)
                                total_pages=Signal::derive(move || state.get().total_pages)
                                total_count=Signal::derive(move || state.get().total_count)
                                page_size=Signal::derive(move || state.get().page_size)
                                on_page_change=Callback::new(go_to_page)
                                on_page_size_change=Callback::new(change_page_size)
                                page_size_options=vec![25, 50, 100]
                            />
                        </div>
                        <div class="filter-panel-header__right"></div>
                    </div>
                    <div class="filter-panel-content">
                        <Flex gap=FlexGap::Small align=FlexAlign::End>
                            <div style="flex: 1; max-width: 320px;">
                                <Input
                                    value=search_signal
                                    placeholder="Код или название роли..."
                                />
                            </div>
                            <Button
                                appearance=ButtonAppearance::Primary
                                on_click=move |_| apply_search()
                                disabled=Signal::derive(move || loading.get())
                            >
                                "Найти"
                            </Button>
                            <Button
                                appearance=ButtonAppearance::Secondary
                                on_click=move |_| {
                                    search_signal.set(String::new());
                                    state.update(|s| { s.search_query = String::new(); s.page = 0; recalc(s); });
                                }
                            >
                                "Сбросить"
                            </Button>
                        </Flex>
                    </div>
                </div>

                <div class="table-wrapper">
                    <Table attr:style="width: 100%;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell min_width=120.0>"Код"</TableHeaderCell>
                                <TableHeaderCell min_width=180.0>"Название"</TableHeaderCell>
                                <TableHeaderCell min_width=100.0>"Тип"</TableHeaderCell>
                                <TableHeaderCell min_width=120.0></TableHeaderCell>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            <For
                                each=move || state.get().items
                                key=|r| format!("{}:{}", r.id, r.name)
                                children=move |role| {
                                    let role_id_for_details = role.id.clone();
                                    let role_name_for_details = role.name.clone();
                                    let role_id_for_delete = role.id.clone();
                                    let role_for_edit = role.clone();
                                    let is_system = role.is_system;
                                    view! {
                                        <TableRow>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    <span style="font-weight: 500; font-family: monospace;">{role.code.clone()}</span>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {role.name.clone()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {if role.is_system {
                                                        view! { <span class="badge badge--info">"Системная"</span> }.into_any()
                                                    } else {
                                                        view! { <span class="badge badge--neutral">"Пользовательская"</span> }.into_any()
                                                    }}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <Flex gap=FlexGap::Small>
                                                    <Button
                                                        appearance=ButtonAppearance::Subtle
                                                        on_click=move |_| open_role_details(role_id_for_details.clone(), role_name_for_details.clone())
                                                        attr:title="Права"
                                                    >
                                                        {icon("shield")}
                                                    </Button>
                                                    {if !is_system {
                                                        view! {
                                                            <>
                                                                <Button
                                                                    appearance=ButtonAppearance::Subtle
                                                                    on_click=move |_| editing_role.set(Some(role_for_edit.clone()))
                                                                    attr:title="Редактировать"
                                                                >
                                                                    {icon("edit")}
                                                                </Button>
                                                                <DeleteRoleButton
                                                                    role_id=role_id_for_delete.clone()
                                                                    on_deleted=move || load_data()
                                                                />
                                                            </>
                                                        }.into_any()
                                                    } else {
                                                        view! { <></> }.into_any()
                                                    }}
                                                </Flex>
                                            </TableCell>
                                        </TableRow>
                                    }
                                }
                            />
                        </TableBody>
                    </Table>
                </div>

                {move || if show_create_form.get() {
                    view! {
                        <CreateRoleForm
                            on_close=move || set_show_create_form.set(false)
                            on_created=move || {
                                set_show_create_form.set(false);
                                load_data();
                            }
                        />
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }}

                {move || editing_role.get().map(|role| view! {
                    <EditRoleForm
                        role=role
                        on_close=move || editing_role.set(None)
                        on_saved=move || { editing_role.set(None); load_data(); }
                    />
                })}
            </div>
        </PageFrame>
    }
}

#[component]
fn DeleteRoleButton<F>(role_id: String, on_deleted: F) -> impl IntoView
where
    F: Fn() + 'static + Copy + Send + Sync,
{
    let (deleting, set_deleting) = signal(false);

    let on_click = move |_| {
        let id = role_id.clone();
        set_deleting.set(true);
        spawn_local(async move {
            let _ = api::delete_role(&id).await;
            set_deleting.set(false);
            on_deleted();
        });
    };

    view! {
        <Button
            appearance=ButtonAppearance::Subtle
            on_click=on_click
            disabled=Signal::derive(move || deleting.get())
            attr:title="Удалить"
        >
            {icon("trash")}
        </Button>
    }
}

#[component]
fn CreateRoleForm<F1, F2>(on_close: F1, on_created: F2) -> impl IntoView
where
    F1: Fn() + 'static + Copy + Send + Sync,
    F2: Fn() + 'static + Copy + Send + Sync,
{
    let code = RwSignal::new(String::new());
    let name = RwSignal::new(String::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (saving, set_saving) = signal(false);

    let on_save = move |_| {
        if code.get().trim().is_empty() {
            set_error.set(Some("Код обязателен".to_string()));
            return;
        }
        if name.get().trim().is_empty() {
            set_error.set(Some("Название обязательно".to_string()));
            return;
        }

        let dto = CreateRoleDto {
            code: code.get(),
            name: name.get(),
        };

        set_saving.set(true);
        set_error.set(None);

        spawn_local(async move {
            match api::create_role(dto).await {
                Ok(_) => on_created(),
                Err(e) => {
                    set_error.set(Some(format!("Ошибка создания: {}", e)));
                    set_saving.set(false);
                }
            }
        });
    };

    view! {
        <div class="modal-overlay" on:click=move |_| on_close()>
            <div class="modal" on:click=move |ev| ev.stop_propagation()>
                <div class="modal-header">
                    <h2 class="modal-title">"Новая роль"</h2>
                    <Button appearance=ButtonAppearance::Subtle on_click=move |_| on_close()>
                        {icon("x")}
                    </Button>
                </div>
                <div class="modal-body">
                    {move || error.get().map(|e| view! { <div class="alert alert--error">{e}</div> })}
                    <div class="form__group">
                        <Label>"Код роли"</Label>
                        <Input value=code placeholder="Напр: external_analyst" disabled=Signal::derive(move || saving.get()) />
                    </div>
                    <div class="form__group">
                        <Label>"Название"</Label>
                        <Input value=name placeholder="Напр: Внешний аналитик" disabled=Signal::derive(move || saving.get()) />
                    </div>
                </div>
                <div class="modal-footer">
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| on_close() disabled=Signal::derive(move || saving.get())>
                        "Отмена"
                    </Button>
                    <Button appearance=ButtonAppearance::Primary on_click=on_save disabled=Signal::derive(move || saving.get())>
                        {move || if saving.get() { "Сохранение..." } else { "Создать" }}
                    </Button>
                </div>
            </div>
        </div>
    }
}

#[component]
fn EditRoleForm<F1, F2>(role: Role, on_close: F1, on_saved: F2) -> impl IntoView
where
    F1: Fn() + 'static + Copy + Send + Sync,
    F2: Fn() + 'static + Copy + Send + Sync,
{
    let name = RwSignal::new(role.name.clone());
    let (error, set_error) = signal::<Option<String>>(None);
    let (saving, set_saving) = signal(false);

    let role_code_display = role.code.clone();

    let on_save = move |_| {
        if name.get().trim().is_empty() {
            set_error.set(Some("Название обязательно".to_string()));
            return;
        }

        let dto = UpdateRoleDto {
            id: role.id.clone(),
            name: name.get(),
        };

        set_saving.set(true);
        set_error.set(None);

        spawn_local(async move {
            match api::update_role(dto).await {
                Ok(_) => on_saved(),
                Err(e) => {
                    set_error.set(Some(format!("Ошибка сохранения: {}", e)));
                    set_saving.set(false);
                }
            }
        });
    };

    view! {
        <div class="modal-overlay" on:click=move |_| on_close()>
            <div class="modal" on:click=move |ev| ev.stop_propagation()>
                <div class="modal-header">
                    <h2 class="modal-title">{format!("Редактирование: {}", role_code_display)}</h2>
                    <Button appearance=ButtonAppearance::Subtle on_click=move |_| on_close()>
                        {icon("x")}
                    </Button>
                </div>
                <div class="modal-body">
                    {move || error.get().map(|e| view! { <div class="alert alert--error">{e}</div> })}
                    <div class="form__group">
                        <Label>"Название"</Label>
                        <Input value=name disabled=Signal::derive(move || saving.get()) />
                    </div>
                </div>
                <div class="modal-footer">
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| on_close() disabled=Signal::derive(move || saving.get())>
                        "Отмена"
                    </Button>
                    <Button appearance=ButtonAppearance::Primary on_click=on_save disabled=Signal::derive(move || saving.get())>
                        {move || if saving.get() { "Сохранение..." } else { "Сохранить" }}
                    </Button>
                </div>
            </div>
        </div>
    }
}
