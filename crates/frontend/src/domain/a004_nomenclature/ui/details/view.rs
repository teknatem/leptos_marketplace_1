use super::dimension_input::DimensionInput;
use super::model::{
    fetch_barcodes_by_nomenclature, fetch_dimension_values, BarcodesByNomenclatureResponse,
    DimensionValuesResponse,
};
use crate::shared::icons::icon;
use contracts::domain::a004_nomenclature::aggregate::NomenclatureDto;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

fn opt(v: String) -> Option<String> {
    if v.trim().is_empty() {
        None
    } else {
        Some(v)
    }
}

#[component]
pub fn NomenclatureDetails(
    id: Option<String>,
    #[prop(into)] on_saved: Callback<()>,
    #[prop(into)] on_cancel: Callback<()>,
) -> impl IntoView {
    // Form fields (Thaw-friendly RwSignals)
    let id_state = RwSignal::new(id.clone());
    let code = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let full_description = RwSignal::new(String::new());
    let article = RwSignal::new(String::new());
    let comment = RwSignal::new(String::new());
    let is_folder = RwSignal::new(false);
    let parent_id = RwSignal::new(String::new());

    // Dimension fields
    let dim1_category = RwSignal::new(String::new());
    let dim2_line = RwSignal::new(String::new());
    let dim3_model = RwSignal::new(String::new());
    let dim4_format = RwSignal::new(String::new());
    let dim5_sink = RwSignal::new(String::new());
    let dim6_size = RwSignal::new(String::new());

    let error = RwSignal::new(None::<String>);
    let saving = RwSignal::new(false);

    // Tabs
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

    // Load entity & barcodes on mount (edit mode)
    Effect::new(move |_| {
        let Some(nomenclature_id) = id_state.get() else {
            return;
        };

        let id_for_entity = nomenclature_id.clone();
        spawn_local(async move {
            error.set(None);
            match super::model::fetch_by_id(id_for_entity).await {
                Ok(item) => {
                    code.set(item.base.code);
                    description.set(item.base.description);
                    full_description.set(item.full_description);
                    article.set(item.article);
                    comment.set(item.base.comment.unwrap_or_default());
                    is_folder.set(item.is_folder);
                    parent_id.set(item.parent_id.unwrap_or_default());

                    dim1_category.set(item.dim1_category);
                    dim2_line.set(item.dim2_line);
                    dim3_model.set(item.dim3_model);
                    dim4_format.set(item.dim4_format);
                    dim5_sink.set(item.dim5_sink);
                    dim6_size.set(item.dim6_size);
                }
                Err(e) => error.set(Some(e)),
            }
        });

        let id_for_barcodes = nomenclature_id.clone();
        spawn_local(async move {
            set_barcodes_loading.set(true);
            match fetch_barcodes_by_nomenclature(id_for_barcodes).await {
                Ok(data) => set_barcodes.set(Some(data)),
                Err(_) => set_barcodes.set(None),
            }
            set_barcodes_loading.set(false);
        });
    });

    let is_edit_mode = Signal::derive(move || id_state.get().is_some());
    let is_form_valid = Signal::derive(move || !description.get().trim().is_empty());

    let handle_save = move |_| {
        if !is_form_valid.get() {
            error.set(Some("Наименование обязательно для заполнения".to_string()));
            return;
        }

        saving.set(true);
        error.set(None);

        let dto = NomenclatureDto {
            id: id_state.get(),
            code: opt(code.get()),
            description: description.get(),
            full_description: opt(full_description.get()),
            is_folder: is_folder.get(),
            parent_id: opt(parent_id.get()),
            article: opt(article.get()),
            comment: opt(comment.get()),
            updated_at: None,
            mp_ref_count: 0,
            dim1_category: opt(dim1_category.get()),
            dim2_line: opt(dim2_line.get()),
            dim3_model: opt(dim3_model.get()),
            dim4_format: opt(dim4_format.get()),
            dim5_sink: opt(dim5_sink.get()),
            dim6_size: opt(dim6_size.get()),
            is_assembly: None,
            base_nomenclature_ref: None,
        };

        spawn_local(async move {
            match super::model::save_form(dto).await {
                Ok(_) => {
                    saving.set(false);
                    on_saved.run(());
                }
                Err(e) => {
                    saving.set(false);
                    error.set(Some(e));
                }
            }
        });
    };

    view! {
        <div class="details-container nomenclature-details">
            <div class="modal-header">
                <h3 class="modal-title">
                    {move || if is_edit_mode.get() { "Редактирование номенклатуры" } else { "Новая номенклатура" }}
                </h3>
                <div class="modal-header-actions">
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=handle_save
                        disabled=Signal::derive(move || saving.get() || !is_form_valid.get())
                    >
                        {icon("save")}
                        " Сохранить"
                    </Button>
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| on_cancel.run(())>
                        {icon("x")}
                        " Закрыть"
                    </Button>
                </div>
            </div>

            <div class="modal-body">
                {move || error.get().map(|e| view! {
                    <div class="warning-box" style="background: var(--color-error-50); border-color: var(--color-error-100); margin-bottom: var(--spacing-md);">
                        <span class="warning-box__icon" style="color: var(--color-error);">"⚠"</span>
                        <span class="warning-box__text" style="color: var(--color-error);">{e}</span>
                    </div>
                })}

                <div class="detail-tabs" style="margin-bottom: var(--spacing-md);">
                    <button
                        type="button"
                        class=move || if active_tab.get() == "general" { "detail-tabs__item detail-tabs__item--active" } else { "detail-tabs__item" }
                        on:click=move |_| set_active_tab.set("general")
                    >
                        "Основная"
                    </button>
                    <button
                        type="button"
                        class=move || if active_tab.get() == "barcodes" { "detail-tabs__item detail-tabs__item--active" } else { "detail-tabs__item" }
                        on:click=move |_| set_active_tab.set("barcodes")
                        disabled=move || !is_edit_mode.get()
                        title=move || if is_edit_mode.get() { "" } else { "Доступно после сохранения" }
                    >
                        <span>"Штрихкоды"</span>
                        <span class="detail-tabs__badge">
                            {move || barcodes.get().map(|b| b.total_count).unwrap_or(0)}
                        </span>
                    </button>
                </div>

                <div style="height: 60vh; overflow: hidden;">
                    {move || if active_tab.get() == "general" {
                        view! {
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-lg); height: 100%; overflow-y: auto;">
                                <div class="details-section">
                                    <h4 class="details-section__title">"Основные поля"</h4>
                                    <div class="details-grid--3col">
                                        <div class="form__group" style="grid-column: 1 / -1;">
                                            <label class="form__label">"Наименование"</label>
                                            <Input value=description placeholder="Введите наименование" />
                                        </div>

                                        <div class="form__group" style="grid-column: 1 / -1;">
                                            <label class="form__label">"Полное наименование"</label>
                                            <Input value=full_description placeholder="Опционально" />
                                        </div>

                                        <div class="form__group">
                                            <label class="form__label">"Код"</label>
                                            <Input value=code placeholder="Опционально" />
                                        </div>

                                        <div class="form__group">
                                            <label class="form__label">"Артикул"</label>
                                            <Input value=article placeholder="Опционально" />
                                        </div>

                                        <div class="form__group">
                                            <label class="form__label">"Родитель (UUID)"</label>
                                            <Input value=parent_id placeholder="Опционально" />
                                        </div>

                                        <div class="form__group" style="grid-column: 1 / -1;">
                                            <label class="form__label">"Комментарий"</label>
                                            <Textarea value=comment placeholder="Опционально" attr:rows=3 />
                                        </div>

                                        <div class="details-flags" style="grid-column: 1 / -1;">
                                            <Checkbox checked=is_folder label="Это папка" />
                                        </div>
                                    </div>
                                </div>

                                <div class="details-section">
                                    <h4 class="details-section__title">"Измерения"</h4>

                                    <DimensionInput
                                        id="dim1_category"
                                        label="Категория"
                                        placeholder="Категория (макс. 40 символов)"
                                        maxlength=40
                                        value=Signal::derive(move || dim1_category.get())
                                        on_change=Callback::new(move |v| dim1_category.set(v))
                                        options=Signal::derive(move || dimension_values.get().map(|d| d.dim1_category).unwrap_or_default())
                                    />

                                    <DimensionInput
                                        id="dim2_line"
                                        label="Линейка"
                                        placeholder="Линейка (макс. 40 символов)"
                                        maxlength=40
                                        value=Signal::derive(move || dim2_line.get())
                                        on_change=Callback::new(move |v| dim2_line.set(v))
                                        options=Signal::derive(move || dimension_values.get().map(|d| d.dim2_line).unwrap_or_default())
                                    />

                                    <DimensionInput
                                        id="dim3_model"
                                        label="Модель"
                                        placeholder="Модель (макс. 80 символов)"
                                        maxlength=80
                                        value=Signal::derive(move || dim3_model.get())
                                        on_change=Callback::new(move |v| dim3_model.set(v))
                                        options=Signal::derive(move || dimension_values.get().map(|d| d.dim3_model).unwrap_or_default())
                                    />

                                    <DimensionInput
                                        id="dim4_format"
                                        label="Формат"
                                        placeholder="Формат (макс. 20 символов)"
                                        maxlength=20
                                        value=Signal::derive(move || dim4_format.get())
                                        on_change=Callback::new(move |v| dim4_format.set(v))
                                        options=Signal::derive(move || dimension_values.get().map(|d| d.dim4_format).unwrap_or_default())
                                    />

                                    <DimensionInput
                                        id="dim5_sink"
                                        label="Раковина"
                                        placeholder="Раковина (макс. 40 символов)"
                                        maxlength=40
                                        value=Signal::derive(move || dim5_sink.get())
                                        on_change=Callback::new(move |v| dim5_sink.set(v))
                                        options=Signal::derive(move || dimension_values.get().map(|d| d.dim5_sink).unwrap_or_default())
                                    />

                                    <DimensionInput
                                        id="dim6_size"
                                        label="Размер"
                                        placeholder="Размер (макс. 20 символов)"
                                        maxlength=20
                                        value=Signal::derive(move || dim6_size.get())
                                        on_change=Callback::new(move |v| dim6_size.set(v))
                                        options=Signal::derive(move || dimension_values.get().map(|d| d.dim6_size).unwrap_or_default())
                                    />
                                </div>
                            </div>
                        }.into_any()
                    } else if active_tab.get() == "barcodes" {
                        view! {
                            <div style="height: 100%; overflow-y: auto;">
                                {move || {
                                    if barcodes_loading.get() {
                                        view! { <div style="padding: var(--spacing-md); color: var(--color-text-tertiary);">"Загрузка..."</div> }.into_any()
                                    } else if let Some(data) = barcodes.get() {
                                        view! {
                                            <div class="details-section">
                                                <h4 class="details-section__title">
                                                    {format!("Штрихкоды ({})", data.total_count)}
                                                </h4>
                                                <Table>
                                                    <TableHeader>
                                                        <TableRow>
                                                            <TableHeaderCell resizable=true min_width=180.0>"Штрихкод"</TableHeaderCell>
                                                            <TableHeaderCell resizable=true min_width=110.0>"Источник"</TableHeaderCell>
                                                            <TableHeaderCell resizable=true min_width=120.0>"Артикул"</TableHeaderCell>
                                                            <TableHeaderCell resizable=true min_width=160.0>"Обновлено"</TableHeaderCell>
                                                            <TableHeaderCell resizable=false>"Активен"</TableHeaderCell>
                                                        </TableRow>
                                                    </TableHeader>
                                                    <TableBody>
                                                        {data.barcodes.clone().into_iter().map(|b| {
                                                            let src = b.source.clone();
                                                            let badge_color = match src.as_str() {
                                                                "WB" => BadgeColor::Important,
                                                                "OZON" => BadgeColor::Brand,
                                                                "YM" => BadgeColor::Warning,
                                                                "1C" => BadgeColor::Success,
                                                                _ => BadgeColor::Brand,
                                                            };
                                                            view! {
                                                                <TableRow>
                                                                    <TableCell><TableCellLayout truncate=true>{b.barcode.clone()}</TableCellLayout></TableCell>
                                                                    <TableCell>
                                                                        <Badge appearance=BadgeAppearance::Tint color=badge_color>
                                                                            {b.source.clone()}
                                                                        </Badge>
                                                                    </TableCell>
                                                                    <TableCell><TableCellLayout truncate=true>{b.article.clone().unwrap_or_else(|| "—".to_string())}</TableCellLayout></TableCell>
                                                                    <TableCell><TableCellLayout truncate=true>{b.updated_at.clone()}</TableCellLayout></TableCell>
                                                                    <TableCell>
                                                                        {if b.is_active {
                                                                            view! { <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Success>"✓"</Badge> }.into_any()
                                                                        } else {
                                                                            view! { <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Danger>"✗"</Badge> }.into_any()
                                                                        }}
                                                                    </TableCell>
                                                                </TableRow>
                                                            }
                                                        }).collect_view()}
                                                    </TableBody>
                                                </Table>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! { <div style="padding: var(--spacing-md); color: var(--color-text-tertiary);">"Нет данных"</div> }.into_any()
                                    }
                                }}
                            </div>
                        }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }}
                </div>
            </div>
        </div>
    }
}
