#![allow(deprecated)]

use chrono::{Datelike, Duration, Local, NaiveDate};
use contracts::shared::universal_dashboard::{
    ComparisonOp, ConditionDef, DashboardConfig, DataSourceSchema, DatePreset, FieldDef, FieldType,
    FilterCondition, FilterOperator,
};

/// Result of query building
pub struct QueryResult {
    /// SQL query string
    pub sql: String,
    /// Bound parameters
    pub params: Vec<QueryParam>,
}

/// Query parameter
#[derive(Debug, Clone)]
pub enum QueryParam {
    Text(String),
    Integer(i64),
    Numeric(f64),
}

/// Dynamic SQL query builder
pub struct QueryBuilder<'a> {
    schema: &'a DataSourceSchema,
    config: &'a DashboardConfig,
}

impl<'a> QueryBuilder<'a> {
    /// Create a new query builder
    pub fn new(schema: &'a DataSourceSchema, config: &'a DashboardConfig) -> Self {
        Self { schema, config }
    }

    /// Check if a field is enabled (in enabled_fields list or list is empty = all enabled)
    fn is_field_enabled(&self, field_id: &str) -> bool {
        self.config.enabled_fields.is_empty()
            || self.config.enabled_fields.contains(&field_id.to_string())
    }

    /// Build the SQL query
    pub fn build(&self) -> Result<QueryResult, String> {
        // Build SELECT clause
        let select_clause = self.build_select_clause()?;

        // Build FROM clause
        let from_clause = self.build_from_clause();

        // Build JOIN clause
        let join_clause = self.build_join_clause()?;

        // Build WHERE clause
        let (where_clause, params) = self.build_where_clause()?;

        // Build GROUP BY clause
        let group_by_clause = self.build_group_by_clause()?;

        // Build ORDER BY clause
        let order_by_clause = self.build_order_by_clause()?;

        // Combine all parts
        let mut sql = format!("SELECT {} FROM {}", select_clause, from_clause);

        if !join_clause.is_empty() {
            sql.push_str(&format!(" {}", join_clause));
        }

        if !where_clause.is_empty() {
            sql.push_str(&format!(" WHERE {}", where_clause));
        }

        if !group_by_clause.is_empty() {
            sql.push_str(&format!(" GROUP BY {}", group_by_clause));
        }

        if !order_by_clause.is_empty() {
            sql.push_str(&format!(" ORDER BY {}", order_by_clause));
        }

        Ok(QueryResult { sql, params })
    }

    /// Build SELECT clause with grouping and aggregated columns
    fn build_select_clause(&self) -> Result<String, String> {
        let mut columns = Vec::new();
        let main_table = self.schema.id;

        // Add grouping columns (only if enabled)
        for grouping_field_id in &self.config.groupings {
            if !self.is_field_enabled(grouping_field_id) {
                continue;
            }
            let field = self.find_field(grouping_field_id)?;

            // For ref fields, select both UUID and display name
            if field.ref_table.is_some() {
                columns.push(format!("{}.{}", main_table, field.db_column));
                if let Some(ref_table) = field.ref_table {
                    if let Some(ref_display_col) = field.ref_display_column {
                        columns.push(format!(
                            "{}.{} AS {}_display",
                            ref_table, ref_display_col, field.id
                        ));
                    }
                }
            } else {
                columns.push(format!("{}.{}", main_table, field.db_column));
            }
        }

        // Add display fields (non-aggregated, non-grouping fields, only if enabled)
        for display_field_id in &self.config.display_fields {
            if !self.is_field_enabled(display_field_id) {
                continue;
            }
            if !self.config.groupings.contains(display_field_id) {
                let field = self.find_field(display_field_id)?;

                // For ref fields, select both UUID and display name
                if field.ref_table.is_some() {
                    columns.push(format!("{}.{}", main_table, field.db_column));
                    if let Some(ref_table) = field.ref_table {
                        if let Some(ref_display_col) = field.ref_display_column {
                            columns.push(format!(
                                "{}.{} AS {}_display",
                                ref_table, ref_display_col, field.id
                            ));
                        }
                    }
                } else {
                    columns.push(format!(
                        "{}.{} AS {}",
                        main_table, field.db_column, display_field_id
                    ));
                }
            }
        }

        // Add aggregated columns (only if enabled)
        for selected_field in &self.config.selected_fields {
            if !self.is_field_enabled(&selected_field.field_id) {
                continue;
            }
            let field = self.find_field(&selected_field.field_id)?;

            if let Some(aggregate) = &selected_field.aggregate {
                // Aggregated field
                let agg_expr = format!(
                    "{}({}.{}) AS {}",
                    aggregate.to_sql(),
                    main_table,
                    field.db_column,
                    field.id
                );
                columns.push(agg_expr);
            } else if !self.config.groupings.contains(&selected_field.field_id) {
                // Non-aggregated, non-grouping field - add as-is
                columns.push(format!(
                    "{}.{} AS {}",
                    main_table, field.db_column, field.id
                ));
            }
        }

        if columns.is_empty() {
            return Err("No columns selected".to_string());
        }

        Ok(columns.join(", "))
    }

