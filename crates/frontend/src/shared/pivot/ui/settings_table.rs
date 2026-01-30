use leptos::prelude::*;
use leptos::ev::{Event, FocusEvent};
use leptos::task::spawn_local;
use leptos::logging::log;
use thaw::*;
use wasm_bindgen::JsCast;
use contracts::shared::pivot::{
    AggregateFunction, DashboardConfig, DataSourceSchemaOwned, DistinctValue, FieldDefOwned,
    FieldFilter, FieldRole, FieldType, FilterOperator, SelectedField,
};
use crate::shared::pivot::api;

#[component]
pub fn SettingsTable(
    /// Current configuration
    #[prop(into)]
    config: Signal<DashboardConfig>,
    /// Schema for the data source
    schema: DataSourceSchemaOwned,
    /// Callback when configuration changes
    on_config_change: Callback<DashboardConfig>,
    /// Optional callback to save configuration
    #[prop(optional)]
    on_save: Option<Callback<()>>,
) -> impl IntoView {
    let show_all = RwSignal::new(true);
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

    // Filter and sort fields
    let filtered_fields = move || {
        let mut fields = schema.fields.clone();
        
        // Filter if needed
        if !show_all.get() {
            let cfg = config.get();
            fields.retain(|field| {
                // Show field if it has a role or a filter
                cfg.groupings.contains(&field.id) ||
                cfg.display_fields.contains(&field.id) ||
                cfg.selected_fields.iter().any(|sf| sf.field_id == field.id) ||
                cfg.filters.field_filters.iter().any(|f| f.field_id == field.id)
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
            if ascending { cmp } else { cmp.reverse() }
        });
        
        fields
    };

    view! {
        <div class="settings-table-wrapper">
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center style="margin-bottom: 16px;">
                <Space>
                    <Button
                        appearance=move || if show_all.get() { ButtonAppearance::Primary } else { ButtonAppearance::Secondary }
                        on_click=move |_| show_all.set(true)
                    >
                        "Все"
                    </Button>
                    <Button
                        appearance=move || if !show_all.get() { ButtonAppearance::Primary } else { ButtonAppearance::Secondary }
                        on_click=move |_| show_all.set(false)
                    >
                        "Выбранные"
                    </Button>
                </Space>
                {move || on_save.as_ref().map(|save_cb| {
                    let save_cb = *save_cb;
                    view! {
                        <Button
                            appearance=ButtonAppearance::Secondary
                            on_click=move |_| save_cb.run(())
                        >
                            " Сохранить"
                        </Button>
                    }
                })}
            </Flex>
            <Table>
                <TableHeader>
                    <TableRow>
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
                        <TableHeaderCell min_width=120.0>"Фильтр"</TableHeaderCell>
                        <TableHeaderCell min_width=150.0>"Значение"</TableHeaderCell>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {move || {
                        filtered_fields()
                            .into_iter()
                            .map(|field| {
                                view! {
                                    <FieldRow
                                        field=field
                                        config=config
                                        on_config_change=on_config_change
                                    />
                                }
                            })
                            .collect_view()
                    }}
                </TableBody>
            </Table>
        </div>
    }
}

#[component]
fn FieldRow(
    field: FieldDefOwned,
    #[prop(into)] config: Signal<DashboardConfig>,
    on_config_change: Callback<DashboardConfig>,
) -> impl IntoView {
    let field_id = field.id.clone();
    let field_id_for_code = field_id.clone();
    let field_id_for_agg = field_id.clone();
    let field_name = field.name.clone();
    let field_type = field.field_type;
    let can_group = field.can_group;
    let can_aggregate = field.can_aggregate;

    // State for distinct values (lazy loading)
    let distinct_values = RwSignal::new(Vec::<DistinctValue>::new());
    let values_loading = RwSignal::new(false);
    let values_loaded = RwSignal::new(false);

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


    // Get current filter
    let get_filter = {
        let field_id = field_id.clone();
        move || -> Option<FieldFilter> {
            let cfg = config.get();
            cfg.filters
                .field_filters
                .iter()
                .find(|f| f.field_id == field_id)
                .cloned()
        }
    };

    // Handler for role change
    let on_role_change = {
        let field_id = field_id.clone();
        move |ev: Event| {
            let value = event_target_value(&ev);
            let role = match value.as_str() {
                "grouping" => FieldRole::Grouping,
                "display" => FieldRole::Display,
                "measure" => FieldRole::Measure,
                _ => FieldRole::None,
            };

            let mut cfg = config.get();
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
        }
    };

    // Handler for aggregate function change (stored for multi-use in reactive context)
    let on_aggregate_change = StoredValue::new({
        let field_id = field_id.clone();
        let on_config_change = on_config_change.clone();
        move |ev: Event| {
            let value = event_target_value(&ev);
            let agg_fn = match value.as_str() {
                "count" => AggregateFunction::Count,
                "avg" => AggregateFunction::Avg,
                "min" => AggregateFunction::Min,
                "max" => AggregateFunction::Max,
                _ => AggregateFunction::Sum,
            };

            let mut cfg = config.get();
            if let Some(field) = cfg.selected_fields.iter_mut().find(|sf| sf.field_id == field_id) {
                field.aggregate = Some(agg_fn);
            }
            on_config_change.run(cfg);
        }
    });

    // Handler for filter operator change
    let on_filter_op_change = {
        let field_id = field_id.clone();
        move |ev: Event| {
            let value = event_target_value(&ev);
            let operator = match value.as_str() {
                "noteq" => FilterOperator::NotEq,
                "lt" => FilterOperator::Lt,
                "gt" => FilterOperator::Gt,
                "lteq" => FilterOperator::LtEq,
                "gteq" => FilterOperator::GtEq,
                "like" => FilterOperator::Like,
                "in" => FilterOperator::In,
                "between" => FilterOperator::Between,
                "isnull" => FilterOperator::IsNull,
                _ => FilterOperator::Eq,
            };

            let mut cfg = config.get();
            if let Some(filter) = cfg
                .filters
                .field_filters
                .iter_mut()
                .find(|f| f.field_id == field_id)
            {
                filter.operator = operator;
            } else {
                cfg.filters.field_filters.push(FieldFilter {
                    field_id: field_id.clone(),
                    operator,
                    value: String::new(),
                    value2: None,
                });
            }
            on_config_change.run(cfg);
        }
    };

    // Handler for filter value change (stored for multi-use)
    let on_filter_value_change = StoredValue::new({
        let field_id = field_id.clone();
        let on_config_change = on_config_change.clone();
        move |ev: Event| {
            let value = event_target_value(&ev);

            let mut cfg = config.get();
            if value.is_empty() {
                cfg.filters.field_filters.retain(|f| f.field_id != field_id);
            } else {
                if let Some(filter) = cfg
                    .filters
                    .field_filters
                    .iter_mut()
                    .find(|f| f.field_id == field_id)
                {
                    filter.value = value;
                } else {
                    cfg.filters.field_filters.push(FieldFilter {
                        field_id: field_id.clone(),
                        operator: FilterOperator::Eq,
                        value,
                        value2: None,
                    });
                }
            }
            on_config_change.run(cfg);
        }
    });

    // Handler for lazy loading distinct values on focus (stored for multi-use)
    let on_filter_focus = StoredValue::new({
        let field_id = field_id.clone();
        move |_ev: FocusEvent| {
            // Only load for text fields (not numeric, not date)
            if values_loaded.get()
                || values_loading.get()
                || matches!(
                    field_type,
                    FieldType::Numeric | FieldType::Integer | FieldType::Date
                )
            {
                return;
            }

            values_loading.set(true);
            let field_id_clone = field_id.clone();
            let schema_id = config.get().data_source.clone();

            spawn_local(async move {
                match api::get_distinct_values(&schema_id, &field_id_clone).await {
                    Ok(resp) => {
                        distinct_values.set(resp.values);
                        values_loaded.set(true);
                        values_loading.set(false);
                    }
                    Err(e) => {
                        log!("Failed to load distinct values for {}: {}", field_id_clone, e);
                        values_loading.set(false);
                    }
                }
            });
        }
    });

    view! {
        <TableRow>
            <TableCell>
                <TableCellLayout>{field_name}</TableCellLayout>
            </TableCell>
            <TableCell>
                <TableCellLayout>
                    <code>{field_id_for_code}</code>
                </TableCellLayout>
            </TableCell>
            <TableCell>
                <TableCellLayout>
                    <select class="role-select" on:change=on_role_change>
                        <option value="none" selected={move || get_role_func.with_value(|f| f() == FieldRole::None)}>"Нет"</option>
                        {can_group.then(|| {
                            view! {
                                <>
                                <option value="grouping" selected={move || get_role_func.with_value(|f| f() == FieldRole::Grouping)}>"Группировка"</option>
                                <option value="display" selected={move || get_role_func.with_value(|f| f() == FieldRole::Display)}>"Отображать"</option>
                                </>
                            }
                        })}

                        {can_aggregate.then(|| {
                            view! { <option value="measure" selected={move || get_role_func.with_value(|f| f() == FieldRole::Measure)}>"Показатель"</option> }
                        })}

                    </select>
                </TableCellLayout>
            </TableCell>
            <TableCell>
                <TableCellLayout>
                {move || {
                    if get_role_func.with_value(|f| f() == FieldRole::Measure) {
                        let current_agg = config.get()
                            .selected_fields
                            .iter()
                            .find(|sf| sf.field_id == field_id_for_agg)
                            .and_then(|sf| sf.aggregate);

                        let sum_sel = current_agg == Some(AggregateFunction::Sum);
                        let count_sel = current_agg == Some(AggregateFunction::Count);
                        let avg_sel = current_agg == Some(AggregateFunction::Avg);
                        let min_sel = current_agg == Some(AggregateFunction::Min);
                        let max_sel = current_agg == Some(AggregateFunction::Max);

                        let handler = move |ev: Event| {
                            on_aggregate_change.with_value(|h| h(ev));
                        };

                        view! {
                            <select class="function-select" on:change=handler>
                                <option value="sum" selected=sum_sel>
                                    "SUM"
                                </option>
                                <option value="count" selected=count_sel>
                                    "COUNT"
                                </option>
                                <option value="avg" selected=avg_sel>
                                    "AVG"
                                </option>
                                <option value="min" selected=min_sel>
                                    "MIN"
                                </option>
                                <option value="max" selected=max_sel>
                                    "MAX"
                                </option>
                            </select>
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
                    <select class="filter-select" on:change=on_filter_op_change>
                        <option value="eq">"="</option>
                        <option value="noteq">"≠"</option>
                        <option value="lt">"<"</option>
                        <option value="gt">">"</option>
                        <option value="lteq">"≤"</option>
                        <option value="gteq">"≥"</option>
                        <option value="like">"содержит"</option>
                        <option value="in">"в списке"</option>
                        <option value="between">"между"</option>
                        <option value="isnull">"пусто"</option>
                    </select>
                </TableCellLayout>
            </TableCell>
            <TableCell>
                <TableCellLayout>
                {move || {
                    let is_text_field = matches!(field_type, FieldType::Text);
                    let has_values = !distinct_values.get().is_empty();

                    if is_text_field && has_values {
                        let current_value = get_filter().map(|f| f.value).unwrap_or_default();
                        let values = distinct_values.get();
                        let change_handler = move |ev: Event| {
                            on_filter_value_change.with_value(|h| h(ev));
                        };
                        
                        view! {
                            <select class="filter-value-select" on:change=change_handler>
                                <option value="">"-- Выберите значение --"</option>
                                {values
                                    .into_iter()
                                    .map(|dv| {
                                        let is_selected = dv.value == current_value;
                                        let val = dv.value.clone();
                                        let disp = dv.display.clone();
                                        view! { <option value=val selected=is_selected>{disp}</option> }
                                    })
                                    .collect_view()}

                            </select>
                        }
                            .into_any()
                    } else {
                        let filter_value = get_filter().map(|f| f.value).unwrap_or_default();
                        let input_handler = move |ev: Event| {
                            on_filter_value_change.with_value(|h| h(ev));
                        };
                        let focus_handler = move |ev: FocusEvent| {
                            on_filter_focus.with_value(|h| h(ev));
                        };
                        
                        view! {
                            <input
                                type="text"
                                class="filter-input"
                                placeholder="Значение"
                                value=filter_value
                                on:input=input_handler
                                on:focus=focus_handler
                            />
                        }
                            .into_any()
                    }
                }}
                </TableCellLayout>
            </TableCell>
        </TableRow>
    }
}

/// Helper to get event target value
fn event_target_value(ev: &Event) -> String {
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|input| input.value())
        .or_else(|| {
            ev.target()
                .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
                .map(|select| select.value())
        })
        .unwrap_or_default()
}
