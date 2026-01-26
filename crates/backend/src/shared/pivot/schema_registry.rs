use contracts::shared::pivot::DataSourceSchema;
use std::collections::HashMap;

/// Registry of all available data source schemas
pub struct SchemaRegistry {
    schemas: HashMap<String, &'static DataSourceSchema>,
}

impl SchemaRegistry {
    /// Create a new schema registry
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    /// Register a schema
    pub fn register(&mut self, schema: &'static DataSourceSchema) {
        self.schemas.insert(schema.id.to_string(), schema);
    }

    /// Get a schema by ID
    pub fn get(&self, id: &str) -> Option<&'static DataSourceSchema> {
        self.schemas.get(id).copied()
    }

    /// List all available schemas
    pub fn list(&self) -> Vec<&'static DataSourceSchema> {
        self.schemas.values().copied().collect()
    }
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}
