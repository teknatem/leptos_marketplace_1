//! Schema registry for pivot tables
//!
//! Central registry that combines auto-generated schemas from metadata
//! and custom schemas defined in code.

use std::collections::HashMap;

use contracts::shared::metadata::{EntityMetadataInfo, FieldMetadata};
use contracts::shared::universal_dashboard::{
    DataSourceSchema, DataSourceSchemaOwned, SchemaInfo, SchemaSource,
};

use super::metadata_converter::{metadata_to_pivot_schema, RefResolver};
use crate::data_schemes::ds01_wb_finance_report::schema::{DS01_SCHEMA, DS01_TABLE_NAME};
use crate::data_schemes::ds02_mp_sales_register::schema::{DS02_SCHEMA, DS02_TABLE_NAME};
use crate::data_schemes::ds03_p904_sales::schema::{DS03_SCHEMA, DS03_TABLE_NAME};
use contracts::domain::a002_organization::{ENTITY_METADATA as A002_META, FIELDS as A002_FIELDS};
use contracts::domain::a004_nomenclature::{ENTITY_METADATA as A004_META, FIELDS as A004_FIELDS};
use contracts::domain::a005_marketplace::{ENTITY_METADATA as A005_META, FIELDS as A005_FIELDS};
use contracts::domain::a006_connection_mp::{ENTITY_METADATA as A006_META, FIELDS as A006_FIELDS};
use contracts::domain::a012_wb_sales::{ENTITY_METADATA as A012_META, FIELDS as A012_FIELDS};
use contracts::domain::a036_wb_sales_funnel_daily::{
    ENTITY_METADATA as A036_META, FIELDS as A036_FIELDS,
};
use contracts::domain::a037_wb_product_snapshot::{
    ENTITY_METADATA as A037_META, FIELDS as A037_FIELDS,
};

/// Information about a registered entity with metadata
pub struct RegisteredEntity {
    pub entity: &'static EntityMetadataInfo,
    pub fields: &'static [FieldMetadata],
}

/// Schema registry combining auto and custom schemas
pub struct SchemaRegistry {
    /// Auto-generated schemas from entity metadata
    auto_schemas: HashMap<String, RegisteredEntity>,
    /// Custom schemas defined in code
    custom_schemas: HashMap<String, CustomSchemaEntry>,
}

/// Entry for a custom schema
struct CustomSchemaEntry {
    schema: &'static DataSourceSchema,
    table_name: &'static str,
}

impl SchemaRegistry {
    fn canonical_schema_id(id: &str) -> &str {
        match id {
            "p903_wb_finance_report" | "s001_wb_finance" => "ds01_wb_finance_report",
            "p900_sales_register" => "ds02_mp_sales_register",
            _ => id,
        }
    }

    /// Create a new registry with all available schemas
    pub fn new() -> Self {
        let mut registry = Self {
            auto_schemas: HashMap::new(),
            custom_schemas: HashMap::new(),
        };

        // Register custom schemas
        registry.register_custom_schema(&DS01_SCHEMA, DS01_TABLE_NAME);
        registry.register_custom_schema(&DS02_SCHEMA, DS02_TABLE_NAME);
        registry.register_custom_schema(&DS03_SCHEMA, DS03_TABLE_NAME);

        // Only explicitly approved metadata projections are executable. Connection
        // entities with credentials are never auto-registered wholesale.
        registry.register_auto_schemas();

        registry
    }

    /// Register custom schema
    fn register_custom_schema(
        &mut self,
        schema: &'static DataSourceSchema,
        table_name: &'static str,
    ) {
        self.custom_schemas.insert(
            schema.id.to_string(),
            CustomSchemaEntry { schema, table_name },
        );
    }

    /// Register auto schemas from entities with metadata
    fn register_auto_schemas(&mut self) {
        self.register_metadata_schema(&A002_META, A002_FIELDS);
        self.register_metadata_schema(&A004_META, A004_FIELDS);
        self.register_metadata_schema(&A005_META, A005_FIELDS);
        self.register_metadata_schema(&A006_META, A006_FIELDS);
        self.register_metadata_schema(&A012_META, A012_FIELDS);
        // WB daily snapshots: flat daily totals per cabinet/date exposed as a base schema.
        // Per-nomenclature detail (lines_json) is visible_in_list=false and thus excluded
        // here — that detail is reachable via raw SQL + json_each (see field ai_hint).
        self.register_metadata_schema(&A036_META, A036_FIELDS);
        self.register_metadata_schema(&A037_META, A037_FIELDS);
    }

    fn register_metadata_schema(
        &mut self,
        entity: &'static EntityMetadataInfo,
        fields: &'static [FieldMetadata],
    ) {
        if entity.table_name.is_some() {
            self.auto_schemas.insert(
                entity.entity_index.to_string(),
                RegisteredEntity { entity, fields },
            );
        }
    }

