use crate::shared::icons::icon;
use crate::system::tasks::api;
use contracts::system::tasks::metadata::{TaskConfigFieldTypeDto, TaskMetadataDto};
use leptos::prelude::*;
use leptos::task::spawn_local;

// ============================================================================
// Helpers
// ============================================================================

fn field_type_label(t: &TaskConfigFieldTypeDto) -> &'static str {
    match t {
        TaskConfigFieldTypeDto::ConnectionMp => "Кабинет МП",
        TaskConfigFieldTypeDto::Integer => "Число",
        TaskConfigFieldTypeDto::Text => "Текст",
        TaskConfigFieldTypeDto::Date => "Дата",
    }
}

fn field_type_badge_style(t: &TaskConfigFieldTypeDto) -> &'static str {
    match t {
        TaskConfigFieldTypeDto::ConnectionMp => {
            "display:inline-block;padding:1px 7px;border-radius:10px;\
             background:var(--colorPaletteBlueBorder2,#0f6cbd22);\
             color:var(--colorBrandForeground1);font-size:11px;font-weight:600;"
        }
        TaskConfigFieldTypeDto::Integer => {
            "display:inline-block;padding:1px 7px;border-radius:10px;\
             background:var(--colorPaletteGreenBackground2,#10750022);\
             color:var(--colorPaletteGreenForeground1);font-size:11px;font-weight:600;"
        }
        TaskConfigFieldTypeDto::Text => {
            "display:inline-block;padding:1px 7px;border-radius:10px;\
             background:var(--colorNeutralBackground3);\
             color:var(--color-text-secondary);font-size:11px;font-weight:600;"
        }
        TaskConfigFieldTypeDto::Date => {
            "display:inline-block;padding:1px 7px;border-radius:10px;\
             background:var(--colorPalettePurpleBackground2,#7b449b22);\
             color:var(--colorPalettePurpleForeground2,#7b449b);font-size:11px;font-weight:600;"
        }
    }
}

// ============================================================================
// Card per task type
// ============================================================================

