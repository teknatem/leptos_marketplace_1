use sqlparser::ast::{ObjectName, Query, Statement, TableFactor, Visit, Visitor};
use sqlparser::dialect::SQLiteDialect;
use sqlparser::parser::Parser;
use std::collections::HashSet;
use std::ops::ControlFlow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadQueryInfo {
    pub tables: Vec<String>,
}

#[derive(Default)]
struct RelationVisitor {
    relations: HashSet<String>,
    cte_names: HashSet<String>,
    unsupported_table_factor: Option<String>,
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
    if let Some(factor) = visitor.unsupported_table_factor {
        return Err(format!("Unsupported table expression: {factor}"));
    }

    let mut tables: Vec<String> = visitor
        .relations
        .difference(&visitor.cte_names)
        .cloned()
        .collect();
    tables.sort();
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
}