    /// List all available schemas
    pub fn list_all(&self) -> Vec<SchemaInfo> {
        let mut result = Vec::new();

        // Add custom schemas
        for (id, entry) in &self.custom_schemas {
            result.push(SchemaInfo {
                id: id.clone(),
                name: entry.schema.name.to_string(),
                source: SchemaSource::Custom,
                table_name: entry.table_name.to_string(),
            });
        }

        // Add auto schemas
        for (id, entry) in &self.auto_schemas {
            if let Some(table_name) = entry.entity.table_name {
                result.push(SchemaInfo {
                    id: id.clone(),
                    name: entry.entity.ui.list_name.to_string(),
                    source: SchemaSource::Auto,
                    table_name: table_name.to_string(),
                });
            }
        }

        // Sort by id
        result.sort_by(|a, b| a.id.cmp(&b.id));
        result
    }

    /// Get schema by ID
    pub fn get_schema(&self, id: &str) -> Option<DataSourceSchemaOwned> {
        let id = Self::canonical_schema_id(id);
        // Check custom schemas first
        if let Some(entry) = self.custom_schemas.get(id) {
            return Some(entry.schema.into());
        }

        // Check auto schemas
        if let Some(entry) = self.auto_schemas.get(id) {
            return Some(metadata_to_pivot_schema(entry.entity, entry.fields, self));
        }

        None
    }

    /// Get table name for schema
    pub fn get_table_name(&self, schema_id: &str) -> Option<String> {
        let schema_id = Self::canonical_schema_id(schema_id);
        // Check custom schemas
        if let Some(entry) = self.custom_schemas.get(schema_id) {
            return Some(entry.table_name.to_string());
        }

        // Check auto schemas
        if let Some(entry) = self.auto_schemas.get(schema_id) {
            return entry.entity.table_name.map(|s| s.to_string());
        }

        None
    }

    /// Check if schema exists
    pub fn has_schema(&self, id: &str) -> bool {
        let id = Self::canonical_schema_id(id);
        self.custom_schemas.contains_key(id) || self.auto_schemas.contains_key(id)
    }

    /// Get entity metadata for auto schema
    pub fn get_entity_metadata(&self, id: &str) -> Option<&RegisteredEntity> {
        self.auto_schemas.get(id)
    }
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Implement RefResolver for SchemaRegistry
impl RefResolver for SchemaRegistry {
    fn resolve_ref(&self, aggregate_index: &str) -> (Option<String>, Option<String>) {
        // Look up the referenced aggregate in auto schemas
        if let Some(entry) = self.auto_schemas.get(aggregate_index) {
            let table = entry.entity.table_name.map(|s| s.to_string());
            // Standard display column is "description"
            let display = Some("description".to_string());
            return (table, display);
        }

        // Try to construct table name from index
        // Format: a001 -> a001_<collection_name>
        // This is a fallback for aggregates without metadata
        let table_name = match aggregate_index {
            "a002" => Some("a002_organization".to_string()),
            "a003" => Some("a003_counterparty".to_string()),
            "a004" => Some("a004_nomenclature".to_string()),
            "a005" => Some("a005_marketplace".to_string()),
            "a006" => Some("a006_connection_mp".to_string()),
            _ => None,
        };

        (table_name, Some("description".to_string()))
    }
}

/// Global schema registry instance
static REGISTRY: std::sync::OnceLock<SchemaRegistry> = std::sync::OnceLock::new();

/// Get global schema registry
pub fn get_registry() -> &'static SchemaRegistry {
    REGISTRY.get_or_init(SchemaRegistry::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_custom_schema() {
        let registry = SchemaRegistry::new();
        assert!(registry.has_schema("ds01_wb_finance_report"));
        assert!(registry.has_schema("ds02_mp_sales_register"));
        assert!(registry.has_schema("a012"));
    }

    #[test]
    fn test_registry_list_schemas() {
        let registry = SchemaRegistry::new();
        let schemas = registry.list_all();

        // Should have at least the custom schema
        assert!(!schemas.is_empty());

        // Find ds01
        let ds01 = schemas.iter().find(|s| s.id == "ds01_wb_finance_report");
        assert!(ds01.is_some());
        assert_eq!(ds01.unwrap().source, SchemaSource::Custom);
    }

    #[test]
    fn test_get_custom_schema() {
        let registry = SchemaRegistry::new();
        let schema = registry.get_schema("ds01_wb_finance_report");

        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert_eq!(schema.id, "ds01_wb_finance_report");
        assert!(!schema.fields.is_empty());
    }
}
