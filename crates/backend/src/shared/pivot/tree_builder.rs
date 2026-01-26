use contracts::shared::pivot::{CellValue, ColumnHeader, ColumnType, PivotRow};
use std::collections::HashMap;

/// Raw row from database query
#[derive(Debug, Clone)]
pub struct RawRow {
    /// Column values by column ID
    pub values: HashMap<String, CellValue>,
}

/// Tree builder for transforming flat results to hierarchical pivot structure
pub struct TreeBuilder {
    /// Grouping column IDs (in order)
    grouping_columns: Vec<String>,
    /// Aggregated column IDs
    aggregated_columns: Vec<String>,
}

impl TreeBuilder {
    /// Create a new tree builder
    pub fn new(grouping_columns: Vec<String>, aggregated_columns: Vec<String>) -> Self {
        Self {
            grouping_columns,
            aggregated_columns,
        }
    }

    /// Build a hierarchical pivot tree from flat rows
    pub fn build(&self, rows: Vec<RawRow>) -> Vec<PivotRow> {
        if rows.is_empty() {
            return vec![];
        }

        // If no groupings, return rows as-is
        if self.grouping_columns.is_empty() {
            return rows
                .into_iter()
                .map(|row| PivotRow {
                    level: 0,
                    values: row.values,
                    is_total: false,
                    children: vec![],
                })
                .collect();
        }

        // Build hierarchical structure
        self.build_recursive(&rows, 0)
    }

    /// Recursively build tree at current level
    fn build_recursive(&self, rows: &[RawRow], level: usize) -> Vec<PivotRow> {
        if level >= self.grouping_columns.len() {
            // Leaf level - return data rows
            return rows
                .iter()
                .map(|row| PivotRow {
                    level,
                    values: row.values.clone(),
                    is_total: false,
                    children: vec![],
                })
                .collect();
        }

        let grouping_col = &self.grouping_columns[level];

        // Group rows by current grouping column
        let mut groups: HashMap<String, Vec<RawRow>> = HashMap::new();
        for row in rows {
            let key = self.get_cell_string(&row.values, grouping_col);
            groups.entry(key).or_insert_with(Vec::new).push(row.clone());
        }

        // Build pivot rows for each group
        let mut result = Vec::new();
        let mut group_keys: Vec<_> = groups.keys().cloned().collect();
        group_keys.sort();

        for group_key in group_keys {
            let group_rows = groups.get(&group_key).unwrap();

            // Create group header row with subtotals
            let mut group_values = HashMap::new();

            // Add grouping column value
            group_values.insert(
                grouping_col.clone(),
                group_rows[0].values.get(grouping_col).cloned().unwrap_or(CellValue::Null),
            );

            // Calculate subtotals for aggregated columns
            for agg_col in &self.aggregated_columns {
                let subtotal = self.calculate_subtotal(group_rows, agg_col);
                group_values.insert(agg_col.clone(), subtotal);
            }

            // Build children recursively
            let children = if level + 1 < self.grouping_columns.len() {
                self.build_recursive(group_rows, level + 1)
            } else {
                vec![]
            };

            result.push(PivotRow {
                level,
                values: group_values,
                is_total: true,
                children,
            });
        }

        result
    }

    /// Calculate subtotal for a column
    fn calculate_subtotal(&self, rows: &[RawRow], column: &str) -> CellValue {
        let mut sum = 0.0;
        let mut count = 0;

        for row in rows {
            if let Some(value) = row.values.get(column) {
                match value {
                    CellValue::Number(n) => {
                        sum += n;
                        count += 1;
                    }
                    CellValue::Integer(i) => {
                        sum += *i as f64;
                        count += 1;
                    }
                    _ => {}
                }
            }
        }

        if count > 0 {
            CellValue::Number(sum)
        } else {
            CellValue::Null
        }
    }

    /// Get string representation of a cell value for grouping
    fn get_cell_string(&self, values: &HashMap<String, CellValue>, column: &str) -> String {
        match values.get(column) {
            Some(CellValue::Text(s)) => s.clone(),
            Some(CellValue::Number(n)) => n.to_string(),
            Some(CellValue::Integer(i)) => i.to_string(),
            Some(CellValue::Null) | None => String::new(),
        }
    }

    /// Calculate grand totals
    pub fn calculate_grand_totals(&self, rows: &[RawRow]) -> HashMap<String, CellValue> {
        let mut totals = HashMap::new();

        for agg_col in &self.aggregated_columns {
            let total = self.calculate_subtotal(rows, agg_col);
            totals.insert(agg_col.clone(), total);
        }

        totals
    }
}

/// Helper to create column headers
pub fn create_column_headers(
    grouping_columns: &[String],
    aggregated_columns: &[(String, String)], // (id, name) pairs
) -> Vec<ColumnHeader> {
    let mut headers = Vec::new();

    // Add grouping column headers
    for col_id in grouping_columns {
        headers.push(ColumnHeader {
            id: col_id.clone(),
            name: col_id.clone(), // Will be replaced with actual names from schema
            column_type: ColumnType::Grouping,
        });
    }

    // Add aggregated column headers
    for (col_id, col_name) in aggregated_columns {
        headers.push(ColumnHeader {
            id: col_id.clone(),
            name: col_name.clone(),
            column_type: ColumnType::Aggregated,
        });
    }

    headers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_grouping() {
        let rows = vec![
            RawRow {
                values: HashMap::from([
                    ("date".to_string(), CellValue::Text("2024-01-01".to_string())),
                    ("amount".to_string(), CellValue::Number(100.0)),
                ]),
            },
            RawRow {
                values: HashMap::from([
                    ("date".to_string(), CellValue::Text("2024-01-01".to_string())),
                    ("amount".to_string(), CellValue::Number(200.0)),
                ]),
            },
            RawRow {
                values: HashMap::from([
                    ("date".to_string(), CellValue::Text("2024-01-02".to_string())),
                    ("amount".to_string(), CellValue::Number(150.0)),
                ]),
            },
        ];

        let builder = TreeBuilder::new(
            vec!["date".to_string()],
            vec!["amount".to_string()],
        );

        let result = builder.build(rows);

        assert_eq!(result.len(), 2); // Two date groups
        assert!(result[0].is_total);
        assert!(result[1].is_total);
    }
}
