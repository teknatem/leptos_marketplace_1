use contracts::shared::universal_dashboard::{
    CellValue, ColumnHeader, ColumnType, ExecuteDashboardResponse, PivotRow,
};
use leptos::prelude::*;
use thaw::{
    Button, ButtonAppearance, Table, TableBody, TableCell, TableCellLayout, TableHeader,
    TableHeaderCell, TableRow,
};
use wasm_bindgen::JsCast;
use web_sys::Blob;

#[component]
pub fn PivotTable(
    /// Dashboard execution response
    #[prop(into)]
    response: Signal<Option<ExecuteDashboardResponse>>,
) -> impl IntoView {
    let sort_column = RwSignal::new(None::<String>);
    let sort_ascending = RwSignal::new(true);

    // CSV Export function
    let export_csv = move |_| {
        if let Some(resp) = response.get() {
            let csv_content = generate_csv(&resp);
            download_csv(&csv_content, "dashboard_export.csv");
        }
    };

    view! {
        <div class="pivot-table-container">
            {move || {
                response.get().map(|resp| {
                    let sorted_rows = sort_rows(&resp.rows, sort_column.get(), sort_ascending.get(), &resp.columns);
                    let columns_for_header = resp.columns.clone();
                    let columns_for_body = resp.columns.clone();

                    view! {
                        <div style="margin-bottom: 12px;">
                            <Button
                                appearance=ButtonAppearance::Secondary
                                on_click=export_csv
                            >
                                "📥 Экспорт CSV"
                            </Button>
                        </div>
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    {columns_for_header
                                        .into_iter()
                                        .map(|col| {
                                            let col_id = col.id.clone();
                                            let col_id_for_check = col_id.clone();
                                            let col_id_for_set = col_id.clone();
                                            let col_name = col.name.clone();
                                            let col_type = col.column_type;

                                            view! {
                                                <TableHeaderCell>
                                                    <div
                                                        style=format!("cursor: pointer; user-select: none; display: flex; align-items: center; gap: 4px;")
                                                        on:click=move |_| {
                                                            if sort_column.get().as_ref() == Some(&col_id_for_check) {
                                                                sort_ascending.update(|a| *a = !*a);
                                                            } else {
                                                                sort_column.set(Some(col_id_for_set.clone()));
                                                                sort_ascending.set(true);
                                                            }
                                                        }
                                                    >
                                                        {col_name.clone()}
                                                        {move || {
                                                            if sort_column.get().as_ref() == Some(&col_id) {
                                                                if sort_ascending.get() { " ▲" } else { " ▼" }
                                                            } else {
                                                                ""
                                                            }
                                                        }}
                                                    </div>
                                                </TableHeaderCell>
                                            }
                                        })
                                        .collect_view()}
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                {sorted_rows
                                    .into_iter()
                                    .map(|row| render_pivot_row_thaw(&row, &columns_for_body))
                                    .collect_view()}
                            </TableBody>
                        </Table>
                    }
                })
            }}
        </div>
    }
}

/// Recursively render a pivot row and its children (Thaw version)
fn render_pivot_row_thaw(row: &PivotRow, columns: &[ColumnHeader]) -> impl IntoView {
    // Clone data for use in view
    let row_values = row.values.clone();
    let row_level = row.level;
    let row_is_total = row.is_total;
    let columns_vec: Vec<(String, String, ColumnType)> = columns
        .iter()
        .map(|col| (col.id.clone(), col.name.clone(), col.column_type))
        .collect();

    // Determine if this is a grand total row (level 0 and is_total)
    let row_is_grand_total = row_level == 0 && row_is_total;

    // Apply background color for grand total rows
    let row_style = if row_is_grand_total {
        "background-color: var(--thaw-color-neutral-background-2);"
    } else {
        ""
    };

    let main_row = view! {
        <TableRow attr:style=row_style>
            {columns_vec
                .into_iter()
                .map(|(col_id, _col_name, col_type)| {
                    let cell_value = row_values.get(&col_id);
                    let value = cell_value
                        .map(|v| format_cell_value(v))
                        .unwrap_or_default();

                    // Determine if this is a numeric cell
                    let is_numeric = matches!(cell_value, Some(CellValue::Number(_) | CellValue::Integer(_)));

                    // Also check if column type is Aggregated (numeric aggregate)
                    let is_numeric_column = col_type == ColumnType::Aggregated || is_numeric;

                    let indent_style = if col_type == ColumnType::Grouping {
                        format!("padding-left: {}px;", row_level * 20)
                    } else {
                        String::new()
                    };
                    let weight = if row_is_total { "font-weight: 600;" } else { "" };
                    let span_style = format!("{}{}", indent_style, weight);

                    // Style for TableCell to align content to the right
                    let cell_style = if is_numeric_column {
                        "justify-content: flex-end;"
                    } else {
                        ""
                    };

                    view! {
                        <TableCell>
                            <TableCellLayout attr:style=cell_style>
                                <span style=span_style>{value}</span>
                            </TableCellLayout>
                        </TableCell>
                    }
                })
                .collect_view()}
        </TableRow>
    };

    // Render all rows (main + children) as a flat list
    let mut all_rows = vec![main_row.into_any()];

    for child in &row.children {
        all_rows.push(render_pivot_row_thaw(child, columns).into_any());
    }

    all_rows.into_view()
}

