#![allow(deprecated)]

use chrono::{Datelike, Duration, Local, NaiveDate};
use contracts::shared::universal_dashboard::{
    AggregateFunction, ComparisonOp, ConditionDef, DashboardConfig, DataSourceSchemaOwned,
    DatePreset, FieldDefOwned, FilterCondition, FilterOperator, SortDirection, ValueType,
};

/// Result of query building
#[derive(Debug)]
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
    schema: &'a DataSourceSchemaOwned,
    config: &'a DashboardConfig,
    table_name: String,
}

impl<'a> QueryBuilder<'a> {
    fn filter_expr_for_field(column_ref: &str, value_type: &ValueType) -> String {
        match value_type {
            // Date fields are often stored as RFC3339 text in SQLite.
            // Normalize to YYYY-MM-DD so the period end includes the whole last day.
            ValueType::Date | ValueType::DateTime => format!("substr({}, 1, 10)", column_ref),
            _ => column_ref.to_string(),
        }
    }

    /// Create a new query builder
    pub fn new(
        schema: &'a DataSourceSchemaOwned,
        config: &'a DashboardConfig,
        table_name: String,
    ) -> Self {
        Self {
            schema,
            config,
            table_name,
        }
    }

    /// Check if a field is enabled (in enabled_fields list or list is empty = all enabled)
    fn is_field_enabled(&self, field_id: &str) -> bool {
        self.config.enabled_fields.is_empty()
            || self.config.enabled_fields.contains(&field_id.to_string())
    }

