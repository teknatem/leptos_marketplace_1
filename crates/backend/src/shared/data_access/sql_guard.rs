use sqlparser::ast::{
    Expr, FunctionArg, FunctionArgExpr, FunctionArguments, ObjectName, Query, Select, SelectItem,
    Statement, TableFactor, Visit, Visitor,
};
use sqlparser::dialect::SQLiteDialect;
use sqlparser::parser::Parser;
use std::collections::HashSet;
use std::ops::ControlFlow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadQueryInfo {
    pub tables: Vec<String>,
}

// SQLite table-valued JSON functions. They are safe pseudo-relations (they read
// only the JSON column passed as their argument; any nested subquery in that argument
// is still visited and enforced separately). Excluded from the enforced table list so
// callers can unpack `lines_json`/`*_json` blobs via `json_each(...)` / `json_tree(...)`.
const JSON_TABLE_FUNCTIONS: &[&str] = &["json_each", "json_tree"];

const SENSITIVE_FIELD_NAMES: &[&str] = &[
    "api_key",
    "api_key_stats",
    "password",
    "access_token",
    "refresh_token",
    "client_secret",
    "private_key",
    "secret",
];

fn enforce_sensitive_field_policy(
    visitor: &RelationVisitor,
    tables: &[String],
) -> Result<(), String> {
    let mut blocked: Vec<&str> = SENSITIVE_FIELD_NAMES
        .iter()
        .copied()
        .filter(|field| visitor.referenced_fields.contains(*field))
        .collect();
    blocked.sort_unstable();
    if !blocked.is_empty() {
        return Err(format!(
            "SQL access to protected field(s) is not allowed: {}",
            blocked.join(", ")
        ));
    }

    // a006 contains credentials. Explicit safe columns and COUNT(*) remain available,
    // but a wildcard could expose every credential column.
    if tables.iter().any(|table| table == "a006_connection_mp") {
        if visitor.has_select_wildcard || visitor.has_disallowed_function_wildcard {
            return Err(
                "SELECT * / table.* is not allowed for a006_connection_mp; list safe fields explicitly"
                    .to_string(),
            );
        }
    }
    Ok(())
}

#[derive(Default)]
struct RelationVisitor {
    relations: HashSet<String>,
    cte_names: HashSet<String>,
    unsupported_table_factor: Option<String>,
    referenced_fields: HashSet<String>,
    has_select_wildcard: bool,
    has_disallowed_function_wildcard: bool,
}

impl Visitor for RelationVisitor {
    type Break = ();

    fn pre_visit_query(&mut self, query: &Query) -> ControlFlow<Self::Break> {
        if let Some(with) = &query.with {
            for cte in &with.cte_tables {
                self.cte_names
                    .insert(cte.alias.name.value.to_ascii_lowercase());
            }
        }
        ControlFlow::Continue(())
    }

    fn pre_visit_relation(&mut self, relation: &ObjectName) -> ControlFlow<Self::Break> {
        if let Some(name) = normalize_relation_name(&relation.to_string()) {
            self.relations.insert(name);
        }
        ControlFlow::Continue(())
    }

