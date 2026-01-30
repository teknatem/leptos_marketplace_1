use crate::shared::universal_dashboard::ui::{ConditionDisplay, ConditionEditorModal};
use contracts::shared::universal_dashboard::{
    AggregateFunction, DashboardConfig, DataSourceSchemaOwned, FieldDefOwned, FieldRole,
    FilterCondition, SelectedField,
};
use leptos::logging::log;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn SettingsTable(
    /// Current configuration
    #[prop(into)]
    config: Signal<DashboardConfig>,
    /// Schema for the data source
    schema: DataSourceSchemaOwned,
    /// Current loaded config ID (None if new/unsaved)
    #[prop(into, optional)]
    current_config_id: Option<Signal<Option<String>>>,
    /// Filter state - show only selected fields
    show_only_selected: RwSignal<bool>,
    /// Flag to prevent Effect loops during config loading
    is_loading_config: RwSignal<bool>,
    /// Callback when configuration changes
    on_config_change: Callback<DashboardConfig>,
    /// Callback to save to current config (update existing)
    #[prop(optional)]
    on_save: Option<Callback<()>>,
    /// Callback to save as new config
    #[prop(optional)]
    on_save_as: Option<Callback<()>>,
) -> impl IntoView {
    // Compute show_all from the switch state
    let show_all = Memo::new(move |_| !show_only_selected.get());

    let sort_field = RwSignal::new("name".to_string());
    let sort_ascending = RwSignal::new(true);

    let toggle_sort = move |field: &'static str| {
        if sort_field.get() == field {
            sort_ascending.update(|a| *a = !*a);
        } else {
            sort_field.set(field.to_string());
            sort_ascending.set(true);
        }
    };

    // Master checkbox - computed from config
    let all_enabled = Memo::new({
        let schema_fields = schema.fields.clone();
        move |_| {
            let cfg = config.get();
            let total = schema_fields.len();
            let enabled = cfg.enabled_fields.len();
            enabled == total && enabled > 0
        }
    });

    // Handle master checkbox toggle
    let on_master_toggle = {
        let schema_fields = schema.fields.clone();
        move |new_val: bool| {
            let mut cfg = config.get_untracked();
            if new_val {
                // Enable all
                cfg.enabled_fields = schema_fields.iter().map(|f| f.id.clone()).collect();
            } else {
                // Disable all
                cfg.enabled_fields.clear();
            }
            on_config_change.run(cfg);
        }
    };

    // Filter and sort fields
    let filtered_fields = move || {
        let mut fields = schema.fields.clone();

        // Filter if needed (show only enabled fields when switch is on)
        if !show_all.get() {
            let cfg = config.get();
            fields.retain(|field| {
                // Show field if it's enabled (checked)
                cfg.enabled_fields.contains(&field.id)
            });
        }

        // Sort fields
        let field = sort_field.get();
        let ascending = sort_ascending.get();
        fields.sort_by(|a, b| {
            let cmp = match field.as_str() {
                "name" => a.name.cmp(&b.name),
                "id" => a.id.cmp(&b.id),
                _ => std::cmp::Ordering::Equal,
            };
            if ascending {
                cmp
            } else {
                cmp.reverse()
            }
        });

        fields
    };

    view! {
        <div class="settings-table-wrapper">
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center style="margin-bottom: 16px;">
                <Flex align=FlexAlign::Center style="gap: 12px;">
                    <Switch checked=show_only_selected label="Только выбранные"/>
                </Flex>
                <Space>
                    {move || {
                        // "Сохранить" button - only enabled when we have a loaded config
                        let has_config = current_config_id
                            .map(|sig| sig.get().is_some())
                            .unwrap_or(false);
                        on_save.as_ref().map(|save_cb| {
                            let save_cb = *save_cb;
                            view! {
                                <Button
                                    appearance=ButtonAppearance::Secondary
                                    on_click=move |_| save_cb.run(())
                                    disabled=move || !has_config
                                >
                                    "Сохранить"
                                </Button>
                            }
                        })
                    }}
                    {move || on_save_as.as_ref().map(|save_as_cb| {
                        let save_as_cb = *save_as_cb;
                        view! {
                            <Button
                                appearance=ButtonAppearance::Secondary
                                on_click=move |_| save_as_cb.run(())
                            >
                                "Сохранить как..."
                            </Button>
                        }
                    })}
                </Space>
            </Flex>
            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHeaderCell attr:style="width: 32px;">
                            <div style="text-align: center;">
                                <input
                                    type="checkbox"
                                    checked=move || all_enabled.get()
                                    on:change=move |ev| {
                                        let checked = event_target_checked(&ev);
                                        on_master_toggle(checked);
                                    }
                                    style="cursor: pointer;"
                                />
                            </div>
                        </TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=200.0>
                            <div style="cursor: pointer; user-select: none;" on:click=move |_| toggle_sort("name")>
                                "Наименование"
                                {move || {
                                    if sort_field.get() == "name" {
                                        if sort_ascending.get() { " ▲" } else { " ▼" }
                                    } else {
                                        ""
                                    }
                                }}
                            </div>
                        </TableHeaderCell>
                        <TableHeaderCell attr:style="width: 50px;">
                            <div style="text-align: center;">"Тип"</div>
                        </TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=150.0>
                            <div style="cursor: pointer; user-select: none;" on:click=move |_| toggle_sort("id")>
                                "Идентификатор"
                                {move || {
                                    if sort_field.get() == "id" {
                                        if sort_ascending.get() { " ▲" } else { " ▼" }
                                    } else {
                                        ""
                                    }
                                }}
                            </div>
                        </TableHeaderCell>
                        <TableHeaderCell min_width=120.0>"Роль"</TableHeaderCell>
                        <TableHeaderCell min_width=100.0>"Функция"</TableHeaderCell>
                        <TableHeaderCell min_width=250.0>"Условие"</TableHeaderCell>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    <For
                        each=move || filtered_fields()
                        key=|field| field.id.clone()
                        children=move |field| {
                            view! {
                                <FieldRow
                                    field=field
                                    config=config
                                    is_loading_config=is_loading_config
                                    on_config_change=on_config_change
                                />
                            }
                        }
                    />
                </TableBody>
            </Table>
        </div>
    }
}

