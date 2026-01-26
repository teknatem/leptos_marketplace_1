use leptos::prelude::*;
use contracts::shared::pivot::{
    AggregateFunction, DashboardConfig, DataSourceSchemaOwned,
    SelectedField,
};

#[component]
pub fn ConfigPanel(
    /// Current configuration
    #[prop(into)]
    config: Signal<DashboardConfig>,
    /// Schema for the data source
    schema: DataSourceSchemaOwned,
    /// Callback when configuration changes
    on_config_change: Callback<DashboardConfig>,
    /// Callback to execute the query
    _on_execute: Callback<()>,
) -> impl IntoView {
    // Toggle field selection
    let toggle_field = move |field_id: String, aggregate_fn: Option<AggregateFunction>| {
        let mut cfg = config.get();
        if let Some(idx) = cfg
            .selected_fields
            .iter()
            .position(|f| f.field_id == field_id)
        {
            cfg.selected_fields.remove(idx);
        } else {
            cfg.selected_fields.push(SelectedField {
                field_id: field_id.clone(),
                aggregate: aggregate_fn,
            });
        }
        on_config_change.run(cfg);
    };

    // Toggle grouping
    let toggle_grouping = move |field_id: String| {
        let mut cfg = config.get();
        if let Some(idx) = cfg.groupings.iter().position(|g| g == &field_id) {
            cfg.groupings.remove(idx);
        } else {
            cfg.groupings.push(field_id.clone());
        }
        on_config_change.run(cfg);
    };

    let grouping_fields = schema.fields.iter().filter(|f| f.can_group).count();
    let aggregate_fields = schema.fields.iter().filter(|f| f.can_aggregate).count();

    view! {
        <div class="dashboard-config-panel">
            <div class="config-header">
                <h3 class="section-title">
                    <i class="icon-settings"></i>
                    " Настройка отчета"
                </h3>
                <div class="config-summary">
                    {move || {
                        let groupings = config.get().groupings.len();
                        let fields = config.get().selected_fields.len();
                        view! {
                            <div class="summary-badges">
                                <span class="badge badge-info">{groupings.to_string()} " группировок"</span>
                                <span class="badge badge-success">
                                    {fields.to_string()}
                                    " показателей"
                                </span>
                            </div>
                        }
                    }}
                </div>
            </div>

            <div class="config-sections">
                // Группировки
                <div class="config-section">
                    <div class="section-header">
                        <h4 class="section-subtitle">
                            <i class="icon-layers"></i>
                            " Группировки"
                        </h4>
                        <span class="field-count">
                            {move || config.get().groupings.len().to_string()}
                            " / "
                            {grouping_fields.to_string()}
                        </span>
                    </div>
                    <div class="field-list">
                        {
                            schema
                                .fields
                                .iter()
                                .filter(|f| f.can_group)
                                .map(|field| {
                                    let field_id = field.id.clone();
                                    let field_id_check = field_id.clone();
                                    let field_name = field.name.clone();
                                    let is_selected = move || {
                                        config.get().groupings.contains(&field_id_check)
                                    };
                                    view! {
                                        <label class="field-item">
                                            <input
                                                type="checkbox"
                                                class="field-checkbox"
                                                checked=is_selected
                                                on:change=move |_| {
                                                    toggle_grouping(field_id.clone());
                                                }
                                            />
                                            <span class="field-label">{field_name}</span>
                                        </label>
                                    }
                                })
                                .collect_view()
                        }
                    </div>
                </div>

                // Показатели
                <div class="config-section">
                    <div class="section-header">
                        <h4 class="section-subtitle">
                            <i class="icon-bar-chart"></i>
                            " Показатели (Сумма)"
                        </h4>
                        <span class="field-count">
                            {move || config.get().selected_fields.len().to_string()}
                            " / "
                            {aggregate_fields.to_string()}
                        </span>
                    </div>
                    <div class="field-list">
                        {
                            schema
                                .fields
                                .iter()
                                .filter(|f| f.can_aggregate)
                                .map(|field| {
                                    let field_id = field.id.clone();
                                    let field_id_check = field_id.clone();
                                    let field_name = field.name.clone();
                                    let is_selected = move || {
                                        config
                                            .get()
                                            .selected_fields
                                            .iter()
                                            .any(|sf| sf.field_id == field_id_check)
                                    };
                                    view! {
                                        <label class="field-item">
                                            <input
                                                type="checkbox"
                                                class="field-checkbox"
                                                checked=is_selected
                                                on:change=move |_| {
                                                    toggle_field(
                                                        field_id.clone(),
                                                        Some(AggregateFunction::Sum),
                                                    );
                                                }
                                            />
                                            <span class="field-label">{field_name}</span>
                                        </label>
                                    }
                                })
                                .collect_view()
                        }
                    </div>
                </div>
            </div>

            // Подсказка
            <div class="config-hint">
                <i class="icon-info"></i>
                <span>"Выберите поля и нажмите \"Выполнить\" в шапке"</span>
            </div>
        </div>
    }
}