    /// Build FROM clause
    fn build_from_clause(&self) -> String {
        self.schema.id.to_string()
    }

    /// Build JOIN clause for reference fields
    fn build_join_clause(&self) -> Result<String, String> {
        let mut joins = Vec::new();
        let main_table = self.schema.id;

        // Collect all fields that need JOINs (from groupings, only if enabled)
        for grouping_field_id in &self.config.groupings {
            if !self.is_field_enabled(grouping_field_id) {
                continue;
            }
            let field = self.find_field(grouping_field_id)?;

            if let Some(ref_table) = field.ref_table {
                let join = format!(
                    "LEFT JOIN {} ON {}.{} = {}.id",
                    ref_table, main_table, field.db_column, ref_table
                );
                if !joins.contains(&join) {
                    joins.push(join);
                }
            }
        }

        // Collect all fields that need JOINs (from display_fields, only if enabled)
        for display_field_id in &self.config.display_fields {
            if !self.is_field_enabled(display_field_id) {
                continue;
            }
            let field = self.find_field(display_field_id)?;

            if let Some(ref_table) = field.ref_table {
                let join = format!(
                    "LEFT JOIN {} ON {}.{} = {}.id",
                    ref_table, main_table, field.db_column, ref_table
                );
                if !joins.contains(&join) {
                    joins.push(join);
                }
            }
        }

        Ok(joins.join(" "))
    }