#[component]
fn TaskTypeCard(meta: TaskMetadataDto) -> impl IntoView {
    let (expanded, set_expanded) = signal(false);

    let task_type = meta.task_type.clone();
    let disp_name = meta.display_name.clone();
    let description = meta.description.clone();
    let apis = meta.external_apis.clone();
    let constraints = meta.constraints.clone();
    let fields = meta.config_fields.clone();

    let has_apis = !apis.is_empty();
    let has_constraints = !constraints.is_empty();
    let has_fields = !fields.is_empty();

    view! {
        <div style="border:1px solid var(--color-border);border-radius:var(--radius-md,8px);\
                    background:var(--colorNeutralBackground1);overflow:hidden;">
            // ---- header (always visible, clickable) ----
            <div
                style="display:flex;align-items:center;gap:12px;padding:14px 18px;\
                       cursor:pointer;user-select:none;\
                       border-bottom:1px solid transparent;\
                       transition:background 0.15s;"
                on:click=move |_| set_expanded.update(|v| *v = !*v)
            >
                <span style="display:flex;align-items:center;color:var(--color-text-secondary);
                             transition:transform 0.2s;"
                      style:transform=move || if expanded.get() { "rotate(90deg)" } else { "rotate(0deg)" }>
                    {icon("chevron-right")}
                </span>
                <div style="flex:1;min-width:0;">
                    <div style="display:flex;align-items:center;gap:10px;flex-wrap:wrap;">
                        <span style="font-weight:600;font-size:14px;color:var(--color-text);">
                            {disp_name}
                        </span>
                        <code style="font-size:11px;padding:1px 8px;border-radius:10px;\
                                     background:var(--colorNeutralBackground3);\
                                     color:var(--color-text-secondary);font-family:monospace;">
                            {task_type}
                        </code>
                    </div>
                    <div style="font-size:12px;color:var(--color-text-secondary);margin-top:3px;\
                                white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">
                        {description.clone()}
                    </div>
                </div>
                // badges summary
                <div style="display:flex;gap:6px;flex-shrink:0;">
                    {if has_fields {
                        let n = fields.len();
                        view! {
                            <span style="font-size:11px;padding:2px 8px;border-radius:10px;\
                                         background:var(--colorNeutralBackground3);\
                                         color:var(--color-text-secondary);">
                                {format!("{} пар.", n)}
                            </span>
                        }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }}
                    {if has_apis {
                        let n = apis.len();
                        view! {
                            <span style="font-size:11px;padding:2px 8px;border-radius:10px;\
                                         background:var(--colorNeutralBackground3);\
                                         color:var(--color-text-secondary);">
                                {format!("{} API", n)}
                            </span>
                        }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }}
                </div>
            </div>

            // ---- expanded body ----
            {move || expanded.get().then(|| {
                let description2   = description.clone();
                let apis2          = apis.clone();
                let constraints2   = constraints.clone();
                let fields2        = fields.clone();
                let has_apis2      = has_apis;
                let has_constraints2 = has_constraints;
                let has_fields2    = has_fields;

                view! {
                    <div style="padding:18px;display:flex;flex-direction:column;gap:18px;\
                                border-top:1px solid var(--color-border);\
                                background:var(--colorNeutralBackground2);">

                        // description
                        <div>
                            <div style="font-size:12px;font-weight:600;color:var(--color-text-secondary);\
                                        text-transform:uppercase;letter-spacing:0.06em;margin-bottom:6px;">
                                "Описание"
                            </div>
                            <div style="font-size:13px;color:var(--color-text);line-height:1.6;">
                                {description2}
                            </div>
                        </div>

                        // constraints
                        {if has_constraints2 {
                            let cs = constraints2.clone();
                            view! {
                                <div>
                                    <div style="font-size:12px;font-weight:600;\
                                                color:var(--color-text-secondary);\
                                                text-transform:uppercase;letter-spacing:0.06em;\
                                                margin-bottom:6px;">
                                        "Ограничения"
                                    </div>
                                    <ul style="margin:0;padding-left:18px;display:flex;\
                                               flex-direction:column;gap:3px;">
                                        {cs.into_iter().map(|c| view! {
                                            <li style="font-size:13px;color:var(--color-text);">
                                                {c}
                                            </li>
                                        }).collect_view()}
                                    </ul>
                                </div>
                            }.into_any()
                        } else {
                            view! { <></> }.into_any()
                        }}

                        // external APIs
                        {if has_apis2 {
                            let ap = apis2.clone();
                            view! {
                                <div>
                                    <div style="font-size:12px;font-weight:600;\
                                                color:var(--color-text-secondary);\
                                                text-transform:uppercase;letter-spacing:0.06em;\
                                                margin-bottom:8px;">
                                        "Внешние API"
                                    </div>
                                    <div style="display:flex;flex-direction:column;gap:6px;">
                                        {ap.into_iter().map(|a| view! {
                                            <div style="background:var(--colorNeutralBackground1);\
                                                        border:1px solid var(--color-border);\
                                                        border-radius:var(--radius-sm,6px);\
                                                        padding:10px 14px;\
                                                        display:flex;align-items:flex-start;gap:12px;">
                                                <div style="flex:1;">
                                                    <div style="font-size:13px;font-weight:600;
                                                                color:var(--color-text);">
                                                        {a.name.clone()}
                                                    </div>
                                                    <code style="font-size:11px;\
                                                                 color:var(--color-text-tertiary);">
                                                        {a.base_url.clone()}
                                                    </code>
                                                </div>
                                                {if !a.rate_limit_desc.is_empty() {
                                                    view! {
                                                        <span style="font-size:11px;padding:2px 8px;\
                                                                     border-radius:10px;white-space:nowrap;\
                                                                     background:var(--colorPaletteYellowBackground2,#fef3b422);\
                                                                     color:var(--colorPaletteYellowForeground1,#835c00);\
                                                                     border:1px solid var(--colorPaletteYellowBorder1,#c19c0088);">
                                                            {icon("zap")} " " {a.rate_limit_desc.clone()}
                                                        </span>
                                                    }.into_any()
                                                } else {
                                                    view! { <></> }.into_any()
                                                }}
                                            </div>
                                        }).collect_view()}
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <></> }.into_any()
                        }}

                        // config fields table
                        {if has_fields2 {
                            let flds = fields2.clone();
                            view! {
                                <div>
                                    <div style="font-size:12px;font-weight:600;\
                                                color:var(--color-text-secondary);\
                                                text-transform:uppercase;letter-spacing:0.06em;\
                                                margin-bottom:8px;">
                                        "Параметры конфигурации"
                                    </div>
                                    <div style="overflow-x:auto;">
                                        <table style="width:100%;border-collapse:collapse;\
                                                      font-size:12px;">
                                            <thead>
                                                <tr style="background:var(--colorNeutralBackground3);\
                                                           color:var(--color-text-secondary);\
                                                           font-size:11px;text-transform:uppercase;\
                                                           letter-spacing:0.05em;">
                                                    <th style="text-align:left;padding:7px 12px;\
                                                               white-space:nowrap;">"Ключ"</th>
                                                    <th style="text-align:left;padding:7px 12px;">"Название"</th>
                                                    <th style="text-align:left;padding:7px 12px;">"Тип"</th>
                                                    <th style="text-align:center;padding:7px 12px;">"Обяз."</th>
                                                    <th style="text-align:left;padding:7px 12px;">"По умолч."</th>
                                                    <th style="text-align:left;padding:7px 12px;">"Диапазон"</th>
                                                    <th style="text-align:left;padding:7px 12px;">"Подсказка"</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                {flds.into_iter().enumerate().map(|(i, f)| {
                                                    let bg = if i % 2 == 0 {
                                                        "background:var(--colorNeutralBackground1);"
                                                    } else {
                                                        "background:var(--colorNeutralBackground2);"
                                                    };
                                                    let range = match (f.min_value, f.max_value) {
                                                        (Some(mn), Some(mx)) => format!("{} – {}", mn, mx),
                                                        (Some(mn), None) => format!("≥ {}", mn),
                                                        (None, Some(mx)) => format!("≤ {}", mx),
                                                        (None, None) => "—".to_string(),
                                                    };
                                                    let badge_style = field_type_badge_style(&f.field_type).to_string();
                                                    let type_label  = field_type_label(&f.field_type).to_string();
                                                    let default_str = f.default_value.clone()
                                                        .map(|d| format!("`{}`", d))
                                                        .unwrap_or_else(|| "—".to_string());
                                                    let required_str = if f.required { "✓" } else { "" };
                                                    view! {
                                                        <tr style=bg>
                                                            <td style="padding:7px 12px;white-space:nowrap;">
                                                                <code style="font-family:monospace;\
                                                                             font-size:11px;font-weight:600;\
                                                                             color:var(--colorBrandForeground1);">
                                                                    {f.key.clone()}
                                                                </code>
                                                            </td>
                                                            <td style="padding:7px 12px;white-space:nowrap;">
                                                                {f.label.clone()}
                                                            </td>
                                                            <td style="padding:7px 12px;white-space:nowrap;">
                                                                <span style=badge_style>{type_label}</span>
                                                            </td>
                                                            <td style="padding:7px 12px;text-align:center;\
                                                                       color:var(--colorPaletteGreenForeground1);\
                                                                       font-weight:700;">
                                                                {required_str}
                                                            </td>
                                                            <td style="padding:7px 12px;white-space:nowrap;\
                                                                       font-family:monospace;font-size:11px;\
                                                                       color:var(--color-text-secondary);">
                                                                {default_str}
                                                            </td>
                                                            <td style="padding:7px 12px;white-space:nowrap;\
                                                                       color:var(--color-text-secondary);">
                                                                {range}
                                                            </td>
                                                            <td style="padding:7px 12px;\
                                                                       color:var(--color-text-tertiary);\
                                                                       font-size:11px;">
                                                                {f.hint.clone()}
                                                            </td>
                                                        </tr>
                                                    }
                                                }).collect_view()}
                                            </tbody>
                                        </table>
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div style="font-size:12px;color:var(--color-text-tertiary);\
                                            font-style:italic;">
                                    "Параметры не определены — используется произвольный JSON."
                                </div>
                            }.into_any()
                        }}
                    </div>
                }
            })}
        </div>
    }
}