/// Sort pivot rows based on column and direction
fn sort_rows(
    rows: &[PivotRow],
    sort_column: Option<String>,
    ascending: bool,
    columns: &[ColumnHeader],
) -> Vec<PivotRow> {
    if sort_column.is_none() {
        return rows.to_vec();
    }

    let col_id = sort_column.unwrap();
    let mut sorted = rows.to_vec();

    sorted.sort_by(|a, b| {
        let val_a = a.values.get(&col_id);
        let val_b = b.values.get(&col_id);

        let cmp = match (val_a, val_b) {
            (Some(CellValue::Number(na)), Some(CellValue::Number(nb))) => {
                na.partial_cmp(nb).unwrap_or(std::cmp::Ordering::Equal)
            }
            (Some(CellValue::Integer(ia)), Some(CellValue::Integer(ib))) => ia.cmp(ib),
            (Some(CellValue::Text(ta)), Some(CellValue::Text(tb))) => ta.cmp(tb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        };

        if ascending {
            cmp
        } else {
            cmp.reverse()
        }
    });

    // Recursively sort children
    sorted.iter_mut().for_each(|row| {
        row.children = sort_rows(&row.children, Some(col_id.clone()), ascending, columns);
    });

    sorted
}

/// Generate CSV content from dashboard response
fn generate_csv(response: &ExecuteDashboardResponse) -> String {
    let mut csv = String::new();

    // Header row
    let headers: Vec<String> = response
        .columns
        .iter()
        .map(|col| escape_csv_field(&col.name))
        .collect();
    csv.push_str(&headers.join(";"));
    csv.push('\n');

    // Data rows (flatten hierarchical structure)
    fn flatten_rows<'a>(rows: &'a [PivotRow], result: &mut Vec<&'a PivotRow>) {
        for row in rows {
            result.push(row);
            flatten_rows(&row.children, result);
        }
    }

    let mut flat_rows = Vec::new();
    flatten_rows(&response.rows, &mut flat_rows);

    for row in flat_rows {
        let values: Vec<String> = response
            .columns
            .iter()
            .map(|col| {
                let value = row
                    .values
                    .get(&col.id)
                    .map(|v| format_cell_value(v))
                    .unwrap_or_default();
                escape_csv_field(&value)
            })
            .collect();
        csv.push_str(&values.join(";"));
        csv.push('\n');
    }

    csv
}

/// Escape CSV field (add quotes if needed) - semicolon separator
fn escape_csv_field(field: &str) -> String {
    if field.contains(';') || field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/// Download CSV content as a file
fn download_csv(content: &str, filename: &str) {
    use wasm_bindgen::JsValue;
    use web_sys::{HtmlAnchorElement, Url};

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    // Add UTF-8 BOM for Excel compatibility
    let bom = "\u{FEFF}";
    let content_with_bom = format!("{}{}", bom, content);

    // Create blob
    let array = js_sys::Array::new();
    array.push(&JsValue::from_str(&content_with_bom));

    let options = web_sys::BlobPropertyBag::new();
    options.set_type("text/csv;charset=utf-8;");

    let blob = Blob::new_with_str_sequence_and_options(&array, &options).unwrap();

    // Create download link
    let url = Url::create_object_url_with_blob(&blob).unwrap();
    let anchor = document
        .create_element("a")
        .unwrap()
        .dyn_into::<HtmlAnchorElement>()
        .unwrap();
    anchor.set_href(&url);
    anchor.set_download(filename);
    anchor.click();

    // Cleanup
    Url::revoke_object_url(&url).ok();
}

/// Format a cell value for display with thousand separators
fn format_cell_value(value: &CellValue) -> String {
    match value {
        CellValue::Text(s) => s.clone(),
        CellValue::Number(n) => format_number(*n),
        CellValue::Integer(i) => format_integer(*i),
        CellValue::Null => String::new(),
    }
}

/// Format a number with thousand separators and 2 decimal places
fn format_number(n: f64) -> String {
    let abs_n = n.abs();
    let integer_part = abs_n.floor() as i64;
    let decimal_part = ((abs_n - abs_n.floor()) * 100.0).round() as i64;

    let formatted_integer = format_integer(integer_part);
    let sign = if n < 0.0 { "-" } else { "" };

    format!("{}{}.{:02}", sign, formatted_integer, decimal_part)
}

/// Format an integer with thousand separators
fn format_integer(n: i64) -> String {
    let s = n.abs().to_string();
    let mut result = String::new();
    let len = s.len();

    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(' ');
        }
        result.push(ch);
    }

    if n < 0 {
        format!("-{}", result)
    } else {
        result
    }
}
