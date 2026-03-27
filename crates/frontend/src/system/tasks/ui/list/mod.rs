pub mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use crate::system::tasks::api;
use leptos::ev;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

#[component]
pub fn ScheduledTaskList() -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");

    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);

    let load_tasks = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            match api::fetch_scheduled_tasks().await {
                Ok(tasks) => {
                    state.update(|s| {
                        s.tasks = tasks;
                        s.is_loaded = true;
                    });
                    set_loading.set(false);
                }
                Err(e) => {
                    log!("Failed to fetch scheduled tasks: {}", e);
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    };

    // Load on mount
    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            load_tasks();
        }
    });

    let toggle_enabled = move |id: String, current_status: bool| {
        spawn_local(async move {
            match api::toggle_scheduled_task_enabled(&id, !current_status).await {
                Ok(_) => {
                    load_tasks();
                }
                Err(e) => {
                    log!("Failed to toggle task: {}", e);
                }
            }
        });
    };

    let open_details = move |id: String, code: String| {
        tabs_store.open_tab(
            &format!("sys_scheduled_task_details_{}", id),
            &format!("Задача: {}", code),
        );
    };

    let create_new = move |_| {
        tabs_store.open_tab("sys_scheduled_task_details", "Новая задача");
    };

    view! {
        <div id="sys_scheduled_tasks--list" data-page-category="legacy" class="scheduled-task-list" style="padding: 20px;">
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center style="margin-bottom: 16px;">
                <h2 style="margin: 0; font-size: 24px; font-weight: bold;">"📅 Регламентные задания"</h2>
                <Space>
                    <Button appearance=ButtonAppearance::Primary on_click=create_new>
                        {icon("plus")}
                        " Создать"
                    </Button>
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| load_tasks() disabled=loading>
                        {icon("refresh")}
                        " Обновить"
                    </Button>
                </Space>
            </Flex>

            {move || error.get().map(|err| view! {
                <div style="padding: 12px; background: var(--color-error-50); border: 1px solid var(--color-error-100); border-radius: 8px; display: flex; align-items: center; gap: 8px; margin-bottom: 16px;">
                    <span style="color: var(--color-error); font-size: 18px;">"⚠"</span>
                    <span style="color: var(--color-error);">{err}</span>
                </div>
            })}

            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHeaderCell attr:style="width: 150px;">"Код"</TableHeaderCell>
                        <TableHeaderCell>"Описание"</TableHeaderCell>
                        <TableHeaderCell attr:style="width: 150px;">"Тип"</TableHeaderCell>
                        <TableHeaderCell attr:style="width: 120px;">"Расписание"</TableHeaderCell>
                        <TableHeaderCell attr:style="width: 140px;">"Последний запуск"</TableHeaderCell>
                        <TableHeaderCell attr:style="width: 100px;">"Статус"</TableHeaderCell>
                        <TableHeaderCell attr:style="width: 80px; text-align: center;">"Вкл"</TableHeaderCell>
                        <TableHeaderCell attr:style="width: 80px; text-align: center;">"Действия"</TableHeaderCell>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {move || {
                        if loading.get() {
                            view! {
                                <TableRow>
                                    <TableCell attr:colspan="8" attr:style="padding: 40px; text-align: center;">
                                        <Flex justify=FlexJustify::Center align=FlexAlign::Center gap=FlexGap::Small>
                                            <Spinner />
                                            "Загрузка..."
                                        </Flex>
                                    </TableCell>
                                </TableRow>
                            }.into_any()
                        } else {
                            let tasks = state.get().tasks;
                            if tasks.is_empty() {
                                view! {
                                    <TableRow>
                                        <TableCell attr:colspan="8" attr:style="padding: 40px; text-align: center; color: var(--colorNeutralForeground3);">
                                            "Заданий не найдено"
                                        </TableCell>
                                    </TableRow>
                                }.into_any()
                            } else {
                                tasks.into_iter().map(|task: contracts::system::tasks::response::ScheduledTaskResponse| {
                                    let id = task.id.clone();
                                    let code = task.code.clone();
                                    let id_for_toggle = task.id.clone();
                                    let is_enabled = task.is_enabled;
                                    let id_for_details = task.id.clone();
                                    let code_for_details = task.code.clone();

                                    let status_view = match task.last_run_status.as_deref() {
                                        Some("Completed") => view! { <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Success>"Успешно"</Badge> }.into_any(),
                                        Some("Running") => view! { <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>"Запуск"</Badge> }.into_any(),
                                        Some("Failed") => view! { <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Danger>"Ошибка"</Badge> }.into_any(),
                                        _ => view! { <Badge appearance=BadgeAppearance::Tint>"—"</Badge> }.into_any(),
                                    };

                                    view! {
                                        <TableRow on:dblclick=move |_| open_details(id_for_details.clone(), code_for_details.clone()) attr:style="cursor: pointer;">
                                            <TableCell>{task.code}</TableCell>
                                            <TableCell>{task.description}</TableCell>
                                            <TableCell>
                                                <code style="background: var(--colorNeutralBackground3); padding: 2px 4px; border-radius: 4px; font-size: 0.85em;">
                                                    {task.task_type}
                                                </code>
                                            </TableCell>
                                            <TableCell>{task.schedule_cron.unwrap_or_else(|| "—".to_string())}</TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    <span style="font-size: 0.9em; color: var(--colorNeutralForeground2);">
                                                        {task.last_run_at.map(|d| d.format("%d.%m %H:%M").to_string()).unwrap_or_else(|| "—".to_string())}
                                                    </span>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                {status_view}
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    <div style="text-align: center;">
                                                <div
                                                    on:click=move |_| toggle_enabled(id_for_toggle.clone(), is_enabled)
                                                    style="cursor: pointer; display: inline-block;"
                                                >
                                                    <Checkbox checked=is_enabled attr:disabled=true />
                                                </div>
                                                    </div>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    <div style="text-align: center;">
                                                <Button
                                                    appearance=ButtonAppearance::Transparent
                                                    on_click=move |e: ev::MouseEvent| {
                                                        e.stop_propagation();
                                                        open_details(id.clone(), code.clone());
                                                    }
                                                >
                                                    {icon("settings")}
                                                </Button>
                                                    </div>
                                                </TableCellLayout>
                                            </TableCell>
                                        </TableRow>
                                    }
                                }).collect_view().into_any()
                            }
                        }
                    }}
                </TableBody>
            </Table>
        </div>
    }
}
