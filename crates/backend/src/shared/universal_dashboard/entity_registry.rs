//! Schema registry for pivot tables
//!
//! Central registry that combines auto-generated schemas from metadata
//! and custom schemas defined in code.

use std::collections::HashMap;

use contracts::shared::metadata::{EntityMetadataInfo, FieldMetadata};
use contracts::shared::universal_dashboard::{DataSourceSchema, DataSourceSchemaOwned, SchemaInfo, SchemaSource};

use super::metadata_converter::{metadata_to_pivot_schema, RefResolver};
use super::schemas;

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
    /// Create a new registry with all available schemas
    pub fn new() -> Self {
        let mut registry = Self {
            auto_schemas: HashMap::new(),
            custom_schemas: HashMap::new(),
        };

        // Register custom schemas
        registry.register_custom_schema(
            &schemas::S001_WB_FINANCE_SCHEMA,
            schemas::s001_wb_finance::S001_TABLE_NAME,
        );

        // Register auto schemas from metadata
        // Currently only a001, a017, a018, a019 have metadata
        registry.register_auto_schemas();

        registry
    }

    /// Register custom schema
    fn register_custom_schema(&mut self, schema: &'static DataSourceSchema, table_name: &'static str) {
        self.custom_schemas.insert(
            schema.id.to_string(),
            CustomSchemaEntry { schema, table_name },
        );
    }

    /// Register auto schemas from entities with metadata
    fn register_auto_schemas(&mut self) {
        // Import entities with metadata
        use contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase;

        // Register each entity that has metadata
        // Note: a017-a019 have metadata.json but don't export metadata_gen yet
        self.register_entity_if_available::<Connection1CDatabase>();
    }

    /// Register entity if it has metadata
    fn register_entity_if_available<T>(&mut self)
    where
        T: contracts::domain::common::AggregateRoot,
    {
        if let (Some(entity), Some(fields)) = (T::entity_metadata_info(), T::field_metadata()) {
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
        assert!(registry.has_schema("s001_wb_finance"));
    }

    #[test]
    fn test_registry_list_schemas() {
        let registry = SchemaRegistry::new();
        let schemas = registry.list_all();
        
        // Should have at least the custom schema
        assert!(!schemas.is_empty());
        
        // Find s001
        let s001 = schemas.iter().find(|s| s.id == "s001_wb_finance");
        assert!(s001.is_some());
        assert_eq!(s001.unwrap().source, SchemaSource::Custom);
    }

    #[test]
    fn test_get_custom_schema() {
        let registry = SchemaRegistry::new();
        let schema = registry.get_schema("s001_wb_finance");
        
        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert_eq!(schema.id, "s001_wb_finance");
        assert!(!schema.fields.is_empty());
    }
}
