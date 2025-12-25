use contracts::system::sys_scheduled_task::request::{CreateScheduledTaskDto, UpdateScheduledTaskDto};
use contracts::system::sys_scheduled_task::response::ScheduledTaskResponse;
use contracts::system::sys_scheduled_task::progress::TaskProgressResponse;
use crate::shared::icons::icon;
use crate::system::tasks::api;
use leptos::ev;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

#[component]
pub fn ScheduledTaskDetails(id: String) -> impl IntoView {
    let is_new = id == "new";
    let (_loading, set_loading) = signal(true);
    let (saving, set_saving) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (_task, set_task) = signal::<Option<ScheduledTaskResponse>>(None);

    // Form fields using RwSignal for better compatibility with Thaw
    let code = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let task_type = RwSignal::new(String::new());
    let schedule_cron = RwSignal::new(String::new());
    let is_enabled = RwSignal::new(true);
    let config_json = RwSignal::new(String::new());
    let comment = RwSignal::new(String::new());

    // Execution state
    let session_id = RwSignal::new(None::<String>);
    let progress = RwSignal::new(None::<TaskProgressResponse>);
    let log_content = RwSignal::new(String::new());

    let id_for_load = id.clone();
    let load_task = move || {
        if is_new {
            set_loading.set(false);
            return;
        }

        let task_id = id_for_load.clone();
        spawn_local(async move {
            set_loading.set(true);
            match api::get_scheduled_task(&task_id).await {
                Ok(t) => {
                    code.set(t.code.clone());
                    description.set(t.description.clone());
                    task_type.set(t.task_type.clone());
                    schedule_cron.set(t.schedule_cron.clone().unwrap_or_default());
                    is_enabled.set(t.is_enabled);
                    config_json.set(t.config_json.clone());
                    comment.set(t.comment.clone().unwrap_or_default());
                    set_task.set(Some(t));
                }
                Err(e) => {
                    set_error.set(Some(e));
                }
            }
            set_loading.set(false);
        });
    };

    // Load on mount
    Effect::new(move |_| load_task());

    let id_for_save = id.clone();
    let save_task = move |_| {
        set_saving.set(true);
        let task_id = id_for_save.clone();

        spawn_local(async move {
            let res = if is_new {
                let dto = CreateScheduledTaskDto {
                    code: code.get(),
                    description: description.get(),
                    task_type: task_type.get(),
                    schedule_cron: if schedule_cron.get().is_empty() { None } else { Some(schedule_cron.get()) },
                    is_enabled: is_enabled.get(),
                    config_json: config_json.get(),
                };
                api::create_scheduled_task(dto).await
            } else {
                let dto = UpdateScheduledTaskDto {
                    code: code.get(),
                    description: description.get(),
                    comment: if comment.get().is_empty() { None } else { Some(comment.get()) },
                    task_type: task_type.get(),
                    schedule_cron: if schedule_cron.get().is_empty() { None } else { Some(schedule_cron.get()) },
                    is_enabled: is_enabled.get(),
                    config_json: config_json.get(),
                };
                api::update_scheduled_task(&task_id, dto).await
            };

            match res {
                Ok(t) => {
                    set_task.set(Some(t));
                }
                Err(e) => {
                    set_error.set(Some(e));
                }
            }
            set_saving.set(false);
        });
    };

    let id_for_delete = id.clone();
    let delete_task = move |_| {
        if is_new { return; }
        let task_id = id_for_delete.clone();
        spawn_local(async move {
            match api::delete_scheduled_task(&task_id).await {
                Ok(_) => {
                    // Close tab or redirect
                }
                Err(e) => {
                    set_error.set(Some(e));
                }
            }
        });
    };

    // Poll for progress if session_id is set
    let id_for_effect = id.clone();
    Effect::new(move |_| {
        if let Some(sid) = session_id.get() {
            let task_id = id_for_effect.clone();
            spawn_local(async move {
                loop {
                    match api::get_task_progress(&task_id, &sid).await {
                        Ok(p) => {
                            let status = p.status.clone();
                            progress.set(Some(p.clone()));
                            if let Some(log) = p.log_content {
                                log_content.set(log);
                            }

                            if status == "Completed" || status == "Failed" {
                                break;
                            }
                        }
                        Err(e) => {
                            log!("Error polling progress: {}", e);
                            break;
                        }
                    }
                    gloo_timers::future::TimeoutFuture::new(2000).await;
                }
            });
        }
    });

    view! {
        <div class="scheduled-task-details" style="padding: 20px; max-width: 1200px; margin: 0 auto; color: var(--colorNeutralForeground1);">
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center style="margin-bottom: 24px; border-bottom: 1px solid var(--colorNeutralStroke1); padding-bottom: 16px;">
                <h2 style="margin: 0; color: var(--colorNeutralForeground1); font-size: 24px; font-weight: bold;">
                    {move || if is_new { "🆕 Новая задача".to_string() } else { format!("⚙️ Задача: {}", code.get()) }}
                </h2>
                <Space>
                    <Button 
                        appearance=ButtonAppearance::Primary
                        on_click=move |_| save_task(()) 
                        disabled=saving.get() 
                    >
                        {icon("save")}
                        " Сохранить"
                    </Button>
                    {move || if !is_new {
                        let delete_task_clone = delete_task.clone();
                        view! { 
                            <Button 
                                appearance=ButtonAppearance::Transparent
                                on_click=move |_: ev::MouseEvent| delete_task_clone(()) 
                                attr:style="color: var(--colorPaletteRedForeground1);"
                            >
                                {icon("delete")}
                                " Удалить"
                            </Button> 
                        }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }}
                </Space>
            </Flex>

            {move || error.get().map(|err| view! {
                <div style="padding: 12px; background: var(--color-error-50); border: 1px solid var(--color-error-100); border-radius: 8px; color: var(--color-error); margin-bottom: 24px; display: flex; align-items: center; gap: 8px;">
                    <span style="font-size: 18px;">"⚠"</span>
                    <span>{err}</span>
                </div>
            })}

            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 24px;">
                // Left Column: Basic Info
                <div style="background: var(--colorNeutralBackground1); padding: 20px; border-radius: 12px; border: 1px solid var(--colorNeutralStroke1); box-shadow: var(--shadow2);">
                    <h3 style="margin-top: 0; margin-bottom: 20px; font-size: 1.1rem; color: var(--colorNeutralForeground2); font-weight: 600;">"Основные параметры"</h3>
                    
                    <Flex vertical=true gap=FlexGap::Medium>
                        <div class="form-group" style="display: flex; flex-direction: column; gap: 6px;">
                            <label style="font-size: 0.875rem; font-weight: 600; color: var(--colorNeutralForeground3);">"Код задачи"</label>
                            <Input value=code placeholder="Напр: u501_import_ut" />
                        </div>

                        <div class="form-group" style="display: flex; flex-direction: column; gap: 6px;">
                            <label style="font-size: 0.875rem; font-weight: 600; color: var(--colorNeutralForeground3);">"Описание"</label>
                            <Input value=description placeholder="Напр: Импорт организаций из УТ" />
                        </div>

                        <div class="form-group" style="display: flex; flex-direction: column; gap: 6px;">
                            <label style="font-size: 0.875rem; font-weight: 600; color: var(--colorNeutralForeground3);">"Тип обработчика"</label>
                            <Select value=task_type>
                                <option value="">"— Выберите тип —"</option>
                                <option value="u501_import_ut">"u501: Импорт из 1С:УТ"</option>
                                <option value="u502_import_ozon">"u502: Импорт из OZON"</option>
                                <option value="u503_import_yandex">"u503: Импорт из Yandex Market"</option>
                                <option value="u504_import_wildberries">"u504: Импорт из Wildberries"</option>
                            </Select>
                        </div>

                        <div class="form-group" style="display: flex; flex-direction: column; gap: 6px;">
                            <label style="font-size: 0.875rem; font-weight: 600; color: var(--colorNeutralForeground3);">"Расписание (Cron)"</label>
                            <Input value=schedule_cron placeholder="0 0 * * * (каждую полночь)" />
                        </div>

                        <div style="padding: 10px 0;">
                            <Checkbox checked=is_enabled label="Задача включена" />
                        </div>

                        <div class="form-group" style="display: flex; flex-direction: column; gap: 6px;">
                            <label style="font-size: 0.875rem; font-weight: 600; color: var(--colorNeutralForeground3);">"Комментарий"</label>
                            <Textarea value=comment placeholder="..." attr:rows=3 />
                        </div>
                    </Flex>
                </div>

                // Right Column: Config JSON
                <div style="background: var(--colorNeutralBackground1); padding: 20px; border-radius: 12px; border: 1px solid var(--colorNeutralStroke1); box-shadow: var(--shadow2);">
                    <h3 style="margin-top: 0; margin-bottom: 20px; font-size: 1.1rem; color: var(--colorNeutralForeground2); font-weight: 600;">"Конфигурация (JSON)"</h3>
                    <div style="height: calc(100% - 40px);">
                        <Textarea 
                            value=config_json
                            placeholder="{ ... }" 
                            class="monospace-textarea"
                            attr:rows=15
                            attr:style="font-family: var(--fontFamilyMonospace); background: var(--colorNeutralBackground2);"
                        />
                    </div>
                </div>
            </div>

            // Execution & Logs section
            {move || if !is_new {
                view! {
                    <div style="margin-top: 24px; background: var(--colorNeutralBackground1); padding: 20px; border-radius: 12px; border: 1px solid var(--colorNeutralStroke1); box-shadow: var(--shadow2);">
                        <h3 style="margin-top: 0; margin-bottom: 20px; font-size: 1.1rem; color: var(--colorNeutralForeground2); font-weight: 600;">"Логи и прогресс"</h3>
                        
                        <Flex vertical=true gap=FlexGap::Medium>
                            <div style="background: var(--colorNeutralBackgroundDarkStatic); color: var(--colorNeutralForegroundInvertedStatic); padding: 16px; border-radius: 8px; font-family: var(--fontFamilyMonospace); font-size: 0.85rem; height: 300px; overflow-y: auto; white-space: pre-wrap; border: 1px solid var(--colorNeutralStroke1);">
                                {move || if log_content.get().is_empty() { "Логи пока пусты...".to_string() } else { log_content.get() }}
                            </div>

                            {move || progress.get().map(|p| view! {
                                <div style="display: flex; flex-direction: column; gap: 12px; background: var(--colorNeutralBackground2); padding: 16px; border-radius: 8px; border: 1px solid var(--colorNeutralStroke1);">
                                    <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
                                        <span style="font-weight: 600; font-size: 0.9rem;">"Прогресс выполнения"</span>
                                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>{p.status}</Badge>
                                    </Flex>
                                    
                                    <div style="width: 100%; height: 10px; background: var(--colorNeutralBackground3); border-radius: 5px; overflow: hidden;">
                                        <div style=move || {
                                            let percent = if let (Some(t), Some(pr)) = (p.total_items, p.processed_items) {
                                                if t > 0 { (pr as f64 / t as f64 * 100.0) as i32 } else { 0 }
                                            } else { 0 };
                                            format!("width: {}%; height: 100%; background: var(--colorBrandBackground); transition: width 0.3s ease;", percent)
                                        }></div>
                                    </div>
                                    
                                    <Flex justify=FlexJustify::SpaceBetween style="font-size: 0.85rem; color: var(--colorNeutralForeground3);">
                                        <span>{format!("Обработано: {} / {}", p.processed_items.unwrap_or(0), p.total_items.unwrap_or(0))}</span>
                                        {p.current_item.map(|item| view! {
                                            <span style="font-style: italic;">"Текущий объект: " {item}</span>
                                        })}
                                    </Flex>
                                </div>
                            })}
                        </Flex>
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }}
        </div>
    }
}