    /// Build WHERE clause with filters
    fn build_where_clause(&self) -> Result<(String, Vec<QueryParam>), String> {
        let mut conditions = Vec::new();
        let mut params = Vec::new();

        let main_table = self.schema.id;

        // Date range filters
        if let Some(date_from) = &self.config.filters.date_from {
            // Find date field in schema
            let date_field = self
                .schema
                .fields
                .iter()
                .find(|f| f.field_type == FieldType::Date)
                .ok_or("No date field found in schema")?;

            conditions.push(format!("{}.{} >= ?", main_table, date_field.db_column));
            params.push(QueryParam::Text(date_from.clone()));
        }

        if let Some(date_to) = &self.config.filters.date_to {
            let date_field = self
                .schema
                .fields
                .iter()
                .find(|f| f.field_type == FieldType::Date)
                .ok_or("No date field found in schema")?;

            conditions.push(format!("{}.{} <= ?", main_table, date_field.db_column));
            params.push(QueryParam::Text(date_to.clone()));
        }

        // Dimension filters (legacy)
        for (field_id, values) in &self.config.filters.dimensions {
            if values.is_empty() {
                continue;
            }

            let field = self.find_field(field_id)?;
            let placeholders: Vec<_> = (0..values.len()).map(|_| "?").collect();
            conditions.push(format!(
                "{}.{} IN ({})",
                main_table,
                field.db_column,
                placeholders.join(", ")
            ));

            for value in values {
                match field.field_type {
                    FieldType::Integer => {
                        let int_val = value
                            .parse::<i64>()
                            .map_err(|_| format!("Invalid integer value: {}", value))?;
                        params.push(QueryParam::Integer(int_val));
                    }
                    FieldType::Numeric => {
                        let num_val = value
                            .parse::<f64>()
                            .map_err(|_| format!("Invalid numeric value: {}", value))?;
                        params.push(QueryParam::Numeric(num_val));
                    }
                    _ => {
                        params.push(QueryParam::Text(value.clone()));
                    }
                }
            }
        }

        // Field-specific filters with operators
        for filter in &self.config.filters.field_filters {
            let field = self.find_field(&filter.field_id)?;
            let column_ref = format!("{}.{}", main_table, field.db_column);

            match filter.operator {
                FilterOperator::IsNull => {
                    conditions.push(format!("{} IS NULL", column_ref));
                }
                FilterOperator::Between => {
                    if let Some(value2) = &filter.value2 {
                        conditions.push(format!("{} BETWEEN ? AND ?", column_ref));
                        self.push_typed_param(&mut params, &filter.value, &field.field_type)?;
                        self.push_typed_param(&mut params, value2, &field.field_type)?;
                    } else {
                        return Err("BETWEEN operator requires two values".to_string());
                    }
                }
                FilterOperator::In => {
                    // Parse comma-separated values
                    let values: Vec<&str> = filter.value.split(',').map(|s| s.trim()).collect();
                    if values.is_empty() {
                        continue;
                    }
                    let placeholders: Vec<_> = (0..values.len()).map(|_| "?").collect();
                    conditions.push(format!("{} IN ({})", column_ref, placeholders.join(", ")));
                    for value in values {
                        self.push_typed_param(&mut params, value, &field.field_type)?;
                    }
                }
                FilterOperator::Like => {
                    conditions.push(format!("{} LIKE ?", column_ref));
                    params.push(QueryParam::Text(format!("%{}%", filter.value)));
                }
                _ => {
                    // Standard comparison operators: =, <>, <, >, <=, >=
                    conditions.push(format!("{} {} ?", column_ref, filter.operator.to_sql()));
                    self.push_typed_param(&mut params, &filter.value, &field.field_type)?;
                }
            }
        }

        // New filter conditions (FilterCondition format)
        for condition in &self.config.filters.conditions {
            // Skip inactive conditions
            if !condition.active {
                continue;
            }

            let (sql, mut cond_params) = self.condition_to_sql(condition, main_table)?;
            conditions.push(sql);
            params.append(&mut cond_params);
        }

        Ok((conditions.join(" AND "), params))
    }

    /// Build GROUP BY clause
    fn build_group_by_clause(&self) -> Result<String, String> {
        if self.config.groupings.is_empty() && self.config.display_fields.is_empty() {
            return Ok(String::new());
        }

        let mut columns = Vec::new();
        let main_table = self.schema.id;

        // Add grouping columns (only if enabled)
        for grouping_field_id in &self.config.groupings {
            if !self.is_field_enabled(grouping_field_id) {
                continue;
            }
            let field = self.find_field(grouping_field_id)?;
            columns.push(format!("{}.{}", main_table, field.db_column));

            // Also group by display column for ref fields
            if field.ref_table.is_some() {
                if let Some(ref_table) = field.ref_table {
                    if let Some(ref_display_col) = field.ref_display_column {
                        columns.push(format!("{}.{}", ref_table, ref_display_col));
                    }
                }
            }
        }

        // Add display fields (they are not aggregated, so must be in GROUP BY, only if enabled)
        for display_field_id in &self.config.display_fields {
            if !self.is_field_enabled(display_field_id) {
                continue;
            }
            if !self.config.groupings.contains(display_field_id) {
                let field = self.find_field(display_field_id)?;
                columns.push(format!("{}.{}", main_table, field.db_column));

                // Also group by display column for ref fields
                if field.ref_table.is_some() {
                    if let Some(ref_table) = field.ref_table {
                        if let Some(ref_display_col) = field.ref_display_column {
                            columns.push(format!("{}.{}", ref_table, ref_display_col));
                        }
                    }
                }
            }
        }

        Ok(columns.join(", "))
    }