// ============================================================================
// Main page
// ============================================================================

#[component]
pub fn TaskTypeRegistryPage() -> impl IntoView {
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal::<Option<String>>(None);
    let (types, set_types) = signal::<Vec<TaskMetadataDto>>(vec![]);

    Effect::new(move |_| {
        spawn_local(async move {
            match api::get_task_types().await {
                Ok(list) => {
                    set_types.set(list);
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    });

    view! {
        <div style="padding:24px;max-width:1100px;display:flex;flex-direction:column;gap:16px;">

            // ---- header ----
            <div style="display:flex;align-items:flex-start;gap:16px;flex-wrap:wrap;
                        margin-bottom:4px;">
                <div>
                    <h2 style="margin:0;font-size:18px;font-weight:700;color:var(--color-text);">
                        "Реестр типов заданий"
                    </h2>
                    <p style="margin:4px 0 0;font-size:13px;color:var(--color-text-secondary);">
                        "Все зарегистрированные обработчики регламентных заданий — параметры, "
                        "внешние API и ограничения."
                    </p>
                </div>
            </div>

            // ---- loading ----
            {move || loading.get().then(|| view! {
                <div style="display:flex;align-items:center;gap:8px;\
                            color:var(--color-text-secondary);font-size:13px;">
                    {icon("refresh-cw")} " Загрузка..."
                </div>
            })}

            // ---- error ----
            {move || error.get().map(|e| view! {
                <div style="padding:12px 16px;border-radius:var(--radius-sm);\
                            background:var(--colorPaletteRedBackground2);\
                            color:var(--colorPaletteRedForeground2);font-size:13px;">
                    {icon("alert-circle")} " " {e}
                </div>
            })}

            // ---- count badge ----
            {move || {
                let n = types.get().len();
                if n > 0 {
                    view! {
                        <div style="font-size:12px;color:var(--color-text-tertiary);">
                            {format!("{} тип{} заданий зарегистрировано",
                                n,
                                if n == 1 { "" } else if n < 5 { "а" } else { "ов" }
                            )}
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            // ---- cards ----
            {move || types.get().into_iter().map(|meta| {
                view! { <TaskTypeCard meta=meta /> }
            }).collect_view()}

        </div>
    }
}