    fn pre_visit_select(&mut self, select: &Select) -> ControlFlow<Self::Break> {
        self.has_select_wildcard |= select.projection.iter().any(|item| {
            matches!(
                item,
                SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _)
            )
        });
        ControlFlow::Continue(())
    }

    fn pre_visit_expr(&mut self, expr: &Expr) -> ControlFlow<Self::Break> {
        if let Expr::Function(function) = expr {
            if let FunctionArguments::List(args) = &function.args {
                let is_wildcard = |arg: &FunctionArg| {
                    matches!(
                        arg,
                        FunctionArg::Unnamed(
                            FunctionArgExpr::Wildcard
                                | FunctionArgExpr::WildcardWithOptions(_)
                                | FunctionArgExpr::QualifiedWildcard(_)
                        )
                    )
                };
                let contains_wildcard = args.args.iter().any(is_wildcard);
                let is_plain_count_star = function.name.to_string().eq_ignore_ascii_case("count")
                    && args.args.len() == 1
                    && matches!(
                        args.args.first(),
                        Some(FunctionArg::Unnamed(FunctionArgExpr::Wildcard))
                    );
                self.has_disallowed_function_wildcard |= contains_wildcard && !is_plain_count_star;
            }
        }
        let field = match expr {
            Expr::Identifier(identifier) => Some(identifier.value.as_str()),
            Expr::CompoundIdentifier(identifiers) => identifiers
                .last()
                .map(|identifier| identifier.value.as_str()),
            _ => None,
        };
        if let Some(field) = field {
            self.referenced_fields.insert(field.to_ascii_lowercase());
        }
        ControlFlow::Continue(())
    }

    fn pre_visit_table_factor(&mut self, factor: &TableFactor) -> ControlFlow<Self::Break> {
        match factor {
            TableFactor::Table { .. }
            | TableFactor::Derived { .. }
            | TableFactor::NestedJoin { .. } => {}
            other => self.unsupported_table_factor = Some(other.to_string()),
        }
        ControlFlow::Continue(())
    }
}

fn normalize_relation_name(raw: &str) -> Option<String> {
    let name = raw
        .split('.')
        .next_back()?
        .trim()
        .trim_matches(|c| matches!(c, '"' | '`' | '[' | ']'))
        .to_ascii_lowercase();
    if name.is_empty()
        || name
            .chars()
            .any(|c| !(c.is_ascii_alphanumeric() || c == '_'))
    {
        None
    } else {
        Some(name)
    }
}

pub fn inspect_read_query(sql: &str) -> Result<ReadQueryInfo, String> {
    let sql = sql.trim();
    if sql.is_empty() {
        return Err("SQL query is empty".to_string());
    }
    if sql.contains("--") || sql.contains("/*") || sql.contains("*/") {
        return Err("SQL comments are not allowed in read queries".to_string());
    }

    let statements = Parser::parse_sql(&SQLiteDialect {}, sql)
        .map_err(|error| format!("Invalid SQLite query: {error}"))?;
    if statements.len() != 1 {
        return Err("Exactly one SQL statement is allowed".to_string());
    }
    if !matches!(statements.first(), Some(Statement::Query(_))) {
        return Err("Only SELECT/WITH queries are allowed".to_string());
    }

    let mut visitor = RelationVisitor::default();
    let _ = statements.visit(&mut visitor);
    if let Some(factor) = &visitor.unsupported_table_factor {
        return Err(format!("Unsupported table expression: {factor}"));
    }

    let mut tables: Vec<String> = visitor
        .relations
        .difference(&visitor.cte_names)
        .filter(|name| !JSON_TABLE_FUNCTIONS.contains(&name.as_str()))
        .cloned()
        .collect();
    tables.sort();
    enforce_sensitive_field_policy(&visitor, &tables)?;
    Ok(ReadQueryInfo { tables })
}

