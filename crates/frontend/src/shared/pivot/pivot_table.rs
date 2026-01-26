use leptos::prelude::*;
use contracts::shared::pivot::{CellValue, ColumnHeader, ColumnType, ExecuteDashboardResponse, PivotRow};

#[component]
pub fn PivotTable(
    /// Dashboard execution response
    #[prop(into)]
    response: Signal<Option<ExecuteDashboardResponse>>,
) -> impl IntoView {
    view! {
        <div class="pivot-table-container">
            {move || {
                response
                    .get()
                    .map(|resp| {
                        view! {
                            <table class="pivot-table">
                                <thead>
                                    <tr>
                                        {resp
                                            .columns
                                            .iter()
                                            .map(|col| {
                                                let col_name = col.name.clone();
                                                let col_type = col.column_type;
                                                view! {
                                                    <th class=move || {
                                                        match col_type {
                                                            ColumnType::Grouping => "grouping-header",
                                                            ColumnType::Aggregated => "aggregated-header",
                                                        }
                                                    }>{col_name}</th>
                                                }
                                            })
                                            .collect_view()}
                                    </tr>
                                </thead>
                                <tbody>
                                    {resp
                                        .rows
                                        .iter()
                                        .map(|row| render_pivot_row(row, &resp.columns))
                                        .collect_view()}
                                </tbody>
                            </table>
                        }
                    })
            }}
        </div>
    }
}

/// Recursively render a pivot row and its children
fn render_pivot_row(row: &PivotRow, columns: &[ColumnHeader]) -> impl IntoView {
    let row_class = if row.is_total {
        format!("pivot-row level-{} total-row", row.level)
    } else {
        format!("pivot-row level-{}", row.level)
    };

    let main_row = view! {
        <tr class=row_class.clone()>
            {columns
                .iter()
                .map(|col| {
                    let value = row
                        .values
                        .get(&col.id)
                        .map(|v| format_cell_value(v))
                        .unwrap_or_default();
                    let indent_style = if col.column_type == ColumnType::Grouping {
                        format!("padding-left: {}px;", row.level * 20)
                    } else {
                        String::new()
                    };
                    view! { <td style=indent_style>{value}</td> }
                })
                .collect_view()}
        </tr>
    };

    // Render all rows (main + children) as a flat list wrapped in Fragment
    let mut all_rows = vec![main_row.into_any()];
    
    for child in &row.children {
        all_rows.push(render_pivot_row(child, columns).into_any());
    }
    
    all_rows.into_view()
}

/// Format a cell value for display
fn format_cell_value(value: &CellValue) -> String {
    match value {
        CellValue::Text(s) => s.clone(),
        CellValue::Number(n) => format!("{:.2}", n),
        CellValue::Integer(i) => i.to_string(),
        CellValue::Null => String::new(),
    }
}
