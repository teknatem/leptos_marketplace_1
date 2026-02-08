use contracts::shared::universal_dashboard::GenerateSqlResponse;
use leptos::prelude::*;
use thaw::*;

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
                            <Flex vertical=false gap=FlexGap::Large>
                                <Button
                                    size=ButtonSize::Small
                                    appearance=ButtonAppearance::Secondary
                                    on_click=move |_| {
                                        if let Some(window) = web_sys::window() {
                                            let nav = window.navigator().clipboard();
                                            // Generate formatted SQL and convert to plain text
                                            let formatted_sql = highlight_sql(&sql_text);
                                            let plain_text_sql = html_to_plain_text(&formatted_sql);
                                            let _ = nav.write_text(&plain_text_sql);
                                        }
                                    }
                                >
                                    "📋 Копировать"
                                </Button>
                            </Flex>

                            <div class="sql-query-section">
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

    // Step 2: Highlight string literals (before keywords to avoid conflicts)
    result = highlight_strings(&result);

    // Step 3: Highlight keywords and functions
    let keywords = [
        ("SELECT", "sql-keyword"),
        ("FROM", "sql-keyword"),
        ("WHERE", "sql-keyword"),
        ("GROUP BY", "sql-keyword"),
        ("ORDER BY", "sql-keyword"),
        ("LEFT JOIN", "sql-keyword"),
        ("INNER JOIN", "sql-keyword"),
        ("RIGHT JOIN", "sql-keyword"),
        ("FULL JOIN", "sql-keyword"),
        ("CROSS JOIN", "sql-keyword"),
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
        ("OFFSET", "sql-keyword"),
        ("CASE", "sql-keyword"),
        ("WHEN", "sql-keyword"),
        ("THEN", "sql-keyword"),
        ("ELSE", "sql-keyword"),
        ("END", "sql-keyword"),
        ("SUM", "sql-function"),
        ("COUNT", "sql-function"),
        ("AVG", "sql-function"),
        ("MIN", "sql-function"),
        ("MAX", "sql-function"),
        ("COALESCE", "sql-function"),
        ("CAST", "sql-function"),
        ("UPPER", "sql-function"),
        ("LOWER", "sql-function"),
    ];

    for (keyword, class) in &keywords {
        let highlighted = format!("<span class=\"{}\">{}</span>", class, keyword);
        result = result.replace(&format!(" {} ", keyword), &format!(" {} ", highlighted));
        result = result.replace(&format!(" {}(", keyword), &format!(" {}(", highlighted));
        result = result.replace(
            &format!("<br/>{}", keyword),
            &format!("<br/>{}", highlighted),
        );
        result = result.replace(
            &format!("&nbsp;&nbsp;{}", keyword),
            &format!("&nbsp;&nbsp;{}", highlighted),
        );

        // Handle keyword at start of query
        if result.starts_with(keyword) {
            result = format!("{}{}", highlighted, &result[keyword.len()..]);
        }
    }

    // Step 4: Highlight numbers (disabled)
    // result = highlight_numbers(&result);

    // Step 5: Highlight predefined identifiers (table names, schema names)
    result = highlight_identifiers(&result);

    result
}

/// Highlight string literals in SQL
fn highlight_strings(sql: &str) -> String {
    let mut result = String::new();
    let mut in_string = false;
    let mut prev_char = ' ';

    for ch in sql.chars() {
        if ch == '\'' && prev_char != '\\' {
            if in_string {
                result.push_str("'</span>");
                in_string = false;
            } else {
                result.push_str("<span class=\"sql-string\">'");
                in_string = true;
            }
        } else {
            result.push(ch);
        }
        prev_char = ch;
    }

    // Close unclosed string
    if in_string {
        result.push_str("</span>");
    }

    result
}

/// Highlight numbers in SQL (currently disabled)
#[allow(dead_code)]
fn highlight_numbers(sql: &str) -> String {
    // Simple approach: find standalone numbers
    let mut result = String::new();
    let mut chars = sql.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch.is_numeric() {
            let mut number = String::new();
            number.push(ch);

            // Collect rest of number
            while let Some(&next_ch) = chars.peek() {
                if next_ch.is_numeric() || next_ch == '.' {
                    number.push(chars.next().unwrap());
                } else {
                    break;
                }
            }

            result.push_str(&format!("<span class=\"sql-number\">{}</span>", number));
        } else {
            result.push(ch);
        }
    }

    result
}

/// Highlight predefined identifiers (table names, schema names, special patterns)
fn highlight_identifiers(sql: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = sql.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Skip already highlighted content (both opening and closing span tags)
        if i + 5 < len && &chars[i..i + 5].iter().collect::<String>() == "<span" {
            // Copy until closing >
            while i < len {
                result.push(chars[i]);
                if chars[i] == '>' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Skip closing span tags
        if i + 6 < len && &chars[i..i + 6].iter().collect::<String>() == "</span" {
            // Copy until closing >
            while i < len {
                result.push(chars[i]);
                if chars[i] == '>' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Skip HTML entities (like &nbsp;, &lt;, &gt;, etc.)
        if chars[i] == '&' {
            result.push(chars[i]);
            i += 1;
            // Copy until semicolon
            while i < len && chars[i] != ';' {
                result.push(chars[i]);
                i += 1;
            }
            if i < len && chars[i] == ';' {
                result.push(chars[i]);
                i += 1;
            }
            continue;
        }

        // Check for identifier (starts with letter or underscore)
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let mut identifier = String::new();

            // Collect identifier characters
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                identifier.push(chars[i]);
                i += 1;
            }

            // Check if it matches pattern LNNN_ (e.g., L001_, L123_)
            let is_special_pattern = identifier.len() >= 5
                && identifier.starts_with('L')
                && identifier.chars().skip(1).take(3).all(|c| c.is_numeric())
                && identifier.ends_with('_');

            // Highlight if it's a special pattern or looks like a table/schema name
            // (contains underscore or starts with lowercase letter, except SQL keywords)
            let is_likely_identifier = identifier.contains('_')
                || (identifier.chars().next().unwrap().is_lowercase() && identifier.len() > 3);

            if is_special_pattern {
                result.push_str(&format!(
                    "<span class=\"sql-identifier-special\">{}</span>",
                    identifier
                ));
            } else if is_likely_identifier && !is_keyword(&identifier) {
                result.push_str(&format!(
                    "<span class=\"sql-identifier\">{}</span>",
                    identifier
                ));
            } else {
                result.push_str(&identifier);
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Check if a string is a SQL keyword (to avoid highlighting keywords as identifiers)
fn is_keyword(s: &str) -> bool {
    let keywords = [
        "select", "from", "where", "group", "order", "by", "left", "join", "inner", "right", "on",
        "and", "or", "as", "in", "between", "is", "null", "not", "like", "distinct", "limit",
        "offset", "case", "when", "then", "else", "end", "sum", "count", "avg", "min", "max",
        "coalesce", "cast", "upper", "lower",
    ];
    keywords.contains(&s.to_lowercase().as_str())
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

/// Convert HTML formatted SQL to plain text with line breaks preserved
fn html_to_plain_text(html: &str) -> String {
    let result = html
        .replace("<br/>", "\n")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");

    // Remove HTML tags (for highlighting spans)
    let mut clean = String::new();
    let mut in_tag = false;

    for ch in result.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            clean.push(ch);
        }
    }

    clean
}
