use super::dimension_input::DimensionInput;
use super::view_model::NomenclatureDetailsViewModel;
use super::model::{fetch_barcodes_by_nomenclature, fetch_dimension_values, BarcodesByNomenclatureResponse, DimensionValuesResponse};
use crate::shared::icons::icon;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::rc::Rc;

#[component]
pub fn NomenclatureDetails(
    id: Option<String>,
    #[prop(into)] on_saved: Callback<()>,
    #[prop(into)] on_cancel: Callback<()>,
) -> impl IntoView {
    let vm = NomenclatureDetailsViewModel::new();
    vm.load_if_needed(id.clone());

    let vm_clone = vm.clone();

    // Tab state
    let (active_tab, set_active_tab) = signal("general");

    // Barcodes state
    let (barcodes, set_barcodes) = signal::<Option<BarcodesByNomenclatureResponse>>(None);
    let (barcodes_loading, set_barcodes_loading) = signal(false);

    // Dimension values state
    let (dimension_values, set_dimension_values) = signal::<Option<DimensionValuesResponse>>(None);

    // Load dimension values
    spawn_local(async move {
        match fetch_dimension_values().await {
            Ok(data) => set_dimension_values.set(Some(data)),
            Err(_) => set_dimension_values.set(None),
        }
    });

    // Load barcodes if in edit mode
    if let Some(nomenclature_id) = id.clone() {
        spawn_local(async move {
            set_barcodes_loading.set(true);
            match fetch_barcodes_by_nomenclature(nomenclature_id).await {
                Ok(data) => set_barcodes.set(Some(data)),
                Err(_) => set_barcodes.set(None),
            }
            set_barcodes_loading.set(false);
        });
    }

    view! {
        <div class="details-container">
            <div class="details-header">
                <h3>
                    {
                        let vm = vm_clone.clone();
                        move || if vm.is_edit_mode()() { "Редактирование номенклатуры" } else { "Новая номенклатура" }
                    }
                </h3>
            </div>

            {
                let vm = vm_clone.clone();
                move || vm.error.get().map(|e| view! { <div class="error">{e}</div> })
            }

            // Tab buttons
            <div style="display: flex; gap: 5px; margin-bottom: 20px; border-bottom: 2px solid #ddd;">
                <button
                    on:click=move |_| set_active_tab.set("general")
                    style=move || format!(
                        "padding: 10px 20px; border: none; border-radius: 4px 4px 0 0; cursor: pointer; font-weight: 500; {}",
                        if active_tab.get() == "general" {
                            "background: #2196F3; color: white; border-bottom: 2px solid #2196F3;"
                        } else {
                            "background: #f5f5f5; color: #666;"
                        }
                    )
                >
                    "Основная"
                </button>
                <button
                    on:click=move |_| set_active_tab.set("barcodes")
                    style=move || format!(
                        "padding: 10px 20px; border: none; border-radius: 4px 4px 0 0; cursor: pointer; font-weight: 500; {}",
                        if active_tab.get() == "barcodes" {
                            "background: #2196F3; color: white; border-bottom: 2px solid #2196F3;"
                        } else {
                            "background: #f5f5f5; color: #666;"
                        }
                    )
                >
                    {move || {
                        let count = barcodes.get().map(|b| b.total_count).unwrap_or(0);
                        format!("Штрихкоды [{}]", count)
                    }}
                </button>
            </div>

            // Tab content area with fixed height
            <div style="height: 60vh; overflow: hidden;">
            {
                let vm_for_general = vm_clone.clone();
                move || if active_tab.get() == "general" {
                    view! {
                        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 30px; height: 100%; overflow-y: auto; padding: 0 20px;">
                            // Left column - Main fields
                            <div class="details-form">
                        <div class="form-group">
                            <label for="description">{"Наименование"}</label>
                            <input
                                type="text"
                                id="description"
                                prop:value={
                                    let vm = vm_for_general.clone();
                                    move || vm.form.get().description
                                }
                                on:input={
                                    let vm = vm_for_general.clone();
                                    move |ev| {
                                        vm.form.update(|f| f.description = event_target_value(&ev));
                                    }
                                }
                                placeholder="Введите наименование"
                            />
                        </div>

                        <div class="form-group">
                            <label for="full_description">{"Полное наименование"}</label>
                            <input
                                type="text"
                                id="full_description"
                                prop:value={
                                    let vm = vm_for_general.clone();
                                    move || vm.form.get().full_description.clone().unwrap_or_default()
                                }
                                on:input={
                                    let vm = vm_for_general.clone();
                                    move |ev| {
                                        let v = event_target_value(&ev);
                                        vm.form.update(|f| f.full_description = if v.trim().is_empty() { None } else { Some(v) });
                                    }
                                }
                                placeholder="Полное наименование (опционально)"
                            />
                        </div>

                        <div class="form-group">
                            <label for="code">{"Код"}</label>
                            <input
                                type="text"
                                id="code"
                                prop:value={
                                    let vm = vm_for_general.clone();
                                    move || vm.form.get().code.clone().unwrap_or_default()
                                }
                                on:input={
                                    let vm = vm_for_general.clone();
                                    move |ev| {
                                        vm.form.update(|f| f.code = Some(event_target_value(&ev)));
                                    }
                                }
                                placeholder="Введите код (необязательно)"
                            />
                        </div>

                        <div class="form-group">
                            <label for="article">{"Артикул"}</label>
                            <input
                                type="text"
                                id="article"
                                prop:value={
                                    let vm = vm_for_general.clone();
                                    move || vm.form.get().article.clone().unwrap_or_default()
                                }
                                on:input={
                                    let vm = vm_for_general.clone();
                                    move |ev| {
                                        let v = event_target_value(&ev);
                                        vm.form.update(|f| f.article = if v.trim().is_empty() { None } else { Some(v) });
                                    }
                                }
                                placeholder="Артикул (опционально)"
                            />
                        </div>

                        <div class="form-group">
                            <label for="is_folder">{"Это папка"}</label>
                            <input
                                type="checkbox"
                                id="is_folder"
                                prop:checked={
                                    let vm = vm_for_general.clone();
                                    move || vm.form.get().is_folder
                                }
                                on:change={
                                    let vm = vm_for_general.clone();
                                    move |ev| {
                                        vm.form.update(|f| f.is_folder = event_target_checked(&ev));
                                    }
                                }
                            />
                        </div>

                        <div class="form-group">
                            <label for="parent_id">{"Родитель (UUID)"}</label>
                            <input
                                type="text"
                                id="parent_id"
                                prop:value={
                                    let vm = vm_for_general.clone();
                                    move || vm.form.get().parent_id.clone().unwrap_or_default()
                                }
                                on:input={
                                    let vm = vm_for_general.clone();
                                    move |ev| {
                                        let v = event_target_value(&ev);
                                        vm.form.update(|f| f.parent_id = if v.trim().is_empty() { None } else { Some(v) });
                                    }
                                }
                                placeholder="UUID родителя (опционально)"
                            />
                        </div>

                        <div class="form-group">
                            <label for="comment">{"Комментарий"}</label>
                            <textarea
                                id="comment"
                                prop:value={
                                    let vm = vm_for_general.clone();
                                    move || vm.form.get().comment.clone().unwrap_or_default()
                                }
                                on:input={
                                    let vm = vm_for_general.clone();
                                    move |ev| {
                                        let v = event_target_value(&ev);
                                        vm.form.update(|f| f.comment = if v.trim().is_empty() { None } else { Some(v) });
                                    }
                                }
                                placeholder="Комментарий (опционально)"
                            />
                        </div>
                    </div>

                    // Right column - Измерения (классификация)
                    <div>
                        <h4 style="margin-bottom: 15px; margin-top: 0; color: #333;">{"Измерения"}</h4>

                    <DimensionInput
                        id="dim1_category"
                        label="Категория"
                        placeholder="Категория (макс. 40 символов)"
                        maxlength=40
                        value={
                            let vm = vm_for_general.clone();
                            Signal::derive(move || vm.form.get().dim1_category.clone().unwrap_or_default())
                        }
                        on_change={
                            let vm = vm_for_general.clone();
                            Callback::new(move |v: String| {
                                vm.form.update(|f| f.dim1_category = if v.trim().is_empty() { None } else { Some(v) });
                            })
                        }
                        options={
                            Signal::derive(move || {
                                dimension_values.get()
                                    .map(|dims| dims.dim1_category.clone())
                                    .unwrap_or_default()
                            })
                        }
                    />

                    <DimensionInput
                        id="dim2_line"
                        label="Линейка"
                        placeholder="Линейка (макс. 40 символов)"
                        maxlength=40
                        value={
                            let vm = vm_for_general.clone();
                            Signal::derive(move || vm.form.get().dim2_line.clone().unwrap_or_default())
                        }
                        on_change={
                            let vm = vm_for_general.clone();
                            Callback::new(move |v: String| {
                                vm.form.update(|f| f.dim2_line = if v.trim().is_empty() { None } else { Some(v) });
                            })
                        }
                        options={
                            Signal::derive(move || {
                                dimension_values.get()
                                    .map(|dims| dims.dim2_line.clone())
                                    .unwrap_or_default()
                            })
                        }
                    />

                    <DimensionInput
                        id="dim3_model"
                        label="Модель"
                        placeholder="Модель (макс. 80 символов)"
                        maxlength=80
                        value={
                            let vm = vm_for_general.clone();
                            Signal::derive(move || vm.form.get().dim3_model.clone().unwrap_or_default())
                        }
                        on_change={
                            let vm = vm_for_general.clone();
                            Callback::new(move |v: String| {
                                vm.form.update(|f| f.dim3_model = if v.trim().is_empty() { None } else { Some(v) });
                            })
                        }
                        options={
                            Signal::derive(move || {
                                dimension_values.get()
                                    .map(|dims| dims.dim3_model.clone())
                                    .unwrap_or_default()
                            })
                        }
                    />

                    <DimensionInput
                        id="dim4_format"
                        label="Формат"
                        placeholder="Формат (макс. 20 символов)"
                        maxlength=20
                        value={
                            let vm = vm_for_general.clone();
                            Signal::derive(move || vm.form.get().dim4_format.clone().unwrap_or_default())
                        }
                        on_change={
                            let vm = vm_for_general.clone();
                            Callback::new(move |v: String| {
                                vm.form.update(|f| f.dim4_format = if v.trim().is_empty() { None } else { Some(v) });
                            })
                        }
                        options={
                            Signal::derive(move || {
                                dimension_values.get()
                                    .map(|dims| dims.dim4_format.clone())
                                    .unwrap_or_default()
                            })
                        }
                    />

                    <DimensionInput
                        id="dim5_sink"
                        label="Раковина"
                        placeholder="Раковина (макс. 40 символов)"
                        maxlength=40
                        value={
                            let vm = vm_for_general.clone();
                            Signal::derive(move || vm.form.get().dim5_sink.clone().unwrap_or_default())
                        }
                        on_change={
                            let vm = vm_for_general.clone();
                            Callback::new(move |v: String| {
                                vm.form.update(|f| f.dim5_sink = if v.trim().is_empty() { None } else { Some(v) });
                            })
                        }
                        options={
                            Signal::derive(move || {
                                dimension_values.get()
                                    .map(|dims| dims.dim5_sink.clone())
                                    .unwrap_or_default()
                            })
                        }
                    />

                    <DimensionInput
                        id="dim6_size"
                        label="Размер"
                        placeholder="Размер (макс. 20 символов)"
                        maxlength=20
                        value={
                            let vm = vm_for_general.clone();
                            Signal::derive(move || vm.form.get().dim6_size.clone().unwrap_or_default())
                        }
                        on_change={
                            let vm = vm_for_general.clone();
                            Callback::new(move |v: String| {
                                vm.form.update(|f| f.dim6_size = if v.trim().is_empty() { None } else { Some(v) });
                            })
                        }
                        options={
                            Signal::derive(move || {
                                dimension_values.get()
                                    .map(|dims| dims.dim6_size.clone())
                                    .unwrap_or_default()
                            })
                        }
                    />
                </div>
            </div>
                    }.into_any()
                } else if active_tab.get() == "barcodes" {
                    view! {
                        <div style="height: 100%; overflow-y: auto; padding: 20px;">
                    {move || {
                        if barcodes_loading.get() {
                            view! {
                                <div style="text-align: center; padding: 20px;">
                                    "Загрузка штрихкодов..."
                                </div>
                            }.into_any()
                        } else if let Some(data) = barcodes.get() {
                            if data.barcodes.is_empty() {
                                view! {
                                    <div style="text-align: center; padding: 20px; color: #666;">
                                        "Штрихкоды не найдены"
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div>
                                        <div style="margin-bottom: 15px; color: #666;">
                                            "Всего штрихкодов: " {data.total_count}
                                        </div>
                                        <table style="width: 100%; border-collapse: collapse; background: white; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                                            <thead>
                                                <tr style="background: #f8f9fa; border-bottom: 2px solid #dee2e6;">
                                                    <th style="padding: 12px; text-align: left; font-weight: 600; color: #495057;">"Штрихкод"</th>
                                                    <th style="padding: 12px; text-align: left; font-weight: 600; color: #495057;">"Источник"</th>
                                                    <th style="padding: 12px; text-align: left; font-weight: 600; color: #495057;">"Артикул"</th>
                                                    <th style="padding: 12px; text-align: left; font-weight: 600; color: #495057;">"Дата обновления"</th>
                                                    <th style="padding: 12px; text-align: center; font-weight: 600; color: #495057;">"Активен"</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                {data.barcodes.iter().enumerate().map(|(idx, barcode)| {
                                                    let bg_color = if idx % 2 == 0 { "#fff" } else { "#f9f9f9" };
                                                    view! {
                                                        <tr style={format!("background: {}; border-bottom: 1px solid #eee;", bg_color)}>
                                                            <td style="padding: 10px; font-family: monospace;">{barcode.barcode.clone()}</td>
                                                            <td style="padding: 10px;">
                                                                <span style={format!("padding: 2px 8px; border-radius: 3px; background: {}; color: white; font-size: 11px;",
                                                                    match barcode.source.as_str() {
                                                                        "1C" => "#6c757d",
                                                                        "OZON" => "#0088cc",
                                                                        "WB" => "#8b00ff",
                                                                        "YM" => "#fc0",
                                                                        _ => "#333",
                                                                    }
                                                                )}>
                                                                    {barcode.source.clone()}
                                                                </span>
                                                            </td>
                                                            <td style="padding: 10px;">{barcode.article.clone().unwrap_or_else(|| "-".to_string())}</td>
                                                            <td style="padding: 10px; font-size: 12px;">{barcode.updated_at.clone()}</td>
                                                            <td style="padding: 10px; text-align: center;">
                                                                {if barcode.is_active {
                                                                    view! { <span style="color: #28a745; font-weight: bold;">"✓"</span> }.into_any()
                                                                } else {
                                                                    view! { <span style="color: #dc3545; font-weight: bold;">"✗"</span> }.into_any()
                                                                }}
                                                            </td>
                                                        </tr>
                                                    }
                                                }).collect_view()}
                                            </tbody>
                                        </table>
                                    </div>
                                }.into_any()
                            }
                        } else {
                            view! {
                                <div style="text-align: center; padding: 20px; color: #999;">
                                    "Нет данных"
                                </div>
                            }.into_any()
                        }
                    }}
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }
            </div>

            <div class="details-actions">
                <button
                    class="btn btn-primary"
                    on:click={
                        let vm = vm_clone.clone();
                        move |_| {
                            let cb = Callback::from(move || on_saved.run(()));
                            vm.save_command(Rc::new(move |_| cb.run(())))()
                        }
                    }
                    disabled={
                        let vm = vm_clone.clone();
                        move || !vm.is_form_valid()()
                    }
                >
                    {icon("save")}
                    {"Сохранить"}
                </button>
                <button
                    class="btn btn-secondary"
                    on:click=move |_| on_cancel.run(())
                >
                    {icon("cancel")}
                    {"Отмена"}
                </button>
            </div>
        </div>
    }
}