pub fn wrap_limited_sql(sql: &str, row_limit: usize, alias: &str) -> String {
    let statement = sql.trim().trim_end_matches(';').trim();
    format!("SELECT * FROM ({statement}) AS {alias} LIMIT {row_limit}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_nested_and_cte_tables() {
        let info = inspect_read_query(
            "WITH sales AS (SELECT * FROM p904_sales_data) \
             SELECT * FROM sales JOIN a004_nomenclature n ON n.id = sales.nomenclature_ref",
        )
        .unwrap();
        assert_eq!(
            info.tables,
            vec![
                "a004_nomenclature".to_string(),
                "p904_sales_data".to_string()
            ]
        );
    }

    #[test]
    fn rejects_comments_multiple_statements_and_writes() {
        assert!(inspect_read_query("SELECT 1 -- hidden").is_err());
        assert!(inspect_read_query("SELECT 1; SELECT 2").is_err());
        assert!(inspect_read_query("PRAGMA table_info(a006_connection_mp)").is_err());
        assert!(inspect_read_query("DELETE FROM p904_sales_data").is_err());
    }

    #[test]
    fn permits_select_without_tables() {
        assert_eq!(
            inspect_read_query("SELECT 1").unwrap().tables,
            Vec::<String>::new()
        );
    }

    #[test]
    fn accepts_sqlite_weekday_query_used_by_live_wb_chart() {
        let sql = "SELECT ((CAST(strftime('%w', sale_date) AS INTEGER) + 6) % 7) + 1 AS weekday, \
                   SUM(COALESCE(total_price, 0)) AS sales_amount \
                   FROM a012_wb_sales \
                   WHERE is_deleted = 0 AND substr(sale_date, 1, 10) BETWEEN ? AND ? \
                   GROUP BY 1 ORDER BY 1";
        let info = inspect_read_query(sql).expect("weekday query must pass SQL guard");
        assert_eq!(info.tables, vec!["a012_wb_sales"]);
    }

    #[test]
    fn allows_json_each_unpacking_lines_json() {
        // Per-nomenclature detail (a036/a037) lives in lines_json; json_each must be
        // permitted as a safe table-valued function and must NOT appear as an enforced table.
        let info = inspect_read_query(
            "SELECT json_extract(j.value, '$.nm_id') AS nm_id, \
             SUM(json_extract(j.value, '$.cart_count')) AS cart \
             FROM a036_wb_sales_funnel_daily d, json_each(d.lines_json) j \
             WHERE d.is_deleted = 0 GROUP BY 1",
        )
        .expect("json_each query must pass SQL guard");
        assert_eq!(info.tables, vec!["a036_wb_sales_funnel_daily".to_string()]);

        let info = inspect_read_query(
            "SELECT d.document_date, json_extract(j.value, '$.stock_wb') AS stock_wb \
             FROM a037_wb_product_snapshot d, json_each(d.lines_json) j \
             WHERE json_extract(j.value, '$.nm_id') = ? ORDER BY d.document_date",
        )
        .expect("json_each query must pass SQL guard");
        assert_eq!(info.tables, vec!["a037_wb_product_snapshot".to_string()]);
    }

    #[test]
    fn json_each_argument_subquery_still_enforced_for_sensitive_fields() {
        // A subquery hidden inside json_each(...) must still trip the sensitive-field policy.
        assert!(inspect_read_query(
            "SELECT j.value FROM json_each((SELECT api_key FROM a006_connection_mp)) j"
        )
        .is_err());
    }

    #[test]
    fn blocks_sensitive_fields_and_wildcards_but_allows_count() {
        assert!(inspect_read_query("SELECT api_key FROM a006_connection_mp").is_err());
        assert!(inspect_read_query("SELECT hex(c.api_key) FROM a006_connection_mp c").is_err());
        assert!(inspect_read_query("SELECT * FROM a006_connection_mp").is_err());
        assert!(inspect_read_query("SELECT c.* FROM a006_connection_mp c").is_err());
        assert!(inspect_read_query("SELECT COUNT(*) AS n FROM a006_connection_mp").is_ok());
        assert!(inspect_read_query("SELECT COUNT(c.*) FROM a006_connection_mp c").is_err());
        assert!(inspect_read_query("SELECT SUM(*) FROM a006_connection_mp").is_err());
        assert!(
            inspect_read_query("SELECT id, description, marketplace FROM a006_connection_mp")
                .is_ok()
        );
        assert!(inspect_read_query(
            "WITH c AS (SELECT id, api_key FROM a006_connection_mp) SELECT id FROM c"
        )
        .is_err());
        assert!(inspect_read_query(
            "SELECT id FROM a006_connection_mp WHERE length(api_key_stats) > 0"
        )
        .is_err());
        assert!(
            inspect_read_query("SELECT id FROM a006_connection_mp ORDER BY access_token").is_err()
        );
        assert!(inspect_read_query(
            "SELECT id FROM a006_connection_mp WHERE id IN \
             (SELECT connection_mp_ref FROM x WHERE secret = 'x')"
        )
        .is_err());
        assert!(inspect_read_query(
            "SELECT 'api_key' AS label, COUNT(*) AS n FROM a006_connection_mp"
        )
        .is_ok());
    }
}