#[component]
fn FieldRow(
    field: FieldDefOwned,
    #[prop(into)] config: Signal<DashboardConfig>,
    is_loading_config: RwSignal<bool>,
    on_config_change: Callback<DashboardConfig>,
) -> impl IntoView {
    let field_id = field.id.clone();
    let field_id_for_code = field_id.clone();
    let field_name = field.name.clone();
    let can_group = field.can_group;
    let can_aggregate = field.can_aggregate;

    // Get current role for function display (wrapped in StoredValue for multi-use)
    let get_role_func = StoredValue::new({
        let field_id = field_id.clone();
        move || -> FieldRole {
            let cfg = config.get();
            if cfg.groupings.contains(&field_id) {
                FieldRole::Grouping
            } else if cfg.display_fields.contains(&field_id) {
                FieldRole::Display
            } else if cfg.selected_fields.iter().any(|sf| sf.field_id == field_id) {
                FieldRole::Measure
            } else {
                FieldRole::None
            }
        }
    });

    // Role Select value - create RwSignal that derives from config
    let role_select_value = RwSignal::new({
        let cfg = config.get_untracked();
        if cfg.groupings.contains(&field_id) {
            "grouping".to_string()
        } else if cfg.display_fields.contains(&field_id) {
            "display".to_string()
        } else if cfg.selected_fields.iter().any(|sf| sf.field_id == field_id) {
            "measure".to_string()
        } else {
            "none".to_string()
        }
    });

    // Update role_select_value when config changes (from outside, like loading saved config)
    Effect::new({
        let field_id = field_id.clone();
        move |_| {
            let cfg = config.get();
            let new_value = if cfg.groupings.contains(&field_id) {
                "grouping".to_string()
            } else if cfg.display_fields.contains(&field_id) {
                "display".to_string()
            } else if cfg.selected_fields.iter().any(|sf| sf.field_id == field_id) {
                "measure".to_string()
            } else {
                "none".to_string()
            };
            let current = role_select_value.get_untracked();
            log!(
                "[Effect 1] Field: {}, new_value: {}, current: {}",
                field_id,
                new_value,
                current
            );

            // Force update even if value is the same - set to different value first, then set target
            if current == new_value {
                log!("[Effect 1] FORCING UPDATE for field: {}", field_id);
                // Set to a different valid value first to trigger change
                let temp_value = if new_value == "none" {
                    "grouping"
                } else {
                    "none"
                };
                role_select_value.set(temp_value.to_string());
            }
            role_select_value.set(new_value);
        }
    });

    // Update config when user changes role_select_value
    Effect::new({
        let field_id = field_id.clone();
        let on_config_change = on_config_change.clone();
        move |prev: Option<String>| {
            let current = role_select_value.get();

            // Skip first run (initialization)
            if prev.is_none() {
                return current.clone();
            }

            // Skip if config is being loaded (prevent loop) - use untracked to avoid dependency
            if is_loading_config.get_untracked() {
                log!("[Effect 2] Field: {}, SKIPPED (loading)", field_id);
                return current.clone();
            }

            // Only process if value actually changed
            if Some(&current) == prev.as_ref() {
                log!("[Effect 2] Field: {}, SKIPPED (no change)", field_id);
                return current.clone();
            }

            log!(
                "[Effect 2] Field: {}, updating config, new role: {}",
                field_id,
                current
            );

            let role = match current.as_str() {
                "grouping" => FieldRole::Grouping,
                "display" => FieldRole::Display,
                "measure" => FieldRole::Measure,
                _ => FieldRole::None,
            };

            let mut cfg = config.get_untracked();
            cfg.groupings.retain(|g| g != &field_id);
            cfg.display_fields.retain(|d| d != &field_id);
            cfg.selected_fields.retain(|sf| sf.field_id != field_id);

            match role {
                FieldRole::None => {}
                FieldRole::Grouping => {
                    cfg.groupings.push(field_id.clone());
                }
                FieldRole::Display => {
                    cfg.display_fields.push(field_id.clone());
                }
                FieldRole::Measure => {
                    cfg.selected_fields.push(SelectedField {
                        field_id: field_id.clone(),
                        aggregate: Some(AggregateFunction::Sum),
                    });
                }
            }

            on_config_change.run(cfg);
            current
        }
    });

    // Get current condition (new format) - wrapped in StoredValue for multi-use
    let get_condition = StoredValue::new({
        let field_id = field_id.clone();
        move || -> Option<FilterCondition> {
            let cfg = config.get();
            cfg.filters
                .conditions
                .iter()
                .find(|c| c.field_id == field_id)
                .cloned()
        }
    });

    // Modal state for condition editor
    let editor_open = RwSignal::new(false);
    let field_for_editor = field.clone();
    let editing_field = Signal::derive(move || {
        if editor_open.get() {
            Some(field_for_editor.clone())
        } else {
            None
        }
    });

    // Create callbacks outside of view! macro
    let on_save_callback = Callback::new({
        let field_id = field_id.clone();
        let on_config_change = on_config_change.clone();
        move |new_condition: FilterCondition| {
            let mut cfg = config.get_untracked();
            // Remove existing condition for this field
            cfg.filters.conditions.retain(|c| c.field_id != field_id);
            // Add new condition
            cfg.filters.conditions.push(new_condition);
            on_config_change.run(cfg);
        }
    });

    let on_clear_callback = Callback::new({
        let field_id = field_id.clone();
        let on_config_change = on_config_change.clone();
        move |_| {
            let mut cfg = config.get_untracked();
            cfg.filters.conditions.retain(|c| c.field_id != field_id);
            on_config_change.run(cfg);
        }
    });

    let on_edit_callback = Callback::new(move |_| {
        editor_open.set(true);
    });

    let on_toggle_callback = Callback::new({
        let field_id = field_id.clone();
        let on_config_change = on_config_change.clone();
        move |new_active: bool| {
            let mut cfg = config.get_untracked();
            if let Some(cond) = cfg
                .filters
                .conditions
                .iter_mut()
                .find(|c| c.field_id == field_id)
            {
                cond.active = new_active;
                on_config_change.run(cfg);
            }
        }
    });

    // Aggregate function Select value - create RwSignal that derives from config
    let agg_select_value = RwSignal::new({
        let cfg = config.get_untracked();
        cfg.selected_fields
            .iter()
            .find(|sf| sf.field_id == field_id)
            .and_then(|sf| sf.aggregate)
            .map(|agg| match agg {
                AggregateFunction::Sum => "sum".to_string(),
                AggregateFunction::Count => "count".to_string(),
                AggregateFunction::Avg => "avg".to_string(),
                AggregateFunction::Min => "min".to_string(),
                AggregateFunction::Max => "max".to_string(),
            })
            .unwrap_or_else(|| "sum".to_string())
    });

    // Update agg_select_value when config changes (from outside, like loading saved config)
    Effect::new({
        let field_id = field_id.clone();
        move |_| {
            let cfg = config.get();
            let new_value = cfg
                .selected_fields
                .iter()
                .find(|sf| sf.field_id == field_id)
                .and_then(|sf| sf.aggregate)
                .map(|agg| match agg {
                    AggregateFunction::Sum => "sum".to_string(),
                    AggregateFunction::Count => "count".to_string(),
                    AggregateFunction::Avg => "avg".to_string(),
                    AggregateFunction::Min => "min".to_string(),
                    AggregateFunction::Max => "max".to_string(),
                })
                .unwrap_or_else(|| "sum".to_string());
            let current = agg_select_value.get_untracked();
            log!(
                "[Effect 1 AGG] Field: {}, new_value: {}, current: {}",
                field_id,
                new_value,
                current
            );

            // Force update even if value is the same
            if current == new_value {
                log!("[Effect 1 AGG] FORCING UPDATE for field: {}", field_id);
                let temp_value = if new_value == "sum" { "count" } else { "sum" };
                agg_select_value.set(temp_value.to_string());
            }
            agg_select_value.set(new_value);
        }
    });

    // Update config when user changes agg_select_value
    Effect::new({
        let field_id = field_id.clone();
        let on_config_change = on_config_change.clone();
        move |prev: Option<String>| {
            let current = agg_select_value.get();

            // Skip first run (initialization)
            if prev.is_none() {
                return current.clone();
            }

            // Skip if config is being loaded (prevent loop) - use untracked to avoid dependency
            if is_loading_config.get_untracked() {
                return current.clone();
            }

            // Only process if value actually changed
            if Some(&current) == prev.as_ref() {
                return current.clone();
            }

            let agg_fn = match current.as_str() {
                "count" => AggregateFunction::Count,
                "avg" => AggregateFunction::Avg,
                "min" => AggregateFunction::Min,
                "max" => AggregateFunction::Max,
                _ => AggregateFunction::Sum,
            };

            let mut cfg = config.get_untracked();
            if let Some(field) = cfg
                .selected_fields
                .iter_mut()
                .find(|sf| sf.field_id == field_id)
            {
                field.aggregate = Some(agg_fn);
            }
            on_config_change.run(cfg);
            current
        }
    });

    // Check if field is enabled - computed from config
    let is_enabled = Memo::new({
        let field_id = field_id.clone();
        move |_| {
            let cfg = config.get();
            cfg.enabled_fields.contains(&field_id)
        }
    });

    // Handle checkbox toggle
    let on_enabled_toggle = {
        let field_id = field_id.clone();
        move |checked: bool| {
            let mut cfg = config.get_untracked();
            if checked {
                if !cfg.enabled_fields.contains(&field_id) {
                    cfg.enabled_fields.push(field_id.clone());

                    // Auto-fill default role and function
                    if can_aggregate {
                        // For numeric fields: set as Measure with SUM
                        if !cfg.selected_fields.iter().any(|sf| sf.field_id == field_id) {
                            cfg.selected_fields.push(SelectedField {
                                field_id: field_id.clone(),
                                aggregate: Some(AggregateFunction::Sum),
                            });
                        }
                    } else if can_group {
                        // For text/groupable fields: set as Display
                        if !cfg.display_fields.contains(&field_id)
                            && !cfg.groupings.contains(&field_id)
                        {
                            cfg.display_fields.push(field_id.clone());
                        }
                    }

                    on_config_change.run(cfg);
                }
            } else {
                cfg.enabled_fields.retain(|id| id != &field_id);
                on_config_change.run(cfg);
            }
        }
    };

    view! {
        <TableRow>
            <TableCell>
                <TableCellLayout>
                    <div style="display: flex; justify-content: center;">
                        <input
                            type="checkbox"
                            checked=move || is_enabled.get()
                            on:change=move |ev| {
                                let checked = event_target_checked(&ev);
                                on_enabled_toggle(checked);
                            }
                            style="cursor: pointer;"
                        />
                    </div>
                </TableCellLayout>
            </TableCell>
            <TableCell>
                <TableCellLayout>{field_name}</TableCellLayout>
            </TableCell>
            <TableCell>
                <TableCellLayout>
                    <div style="display: flex; justify-content: center;">
                        {if can_aggregate {
                            view! { <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Informative>"N"</Badge> }.into_any()
                        } else {
                            view! { <span style="color: var(--thaw-color-neutral-foreground-3);">"-"</span> }.into_any()
                        }}
                    </div>
                </TableCellLayout>
            </TableCell>
            <TableCell>
                <TableCellLayout>
                    <code>{field_id_for_code}</code>
                </TableCellLayout>
            </TableCell>
            <TableCell>
                <TableCellLayout>
                    <Select value=role_select_value size=SelectSize::Small>
                        <option value="none">"Нет"</option>
                        {can_group.then(|| {
                            view! {
                                <>
                                <option value="grouping">"Группировка"</option>
                                <option value="display">"Отображать"</option>
                                </>
                            }
                        })}

                        {can_aggregate.then(|| {
                            view! { <option value="measure">"Показатель"</option> }
                        })}

                    </Select>
                </TableCellLayout>
            </TableCell>
            <TableCell>
                <TableCellLayout>
                {move || {
                    if get_role_func.with_value(|f| f() == FieldRole::Measure) {
                        view! {
                            <Select value=agg_select_value size=SelectSize::Small>
                                <option value="sum">"SUM"</option>
                                <option value="count">"COUNT"</option>
                                <option value="avg">"AVG"</option>
                                <option value="min">"MIN"</option>
                                <option value="max">"MAX"</option>
                            </Select>
                        }
                            .into_any()
                    } else {
                        view! { <span class="text-muted">"-"</span> }.into_any()
                    }
                }}
                </TableCellLayout>
            </TableCell>
            <TableCell>
                <TableCellLayout>
                    {move || {
                        let cond = get_condition.with_value(|f| f());
                        view! {
                            <ConditionDisplay
                                condition=cond
                                on_edit=on_edit_callback
                                on_toggle=on_toggle_callback
                            />
                        }
                    }}
                    {move || {
                        let cond = get_condition.with_value(|f| f());
                        view! {
                            <ConditionEditorModal
                                open=editor_open
                                field=editing_field
                                existing_condition=cond
                                on_save=on_save_callback
                                on_delete=on_clear_callback
                            />
                        }
                    }}
                </TableCellLayout>
            </TableCell>
        </TableRow>
    }
}