    /// Build ORDER BY clause
    fn build_order_by_clause(&self) -> Result<String, String> {
        if self.config.groupings.is_empty() {
            return Ok(String::new());
        }

        let mut columns = Vec::new();
        let main_table = self.schema.id;

        for grouping_field_id in &self.config.groupings {
            if !self.is_field_enabled(grouping_field_id) {
                continue;
            }
            let field = self.find_field(grouping_field_id)?;

            // Order by display column for ref fields, otherwise by the field itself
            if let Some(ref_table) = field.ref_table {
                if let Some(ref_display_col) = field.ref_display_column {
                    columns.push(format!("{}.{}", ref_table, ref_display_col));
                } else {
                    columns.push(format!("{}.{}", main_table, field.db_column));
                }
            } else {
                columns.push(format!("{}.{}", main_table, field.db_column));
            }
        }

        Ok(columns.join(", "))
    }

    /// Find a field definition by ID
    fn find_field(&self, field_id: &str) -> Result<&FieldDef, String> {
        self.schema
            .fields
            .iter()
            .find(|f| f.id == field_id)
            .ok_or_else(|| format!("Field not found: {}", field_id))
    }

    /// Push a typed parameter based on field type
    fn push_typed_param(
        &self,
        params: &mut Vec<QueryParam>,
        value: &str,
        field_type: &FieldType,
    ) -> Result<(), String> {
        match field_type {
            FieldType::Integer => {
                let int_val = value
                    .parse::<i64>()
                    .map_err(|_| format!("Invalid integer value: {}", value))?;
                params.push(QueryParam::Integer(int_val));
            }
            FieldType::Numeric => {
                let num_val = value
                    .parse::<f64>()
                    .map_err(|_| format!("Invalid numeric value: {}", value))?;
                params.push(QueryParam::Numeric(num_val));
            }
            _ => {
                params.push(QueryParam::Text(value.to_string()));
            }
        }
        Ok(())
    }

    /// Build SQL fragment from FilterCondition
    pub fn condition_to_sql(
        &self,
        condition: &FilterCondition,
        table_alias: &str,
    ) -> Result<(String, Vec<QueryParam>), String> {
        let field = self.find_field(&condition.field_id)?;
        let column_ref = format!("{}.{}", table_alias, field.db_column);

        let mut params = Vec::new();

        let sql = match &condition.definition {
            ConditionDef::Comparison { operator, value } => {
                self.push_typed_param(&mut params, value, &field.field_type)?;
                format!("{} {} ?", column_ref, comparison_op_to_sql(*operator))
            }
            ConditionDef::Range { from, to } => match (from, to) {
                (Some(f), Some(t)) => {
                    self.push_typed_param(&mut params, f, &field.field_type)?;
                    self.push_typed_param(&mut params, t, &field.field_type)?;
                    format!("{} BETWEEN ? AND ?", column_ref)
                }
                (Some(f), None) => {
                    self.push_typed_param(&mut params, f, &field.field_type)?;
                    format!("{} >= ?", column_ref)
                }
                (None, Some(t)) => {
                    self.push_typed_param(&mut params, t, &field.field_type)?;
                    format!("{} <= ?", column_ref)
                }
                (None, None) => {
                    return Err("Range condition requires at least one bound".to_string())
                }
            },
            ConditionDef::DatePeriod { preset, from, to } => {
                // Resolve preset to absolute dates if present
                let (resolved_from, resolved_to) = if let Some(p) = preset {
                    resolve_date_preset(*p)
                } else {
                    (from.clone(), to.clone())
                };

                match (resolved_from, resolved_to) {
                    (Some(f), Some(t)) => {
                        params.push(QueryParam::Text(f));
                        params.push(QueryParam::Text(t));
                        format!("{} BETWEEN ? AND ?", column_ref)
                    }
                    (Some(f), None) => {
                        params.push(QueryParam::Text(f));
                        format!("{} >= ?", column_ref)
                    }
                    (None, Some(t)) => {
                        params.push(QueryParam::Text(t));
                        format!("{} <= ?", column_ref)
                    }
                    (None, None) => {
                        return Err("Date period condition requires at least one date".to_string())
                    }
                }
            }
            ConditionDef::Nullability { is_null } => {
                if *is_null {
                    format!("{} IS NULL", column_ref)
                } else {
                    format!("{} IS NOT NULL", column_ref)
                }
            }
            ConditionDef::Contains { pattern } => {
                params.push(QueryParam::Text(format!("%{}%", pattern)));
                format!("{} LIKE ?", column_ref)
            }
            ConditionDef::InList { values, negated } => {
                if values.is_empty() {
                    return Err("InList condition requires at least one value".to_string());
                }
                let placeholders = vec!["?"; values.len()].join(", ");
                for value in values {
                    self.push_typed_param(&mut params, value, &field.field_type)?;
                }
                if *negated {
                    format!("{} NOT IN ({})", column_ref, placeholders)
                } else {
                    format!("{} IN ({})", column_ref, placeholders)
                }
            }
        };

        Ok((sql, params))
    }
}

