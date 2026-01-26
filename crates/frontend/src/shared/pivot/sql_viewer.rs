use leptos::prelude::*;
use contracts::shared::pivot::GenerateSqlResponse;

#[component]
pub fn SqlViewer(
    /// SQL response signal
    #[prop(into)]
    sql: Signal<Option<GenerateSqlResponse>>,
) -> impl IntoView {
    view! {
        <div class="sql-viewer-container">
            {move || {
                if let Some(response) = sql.get() {
                    let sql_text = response.sql.clone();
                    let sql_for_highlight = response.sql.clone();
                    let params = response.params.clone();
                    
                    view! {
                        <div class="sql-content">
                            <div class="sql-header-actions">
                                <button
                                    class="btn btn-sm btn-outline"
                                    on:click=move |_| {
                                        if let Some(window) = web_sys::window() {
                                            let nav = window.navigator().clipboard();
                                            let _ = nav.write_text(&sql_text);
                                        }
                                    }

                                    title="Копировать SQL в буфер обмена"
                                >
                                    "📋 Копировать"
                                </button>
                            </div>

                            <div class="sql-query-section">
                                <h3 class="sql-section-title">"SQL запрос"</h3>
                                <div class="sql-query" inner_html=highlight_sql(&sql_for_highlight)></div>
                            </div>

                            {if !params.is_empty() {
                                view! {
                                    <div class="sql-params-section">
                                        <h3 class="sql-section-title">"Параметры"</h3>
                                        <div class="sql-params">
                                            {params
                                                .iter()
                                                .enumerate()
                                                .map(|(i, param)| {
                                                    let param_val = param.clone();
                                                    view! {
                                                        <div class="sql-param">
                                                            <span class="param-index">{format!("${}", i + 1)}</span>
                                                            <span class="param-value">{param_val}</span>
                                                        </div>
                                                    }
                                                })
                                                .collect_view()}

                                        </div>
                                    </div>
                                }
                                    .into_any()
                            } else {
                                view! {}.into_any()
                            }}

                        </div>
                    }
                        .into_any()
                } else {
                    view! {
                        <div class="sql-placeholder">
                            <p class="text-muted">"SQL запрос будет сгенерирован автоматически"</p>
                        </div>
                    }
                        .into_any()
                }
            }}

        </div>
    }
}

/// Highlight SQL keywords and structure with proper formatting
fn highlight_sql(sql: &str) -> String {
    let mut result = html_escape(sql);

    // Step 1: Add line breaks and indentation for major SQL clauses
    result = format_sql_structure(&result);

    // Step 2: Highlight keywords and functions
    let keywords = [
        ("SELECT", "sql-keyword"),
        ("FROM", "sql-keyword"),
        ("WHERE", "sql-keyword"),
        ("GROUP BY", "sql-keyword"),
        ("ORDER BY", "sql-keyword"),
        ("LEFT JOIN", "sql-keyword"),
        ("INNER JOIN", "sql-keyword"),
        ("RIGHT JOIN", "sql-keyword"),
        ("ON", "sql-keyword"),
        ("AND", "sql-keyword"),
        ("OR", "sql-keyword"),
        ("AS", "sql-keyword"),
        ("IN", "sql-keyword"),
        ("BETWEEN", "sql-keyword"),
        ("IS", "sql-keyword"),
        ("NULL", "sql-keyword"),
        ("NOT", "sql-keyword"),
        ("LIKE", "sql-keyword"),
        ("DISTINCT", "sql-keyword"),
        ("LIMIT", "sql-keyword"),
        ("SUM", "sql-function"),
        ("COUNT", "sql-function"),
        ("AVG", "sql-function"),
        ("MIN", "sql-function"),
        ("MAX", "sql-function"),
    ];

    for (keyword, class) in &keywords {
        let highlighted = format!("<span class=\"{}\">{}</span>", class, keyword);
        result = result.replace(&format!(" {} ", keyword), &format!(" {} ", highlighted));
        result = result.replace(&format!(" {}(", keyword), &format!(" {}(", highlighted));
        result = result.replace(&format!("<br/>{}", keyword), &format!("<br/>{}", highlighted));
        result = result.replace(&format!("&nbsp;&nbsp;{}", keyword), &format!("&nbsp;&nbsp;{}", highlighted));
        
        // Handle keyword at start of query
        if result.starts_with(keyword) {
            result = format!("{}{}", highlighted, &result[keyword.len()..]);
        }
    }

    result
}

/// Format SQL structure with line breaks and indentation
fn format_sql_structure(sql: &str) -> String {
    let mut result = sql.to_string();

    // Main clauses on new lines (no indent)
    result = result.replace(" SELECT ", "<br/>SELECT<br/>&nbsp;&nbsp;");
    result = result.replace(" FROM ", "<br/>FROM<br/>&nbsp;&nbsp;");
    result = result.replace(" WHERE ", "<br/>WHERE<br/>&nbsp;&nbsp;");
    result = result.replace(" GROUP BY ", "<br/>GROUP BY<br/>&nbsp;&nbsp;");
    result = result.replace(" ORDER BY ", "<br/>ORDER BY<br/>&nbsp;&nbsp;");
    result = result.replace(" LIMIT ", "<br/>LIMIT ");

    // Handle SELECT keyword at start
    if result.starts_with("SELECT ") {
        result = format!("SELECT<br/>&nbsp;&nbsp;{}", &result[7..]);
    }

    // Format column lists (commas in SELECT and GROUP BY)
    result = result.replace(", ", ",<br/>&nbsp;&nbsp;");

    // JOINs with indentation
    result = result.replace(" LEFT JOIN ", "<br/>LEFT JOIN<br/>&nbsp;&nbsp;");
    result = result.replace(" INNER JOIN ", "<br/>INNER JOIN<br/>&nbsp;&nbsp;");
    result = result.replace(" RIGHT JOIN ", "<br/>RIGHT JOIN<br/>&nbsp;&nbsp;");
    
    // ON clause for joins (extra indent)
    result = result.replace(" ON ", "<br/>&nbsp;&nbsp;&nbsp;&nbsp;ON ");

    // AND/OR in WHERE clause (keep indent)
    result = result.replace(" AND ", "<br/>&nbsp;&nbsp;AND ");
    result = result.replace(" OR ", "<br/>&nbsp;&nbsp;OR ");

    result
}

/// Simple HTML escape
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