    /// Build the SQL query
    pub fn build(&self) -> Result<QueryResult, String> {
        self.validate_config()?;
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

    fn validate_config(&self) -> Result<(), String> {
        for field_id in &self.config.groupings {
            let field = self.find_field(field_id)?;
            if !field.can_group {
                return Err(format!("Field '{}' cannot be used for grouping", field_id));
            }
        }

        for selected in &self.config.selected_fields {
            let field = self.find_field(&selected.field_id)?;
            if let Some(aggregate) = selected.aggregate {
                if !field.can_aggregate {
                    return Err(format!(
                        "Field '{}' cannot be aggregated",
                        selected.field_id
                    ));
                }
                if matches!(aggregate, AggregateFunction::Sum | AggregateFunction::Avg)
                    && !matches!(&field.value_type, ValueType::Integer | ValueType::Numeric)
                {
                    return Err(format!(
                        "Aggregate {:?} requires a numeric field, got '{}'",
                        aggregate, selected.field_id
                    ));
                }
            }
        }

        for field_id in &self.config.display_fields {
            self.find_field(field_id)?;
        }

        for rule in &self.config.sort.rules {
            // Метрику можно сортировать по её полю ("customer_in") ИЛИ по алиасу агрегата
            // ("customer_in_sum"). Алиас агрегата — не поле схемы, но он валиден (метрика
            // присутствует в выборке), поэтому разрешаем его явно.
            if self.resolve_metric_sort(&rule.field_id).is_some() {
                continue;
            }
            self.find_field(&rule.field_id)?;
            let is_output = self.config.groupings.contains(&rule.field_id)
                || self.config.display_fields.contains(&rule.field_id)
                || self
                    .config
                    .selected_fields
                    .iter()
                    .any(|field| field.field_id == rule.field_id);
            if !is_output {
                return Err(format!(
                    "Sort field '{}' must be present in query output",
                    rule.field_id
                ));
            }
        }

        Ok(())
    }

    /// Build SELECT clause with grouping and aggregated columns
    fn build_select_clause(&self) -> Result<String, String> {
        let mut columns = Vec::new();
        let main_table = self.table_name.as_str();

        // Add grouping columns (only if enabled)
        for grouping_field_id in &self.config.groupings {
            if !self.is_field_enabled(grouping_field_id) {
                continue;
            }
            let field = self.find_field(grouping_field_id)?;

            // Determine source table (use source_table if specified, otherwise main_table)
            let source_table = field.source_table.as_deref().unwrap_or(main_table);

            // For ref fields, select both UUID and display name
            if field.ref_table.is_some() {
                columns.push(format!(
                    "{}.{} AS {}",
                    source_table, field.db_column, field.id
                ));
                if let Some(ref_table) = field.ref_table.as_deref() {
                    if let Some(ref_display_col) = field.ref_display_column.as_deref() {
                        columns.push(format!(
                            "{}.{} AS {}_display",
                            ref_table, ref_display_col, field.id
                        ));
                    }
                }
            } else {
                columns.push(format!(
                    "{}.{} AS {}",
                    source_table, field.db_column, field.id
                ));
            }
        }

        // Add display fields (non-aggregated, non-grouping fields, only if enabled)
        for display_field_id in &self.config.display_fields {
            if !self.is_field_enabled(display_field_id) {
                continue;
            }
            if !self.config.groupings.contains(display_field_id) {
                let field = self.find_field(display_field_id)?;

                // Determine source table (use source_table if specified, otherwise main_table)
                let source_table = field.source_table.as_deref().unwrap_or(main_table);

                // For ref fields, select both UUID and display name
                if field.ref_table.is_some() {
                    columns.push(format!(
                        "{}.{} AS {}",
                        source_table, field.db_column, field.id
                    ));
                    if let Some(ref_table) = field.ref_table.as_deref() {
                        if let Some(ref_display_col) = field.ref_display_column.as_deref() {
                            columns.push(format!(
                                "{}.{} AS {}_display",
                                ref_table, ref_display_col, field.id
                            ));
                        }
                    }
                } else {
                    columns.push(format!(
                        "{}.{} AS {}",
                        source_table, field.db_column, display_field_id
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

            // Determine source table (use source_table if specified, otherwise main_table)
            let source_table = field.source_table.as_deref().unwrap_or(main_table);

            if let Some(aggregate) = &selected_field.aggregate {
                // Aggregated field
                let agg_expr = format!(
                    "{}({}.{}) AS {}",
                    aggregate.to_sql(),
                    source_table,
                    field.db_column,
                    field.id
                );
                columns.push(agg_expr);
            } else if !self.config.groupings.contains(&selected_field.field_id) {
                // Non-aggregated, non-grouping field - add as-is
                columns.push(format!(
                    "{}.{} AS {}",
                    source_table, field.db_column, field.id
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
        self.table_name.clone()
    }

    /// Append the LEFT JOIN(s) a single field needs (its `source_table` and/or
    /// `ref_table`), de-duplicating against joins already collected.
    fn push_field_joins(&self, field: &FieldDefOwned, joins: &mut Vec<String>) {
        let main_table = self.table_name.as_str();

        // source_table JOIN: field physically lives in another table reached via join_on_column.
        if let (Some(source_table), Some(join_on_column)) = (
            field.source_table.as_deref(),
            field.join_on_column.as_deref(),
        ) {
            let join = format!(
                "LEFT JOIN {} ON {}.{} = {}.id",
                source_table, main_table, join_on_column, source_table
            );
            if !joins.contains(&join) {
                joins.push(join);
            }
        }

        // ref_table JOIN: reference field that resolves to a display column.
        if let Some(ref_table) = field.ref_table.as_deref() {
            let source_table_name = field.source_table.as_deref().unwrap_or(main_table);
            let join = format!(
                "LEFT JOIN {} ON {}.{} = {}.id",
                ref_table, source_table_name, field.db_column, ref_table
            );
            if !joins.contains(&join) {
                joins.push(join);
            }
        }
    }

    /// Field ids referenced by the active filters (dimensions / field_filters / conditions).
    fn filter_field_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        for (field_id, values) in &self.config.filters.dimensions {
            if !values.is_empty() {
                ids.push(field_id.clone());
            }
        }
        for filter in &self.config.filters.field_filters {
            ids.push(filter.field_id.clone());
        }
        for condition in &self.config.filters.conditions {
            if condition.active {
                ids.push(condition.field_id.clone());
            }
        }
        ids
    }

    /// Build JOIN clause for reference fields
    fn build_join_clause(&self) -> Result<String, String> {
        let mut joins = Vec::new();

        // Groupings and display fields (only when enabled).
        for field_id in self
            .config
            .groupings
            .iter()
            .chain(self.config.display_fields.iter())
        {
            if !self.is_field_enabled(field_id) {
                continue;
            }
            let field = self.find_field(field_id)?;
            self.push_field_joins(field, &mut joins);
        }

        // Fields referenced only by a filter still need their JOIN, otherwise the WHERE
        // clause references a table that was never added to FROM (e.g. a `marketplace`
        // filter on ds03 produced "no such column: a006_connection_mp.marketplace").
        // Unknown ids are surfaced later by build_where_clause; ignore them here.
        for field_id in self.filter_field_ids() {
            if let Ok(field) = self.find_field(&field_id) {
                self.push_field_joins(field, &mut joins);
            }
        }

        // Sort rules may also order by a joined column.
        for rule in &self.config.sort.rules {
            if let Ok(field) = self.find_field(&rule.field_id) {
                self.push_field_joins(field, &mut joins);
            }
        }

        // Date range filters resolve to the schema's date field, which may itself be joined.
        if self.config.filters.date_from.is_some() || self.config.filters.date_to.is_some() {
            if let Some(date_field) = self
                .schema
                .fields
                .iter()
                .find(|f| matches!(&f.value_type, ValueType::Date | ValueType::DateTime))
            {
                self.push_field_joins(date_field, &mut joins);
            }
        }

        Ok(joins.join(" "))
    }

    /// Build WHERE clause with filters
    fn build_where_clause(&self) -> Result<(String, Vec<QueryParam>), String> {
        let mut conditions = Vec::new();
        let mut params = Vec::new();

        let main_table = self.table_name.as_str();

        // Date range filters
        if let Some(date_from) = &self.config.filters.date_from {
            // Find date field in schema
            let date_field = self
                .schema
                .fields
                .iter()
                .find(|f| matches!(&f.value_type, ValueType::Date | ValueType::DateTime))
                .ok_or("No date field found in schema")?;

            let source_table = date_field.source_table.as_deref().unwrap_or(main_table);
            let column_ref = format!("{}.{}", source_table, date_field.db_column);
            conditions.push(format!(
                "{} >= ?",
                Self::filter_expr_for_field(&column_ref, &date_field.value_type)
            ));
            params.push(QueryParam::Text(date_from.clone()));
        }

        if let Some(date_to) = &self.config.filters.date_to {
            let date_field = self
                .schema
                .fields
                .iter()
                .find(|f| matches!(&f.value_type, ValueType::Date | ValueType::DateTime))
                .ok_or("No date field found in schema")?;

            let source_table = date_field.source_table.as_deref().unwrap_or(main_table);
            let column_ref = format!("{}.{}", source_table, date_field.db_column);
            conditions.push(format!(
                "{} <= ?",
                Self::filter_expr_for_field(&column_ref, &date_field.value_type)
            ));
            params.push(QueryParam::Text(date_to.clone()));
        }

        // Dimension filters (legacy)
        for (field_id, values) in &self.config.filters.dimensions {
            if values.is_empty() {
                continue;
            }

            let field = self.find_field(field_id)?;
            if !field.can_filter {
                return Err(format!("Field '{}' is not filterable", field_id));
            }
            let source_table = field.source_table.as_deref().unwrap_or(main_table);
            let placeholders: Vec<_> = (0..values.len()).map(|_| "?").collect();
            conditions.push(format!(
                "{}.{} IN ({})",
                source_table,
                field.db_column,
                placeholders.join(", ")
            ));

            for value in values {
                match &field.value_type {
                    ValueType::Integer => {
                        let int_val = value
                            .parse::<i64>()
                            .map_err(|_| format!("Invalid integer value: {}", value))?;
                        params.push(QueryParam::Integer(int_val));
                    }
                    ValueType::Numeric => {
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
            if !field.can_filter {
                return Err(format!("Field '{}' is not filterable", filter.field_id));
            }
            let source_table = field.source_table.as_deref().unwrap_or(main_table);
            let column_ref = format!("{}.{}", source_table, field.db_column);
            let filter_expr = Self::filter_expr_for_field(&column_ref, &field.value_type);

            match filter.operator {
                FilterOperator::IsNull => {
                    conditions.push(format!("{} IS NULL", column_ref));
                }
                FilterOperator::Between => {
                    if let Some(value2) = &filter.value2 {
                        conditions.push(format!("{} BETWEEN ? AND ?", filter_expr));
                        self.push_typed_param(&mut params, &filter.value, &field.value_type)?;
                        self.push_typed_param(&mut params, value2, &field.value_type)?;
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
                    conditions.push(format!("{} IN ({})", filter_expr, placeholders.join(", ")));
                    for value in values {
                        self.push_typed_param(&mut params, value, &field.value_type)?;
                    }
                }
                FilterOperator::Like => {
                    conditions.push(format!("{} LIKE ?", filter_expr));
                    params.push(QueryParam::Text(format!("%{}%", filter.value)));
                }
                _ => {
                    // Standard comparison operators: =, <>, <, >, <=, >=
                    conditions.push(format!("{} {} ?", filter_expr, filter.operator.to_sql()));
                    self.push_typed_param(&mut params, &filter.value, &field.value_type)?;
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
        let main_table = self.table_name.as_str();

        // Add grouping columns (only if enabled)
        for grouping_field_id in &self.config.groupings {
            if !self.is_field_enabled(grouping_field_id) {
                continue;
            }
            let field = self.find_field(grouping_field_id)?;
            let source_table = field.source_table.as_deref().unwrap_or(main_table);
            columns.push(format!("{}.{}", source_table, field.db_column));

            // Also group by display column for ref fields
            if field.ref_table.is_some() {
                if let Some(ref_table) = field.ref_table.as_deref() {
                    if let Some(ref_display_col) = field.ref_display_column.as_deref() {
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
                let source_table = field.source_table.as_deref().unwrap_or(main_table);
                columns.push(format!("{}.{}", source_table, field.db_column));

                // Also group by display column for ref fields
                if field.ref_table.is_some() {
                    if let Some(ref_table) = field.ref_table.as_deref() {
                        if let Some(ref_display_col) = field.ref_display_column.as_deref() {
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
        if self.config.sort.rules.is_empty() && self.config.groupings.is_empty() {
            return Ok(String::new());
        }

        let mut columns = Vec::new();
        let main_table = self.table_name.as_str();

        if !self.config.sort.rules.is_empty() {
            for rule in &self.config.sort.rules {
                let expression = match self.find_field(&rule.field_id) {
                    Ok(field) => {
                        if self.config.selected_fields.iter().any(|selected| {
                            selected.field_id == rule.field_id && selected.aggregate.is_some()
                        }) {
                            // Сортировка по полю-метрике ("customer_in") — по алиасу агрегата.
                            field.id.clone()
                        } else if let (Some(ref_table), Some(display_column)) = (
                            field.ref_table.as_deref(),
                            field.ref_display_column.as_deref(),
                        ) {
                            format!("{}.{}", ref_table, display_column)
                        } else {
                            let source_table = field.source_table.as_deref().unwrap_or(main_table);
                            format!("{}.{}", source_table, field.db_column)
                        }
                    }
                    // Поля нет — это может быть алиас агрегата метрики ("customer_in_sum").
                    Err(e) => self.resolve_metric_sort(&rule.field_id).ok_or(e)?,
                };
                let direction = match rule.direction {
                    SortDirection::Asc => "ASC",
                    SortDirection::Desc => "DESC",
                };
                columns.push(format!("{} {}", expression, direction));
            }
            return Ok(columns.join(", "));
        }

        for grouping_field_id in &self.config.groupings {
            if !self.is_field_enabled(grouping_field_id) {
                continue;
            }
            let field = self.find_field(grouping_field_id)?;

            // Determine the correct table prefix for ORDER BY
            let table_prefix = if let Some(ref_table) = field.ref_table.as_deref() {
                // For reference fields, order by display column
                if let Some(ref_display_col) = field.ref_display_column.as_deref() {
                    columns.push(format!("{}.{}", ref_table, ref_display_col));
                    continue;
                } else {
                    main_table
                }
            } else if let Some(source_table) = field.source_table.as_deref() {
                // For fields from joined tables (e.g., dim1_category from a004_nomenclature)
                source_table
            } else {
                // For fields from main table
                main_table
            };

            columns.push(format!("{}.{}", table_prefix, field.db_column));
        }

        Ok(columns.join(", "))
    }

    /// Find a field definition by ID
    fn find_field(&self, field_id: &str) -> Result<&FieldDefOwned, String> {
        self.schema
            .fields
            .iter()
            .find(|f| f.id == field_id)
            .ok_or_else(|| format!("Field not found: {}", field_id))
    }

    /// Разрешить сортировку по АГРЕГАТУ метрики, заданному алиасом вида `<field>_<agg>`
    /// (напр. `customer_in_sum`). Если в SELECT есть метрика `<field>` с агрегатом `<agg>`,
    /// вернуть выражение ORDER BY = алиас агрегата в SELECT (= `field_id` метрики).
    /// Так модель может писать естественное `sort: customer_in_sum`, не зная точного алиаса.
    fn resolve_metric_sort(&self, sort_field_id: &str) -> Option<String> {
        self.config.selected_fields.iter().find_map(|selected| {
            let aggregate = selected.aggregate?;
            let expected = format!(
                "{}_{}",
                selected.field_id,
                aggregate.to_sql().to_lowercase()
            );
            if sort_field_id.eq_ignore_ascii_case(&expected) {
                // Алиас агрегата в SELECT — это field.id метрики (см. build_select_clause).
                Some(selected.field_id.clone())
            } else {
                None
            }
        })
    }

    /// Push a typed parameter based on field type
    fn push_typed_param(
        &self,
        params: &mut Vec<QueryParam>,
        value: &str,
        value_type: &ValueType,
    ) -> Result<(), String> {
        match value_type {
            ValueType::Integer => {
                let int_val = value
                    .parse::<i64>()
                    .map_err(|_| format!("Invalid integer value: {}", value))?;
                params.push(QueryParam::Integer(int_val));
            }
            ValueType::Numeric => {
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
        if !field.can_filter {
            return Err(format!(
                "Field '{}' is not filterable (can_filter = false)",
                condition.field_id
            ));
        }
        let source_table = field.source_table.as_deref().unwrap_or(table_alias);
        let column_ref = format!("{}.{}", source_table, field.db_column);
        let filter_expr = Self::filter_expr_for_field(&column_ref, &field.value_type);

        let mut params = Vec::new();

        let sql = match &condition.definition {
            ConditionDef::Comparison { operator, value } => {
                self.push_typed_param(&mut params, value, &field.value_type)?;
                format!("{} {} ?", filter_expr, comparison_op_to_sql(*operator))
            }
            ConditionDef::Range { from, to } => match (from, to) {
                (Some(f), Some(t)) => {
                    self.push_typed_param(&mut params, f, &field.value_type)?;
                    self.push_typed_param(&mut params, t, &field.value_type)?;
                    format!("{} BETWEEN ? AND ?", filter_expr)
                }
                (Some(f), None) => {
                    self.push_typed_param(&mut params, f, &field.value_type)?;
                    format!("{} >= ?", filter_expr)
                }
                (None, Some(t)) => {
                    self.push_typed_param(&mut params, t, &field.value_type)?;
                    format!("{} <= ?", filter_expr)
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
                        format!("{} BETWEEN ? AND ?", filter_expr)
                    }
                    (Some(f), None) => {
                        params.push(QueryParam::Text(f));
                        format!("{} >= ?", filter_expr)
                    }
                    (None, Some(t)) => {
                        params.push(QueryParam::Text(t));
                        format!("{} <= ?", filter_expr)
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
                    self.push_typed_param(&mut params, value, &field.value_type)?;
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
                    can_filter: true,
                    db_column: "date",
                    ref_table: None,
                    ref_display_column: None,
                    source_table: None,
                    join_on_column: None,
                },
                FieldDef {
                    id: "amount",
                    name: "Amount",
                    field_type: FieldType::Numeric,
                    can_group: false,
                    can_aggregate: true,
                    can_filter: false,
                    db_column: "amount",
                    ref_table: None,
                    ref_display_column: None,
                    source_table: None,
                    join_on_column: None,
                },
            ],
            schema_filters: &[],
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

        let schema: DataSourceSchemaOwned = (&schema).into();
        let builder = QueryBuilder::new(&schema, &config, "test_table".to_string());
        let result = builder.build().unwrap();

        assert!(result.sql.contains("SELECT test_table.date"));
        assert!(result.sql.contains("SUM(test_table.amount)"));
        assert!(result.sql.contains("FROM test_table"));
        assert!(result.sql.contains("GROUP BY test_table.date"));
    }

    /// Сортировка по агрегату метрики через алиас `<field>_<agg>` (напр. `amount_sum`):
    /// ORDER BY должен ссылаться на алиас агрегата (`amount`), а не падать «Field not found».
    #[test]
    fn sorts_by_metric_aggregate_alias() {
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
                    can_filter: true,
                    db_column: "date",
                    ref_table: None,
                    ref_display_column: None,
                    source_table: None,
                    join_on_column: None,
                },
                FieldDef {
                    id: "amount",
                    name: "Amount",
                    field_type: FieldType::Numeric,
                    can_group: false,
                    can_aggregate: true,
                    can_filter: false,
                    db_column: "amount",
                    ref_table: None,
                    ref_display_column: None,
                    source_table: None,
                    join_on_column: None,
                },
            ],
            schema_filters: &[],
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
            sort: DashboardSort {
                rules: vec![SortRule {
                    field_id: "amount_sum".to_string(),
                    direction: SortDirection::Desc,
                }],
            },
            filters: DashboardFilters::default(),
        };

        let schema: DataSourceSchemaOwned = (&schema).into();
        let builder = QueryBuilder::new(&schema, &config, "test_table".to_string());
        let result = builder.build().expect("metric-alias sort should build");

        assert!(
            result.sql.contains("ORDER BY amount DESC"),
            "sql = {}",
            result.sql
        );
    }

    #[test]
    fn test_date_filters_use_date_part_for_inclusive_period_end() {
        let schema = DataSourceSchema {
            id: "test_table",
            name: "Test Table",
            fields: &[FieldDef {
                id: "date",
                name: "Date",
                field_type: FieldType::Date,
                can_group: true,
                can_aggregate: false,
                can_filter: true,
                db_column: "date",
                ref_table: None,
                ref_display_column: None,
                source_table: None,
                join_on_column: None,
            }],
            schema_filters: &[],
        };

        let config = DashboardConfig {
            data_source: "test_table".to_string(),
            selected_fields: vec![],
            groupings: vec!["date".to_string()],
            display_fields: vec![],
            enabled_fields: vec!["date".to_string()],
            sort: DashboardSort::default(),
            filters: DashboardFilters {
                date_from: Some("2026-03-01".to_string()),
                date_to: Some("2026-03-31".to_string()),
                ..DashboardFilters::default()
            },
        };

        let schema: DataSourceSchemaOwned = (&schema).into();
        let builder = QueryBuilder::new(&schema, &config, "test_table".to_string());
        let result = builder.build().unwrap();

        assert!(result.sql.contains("substr(test_table.date, 1, 10) >= ?"));
        assert!(result.sql.contains("substr(test_table.date, 1, 10) <= ?"));
    }

    /// Regression: a field that lives in a joined `source_table` and is referenced
    /// ONLY by a filter (not grouped/displayed) must still emit its LEFT JOIN.
    /// Previously this produced SQL referencing a table absent from FROM, which
    /// SQLite rejected with "no such column: a006_connection_mp.marketplace"
    /// (see chat a018 2c9d07fa…).
    #[test]
    fn filter_only_joined_field_emits_join() {
        let schema = DataSourceSchema {
            id: "p904_sales_data",
            name: "Sales",
            fields: &[
                // Category dimension lives in a004_nomenclature (grouped).
                FieldDef {
                    id: "dim1",
                    name: "Category",
                    field_type: FieldType::Text,
                    can_group: true,
                    can_aggregate: false,
                    can_filter: false,
                    db_column: "dim1_category",
                    ref_table: None,
                    ref_display_column: None,
                    source_table: Some("a004_nomenclature"),
                    join_on_column: Some("nomenclature_ref"),
                },
                // Marketplace lives in a006_connection_mp and is used ONLY as a filter.
                FieldDef {
                    id: "marketplace",
                    name: "Marketplace",
                    field_type: FieldType::Text,
                    can_group: true,
                    can_aggregate: false,
                    can_filter: true,
                    db_column: "marketplace",
                    ref_table: None,
                    ref_display_column: None,
                    source_table: Some("a006_connection_mp"),
                    join_on_column: Some("connection_mp_ref"),
                },
                FieldDef {
                    id: "customer_in",
                    name: "Revenue",
                    field_type: FieldType::Numeric,
                    can_group: false,
                    can_aggregate: true,
                    can_filter: false,
                    db_column: "customer_in",
                    ref_table: None,
                    ref_display_column: None,
                    source_table: None,
                    join_on_column: None,
                },
            ],
            schema_filters: &[],
        };

        let config = DashboardConfig {
            data_source: "p904_sales_data".to_string(),
            selected_fields: vec![SelectedField {
                field_id: "customer_in".to_string(),
                aggregate: Some(AggregateFunction::Sum),
            }],
            groupings: vec!["dim1".to_string()],
            display_fields: vec![],
            enabled_fields: vec![],
            sort: DashboardSort::default(),
            filters: DashboardFilters {
                conditions: vec![FilterCondition::new(
                    "marketplace".to_string(),
                    ValueType::Text,
                    ConditionDef::Comparison {
                        operator: ComparisonOp::Eq,
                        value: "Wildberries".to_string(),
                    },
                )],
                ..DashboardFilters::default()
            },
        };

        let schema: DataSourceSchemaOwned = (&schema).into();
        let builder = QueryBuilder::new(&schema, &config, "p904_sales_data".to_string());
        let result = builder.build().unwrap();

        // JOIN for the grouped category (a004) — existing behaviour.
        assert!(
            result.sql.contains(
                "LEFT JOIN a004_nomenclature ON p904_sales_data.nomenclature_ref = a004_nomenclature.id"
            ),
            "missing a004 join; sql = {}",
            result.sql
        );
        // JOIN for the filter-only marketplace (a006) — the regression we fixed.
        assert!(
            result.sql.contains(
                "LEFT JOIN a006_connection_mp ON p904_sales_data.connection_mp_ref = a006_connection_mp.id"
            ),
            "missing a006 join for filter-only field; sql = {}",
            result.sql
        );
        // The WHERE references the joined column, which is now in scope.
        assert!(
            result.sql.contains("a006_connection_mp.marketplace"),
            "sql = {}",
            result.sql
        );
    }

    #[test]
    fn rejects_invalid_field_roles() {
        let schema = DataSourceSchema {
            id: "test_table",
            name: "Test Table",
            fields: &[
                FieldDef {
                    id: "category",
                    name: "Category",
                    field_type: FieldType::Text,
                    can_group: true,
                    can_aggregate: false,
                    can_filter: true,
                    db_column: "category",
                    ref_table: None,
                    ref_display_column: None,
                    source_table: None,
                    join_on_column: None,
                },
                FieldDef {
                    id: "amount",
                    name: "Amount",
                    field_type: FieldType::Numeric,
                    can_group: false,
                    can_aggregate: true,
                    can_filter: false,
                    db_column: "amount",
                    ref_table: None,
                    ref_display_column: None,
                    source_table: None,
                    join_on_column: None,
                },
            ],
            schema_filters: &[],
        };
        let schema: DataSourceSchemaOwned = (&schema).into();
        let config = DashboardConfig {
            data_source: "test_table".to_string(),
            groupings: vec!["amount".to_string()],
            ..DashboardConfig::default()
        };
        let error = QueryBuilder::new(&schema, &config, "test_table".to_string())
            .build()
            .unwrap_err();
        assert!(error.contains("cannot be used for grouping"));
    }

    #[test]
    fn applies_explicit_sort_direction() {
        let schema = DataSourceSchema {
            id: "test_table",
            name: "Test Table",
            fields: &[FieldDef {
                id: "amount",
                name: "Amount",
                field_type: FieldType::Numeric,
                can_group: false,
                can_aggregate: true,
                can_filter: false,
                db_column: "amount",
                ref_table: None,
                ref_display_column: None,
                source_table: None,
                join_on_column: None,
            }],
            schema_filters: &[],
        };
        let schema: DataSourceSchemaOwned = (&schema).into();
        let config = DashboardConfig {
            data_source: "test_table".to_string(),
            selected_fields: vec![SelectedField {
                field_id: "amount".to_string(),
                aggregate: Some(AggregateFunction::Sum),
            }],
            sort: DashboardSort {
                rules: vec![SortRule {
                    field_id: "amount".to_string(),
                    direction: SortDirection::Desc,
                }],
            },
            ..DashboardConfig::default()
        };
        let sql = QueryBuilder::new(&schema, &config, "test_table".to_string())
            .build()
            .unwrap()
            .sql;
        assert!(sql.contains("ORDER BY amount DESC"), "got: {sql}");
    }
}