/// Convert ComparisonOp to SQL operator
fn comparison_op_to_sql(op: ComparisonOp) -> &'static str {
    match op {
        ComparisonOp::Eq => "=",
        ComparisonOp::NotEq => "<>",
        ComparisonOp::Lt => "<",
        ComparisonOp::Gt => ">",
        ComparisonOp::LtEq => "<=",
        ComparisonOp::GtEq => ">=",
    }
}

/// Resolve date preset to absolute dates (YYYY-MM-DD)
fn resolve_date_preset(preset: DatePreset) -> (Option<String>, Option<String>) {
    let now = Local::now().date_naive();

    match preset {
        DatePreset::Today => {
            let date_str = now.format("%Y-%m-%d").to_string();
            (Some(date_str.clone()), Some(date_str))
        }
        DatePreset::Yesterday => {
            let date = now - Duration::days(1);
            let date_str = date.format("%Y-%m-%d").to_string();
            (Some(date_str.clone()), Some(date_str))
        }
        DatePreset::ThisWeek => {
            let start = start_of_week(now);
            let end = start + Duration::days(6);
            (
                Some(start.format("%Y-%m-%d").to_string()),
                Some(end.format("%Y-%m-%d").to_string()),
            )
        }
        DatePreset::LastWeek => {
            let start = start_of_week(now) - Duration::days(7);
            let end = start + Duration::days(6);
            (
                Some(start.format("%Y-%m-%d").to_string()),
                Some(end.format("%Y-%m-%d").to_string()),
            )
        }
        DatePreset::ThisMonth => {
            let start = NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();
            let end = if now.month() == 12 {
                NaiveDate::from_ymd_opt(now.year() + 1, 1, 1).unwrap() - Duration::days(1)
            } else {
                NaiveDate::from_ymd_opt(now.year(), now.month() + 1, 1).unwrap() - Duration::days(1)
            };
            (
                Some(start.format("%Y-%m-%d").to_string()),
                Some(end.format("%Y-%m-%d").to_string()),
            )
        }
        DatePreset::LastMonth => {
            let (year, month) = if now.month() == 1 {
                (now.year() - 1, 12)
            } else {
                (now.year(), now.month() - 1)
            };
            let start = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
            let end =
                NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap() - Duration::days(1);
            (
                Some(start.format("%Y-%m-%d").to_string()),
                Some(end.format("%Y-%m-%d").to_string()),
            )
        }
        DatePreset::ThisQuarter => {
            let quarter_start_month = ((now.month() - 1) / 3) * 3 + 1;
            let start = NaiveDate::from_ymd_opt(now.year(), quarter_start_month, 1).unwrap();
            let end = if quarter_start_month + 3 > 12 {
                NaiveDate::from_ymd_opt(now.year() + 1, 1, 1).unwrap() - Duration::days(1)
            } else {
                NaiveDate::from_ymd_opt(now.year(), quarter_start_month + 3, 1).unwrap()
                    - Duration::days(1)
            };
            (
                Some(start.format("%Y-%m-%d").to_string()),
                Some(end.format("%Y-%m-%d").to_string()),
            )
        }
        DatePreset::LastQuarter => {
            let current_quarter_start = ((now.month() - 1) / 3) * 3 + 1;
            let (start_year, start_month) = if current_quarter_start <= 3 {
                (now.year() - 1, current_quarter_start + 9)
            } else {
                (now.year(), current_quarter_start - 3)
            };
            let start = NaiveDate::from_ymd_opt(start_year, start_month, 1).unwrap();
            let end = NaiveDate::from_ymd_opt(now.year(), current_quarter_start, 1).unwrap()
                - Duration::days(1);
            (
                Some(start.format("%Y-%m-%d").to_string()),
                Some(end.format("%Y-%m-%d").to_string()),
            )
        }
        DatePreset::ThisYear => {
            let start = NaiveDate::from_ymd_opt(now.year(), 1, 1).unwrap();
            let end = NaiveDate::from_ymd_opt(now.year(), 12, 31).unwrap();
            (
                Some(start.format("%Y-%m-%d").to_string()),
                Some(end.format("%Y-%m-%d").to_string()),
            )
        }
        DatePreset::LastYear => {
            let year = now.year() - 1;
            let start = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
            let end = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();
            (
                Some(start.format("%Y-%m-%d").to_string()),
                Some(end.format("%Y-%m-%d").to_string()),
            )
        }
        DatePreset::Last7Days => {
            let start = now - Duration::days(6);
            (
                Some(start.format("%Y-%m-%d").to_string()),
                Some(now.format("%Y-%m-%d").to_string()),
            )
        }
        DatePreset::Last30Days => {
            let start = now - Duration::days(29);
            (
                Some(start.format("%Y-%m-%d").to_string()),
                Some(now.format("%Y-%m-%d").to_string()),
            )
        }
        DatePreset::Last90Days => {
            let start = now - Duration::days(89);
            (
                Some(start.format("%Y-%m-%d").to_string()),
                Some(now.format("%Y-%m-%d").to_string()),
            )
        }
    }
}

/// Get start of week (Monday) for a given date
fn start_of_week(date: NaiveDate) -> NaiveDate {
    let weekday = date.weekday();
    let days_from_monday = weekday.num_days_from_monday();
    date - Duration::days(days_from_monday as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::shared::universal_dashboard::*;

    #[test]
    fn test_simple_query() {
        let schema = DataSourceSchema {
            id: "test_table",
            name: "Test Table",
            fields: &[
                FieldDef {
                    id: "date",
                    name: "Date",
                    field_type: FieldType::Date,
                    can_group: true,
                    can_aggregate: false,
                    db_column: "date",
                    ref_table: None,
                    ref_display_column: None,
                },
                FieldDef {
                    id: "amount",
                    name: "Amount",
                    field_type: FieldType::Numeric,
                    can_group: false,
                    can_aggregate: true,
                    db_column: "amount",
                    ref_table: None,
                    ref_display_column: None,
                },
            ],
        };

        let config = DashboardConfig {
            data_source: "test_table".to_string(),
            selected_fields: vec![SelectedField {
                field_id: "amount".to_string(),
                aggregate: Some(AggregateFunction::Sum),
            }],
            groupings: vec!["date".to_string()],
            display_fields: vec![],
            enabled_fields: vec![],
            sort: DashboardSort::default(),
            filters: DashboardFilters::default(),
        };

        let builder = QueryBuilder::new(&schema, &config);
        let result = builder.build().unwrap();

        assert!(result.sql.contains("SELECT test_table.date"));
        assert!(result.sql.contains("SUM(test_table.amount)"));
        assert!(result.sql.contains("FROM test_table"));
        assert!(result.sql.contains("GROUP BY test_table.date"));
    }
}
