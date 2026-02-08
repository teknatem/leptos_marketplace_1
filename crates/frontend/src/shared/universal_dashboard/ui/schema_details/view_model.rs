//! ViewModel for schema details

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::logging::log;
use contracts::shared::universal_dashboard::{
    DataSourceSchemaOwned, GenerateSqlResponse, SavedDashboardConfigSummary,
    SchemaValidationResult,
};

use crate::shared::universal_dashboard::api;

/// ViewModel for SchemaDetails component
#[derive(Clone)]
pub struct SchemaDetailsVm {
    /// Schema ID
    pub schema_id: RwSignal<String>,
    /// Schema data
    pub schema: RwSignal<Option<DataSourceSchemaOwned>>,
    /// Active tab
    pub active_tab: RwSignal<&'static str>,
    /// Loading state
    pub loading: RwSignal<bool>,
    /// Error message
    pub error: RwSignal<Option<String>>,

    // For Settings tab
    /// Saved dashboard configurations
    pub saved_configs: RwSignal<Vec<SavedDashboardConfigSummary>>,
    /// Loading configs
    pub configs_loading: RwSignal<bool>,
    /// Flag to track if configs were loaded
    pub configs_loaded: RwSignal<bool>,

    // For SQL tab
    /// Generated SQL query
    pub generated_sql: RwSignal<Option<GenerateSqlResponse>>,
    /// Flag to track if SQL was generated
    pub sql_loaded: RwSignal<bool>,

    // For Test tab
    /// Test result
    pub test_result: RwSignal<Option<SchemaValidationResult>>,
    /// Testing in progress
    pub testing: RwSignal<bool>,
}

impl SchemaDetailsVm {
    /// Create new ViewModel
    pub fn new() -> Self {
        Self {
            schema_id: RwSignal::new(String::new()),
            schema: RwSignal::new(None),
            active_tab: RwSignal::new("fields"),
            loading: RwSignal::new(true),
            error: RwSignal::new(None),
            saved_configs: RwSignal::new(Vec::new()),
            configs_loading: RwSignal::new(false),
            configs_loaded: RwSignal::new(false),
            generated_sql: RwSignal::new(None),
            sql_loaded: RwSignal::new(false),
            test_result: RwSignal::new(None),
            testing: RwSignal::new(false),
        }
    }

    /// Load schema data
    pub fn load(&self, schema_id: String) {
        self.schema_id.set(schema_id.clone());
        self.loading.set(true);
        self.error.set(None);

        let schema = self.schema;
        let loading = self.loading;
        let error = self.error;

        spawn_local(async move {
            match api::get_schema(&schema_id).await {
                Ok(response) => {
                    schema.set(Some(response.schema));
                    loading.set(false);
                }
                Err(e) => {
                    log!("Failed to load schema: {}", e);
                    error.set(Some(format!("Ошибка загрузки схемы: {}", e)));
                    loading.set(false);
                }
            }
        });
    }

    /// Load saved configurations
    pub fn load_configs(&self) {
        if self.configs_loaded.get() {
            return; // Already loaded
        }

        let schema_id = self.schema_id.get();
        self.configs_loading.set(true);

        let saved_configs = self.saved_configs;
        let configs_loading = self.configs_loading;
        let configs_loaded = self.configs_loaded;
        let error = self.error;

        spawn_local(async move {
            match api::list_configs(Some(&schema_id)).await {
                Ok(response) => {
                    saved_configs.set(response.configs);
                    configs_loaded.set(true);
                }
                Err(e) => {
                    log!("Failed to load configs: {}", e);
                    error.set(Some(format!("Ошибка загрузки конфигураций: {}", e)));
                }
            }
            configs_loading.set(false);
        });
    }

    /// Generate SQL example
    pub fn generate_sql(&self) {
        if self.sql_loaded.get() {
            return; // Already generated
        }

        let schema_opt = self.schema.get();
        if schema_opt.is_none() {
            return;
        }

        let schema = schema_opt.unwrap();
        
        // Create test config with all fields
        use contracts::shared::universal_dashboard::{
            AggregateFunction, DashboardConfig, DashboardFilters, DashboardSort, SelectedField, ValueType,
        };

        let mut selected_fields = Vec::new();
        let mut groupings = Vec::new();
        let mut enabled_fields = Vec::new();

        // Add grouping fields
        for field in &schema.fields {
            if field.can_group {
                groupings.push(field.id.clone());
                enabled_fields.push(field.id.clone());
            }
        }

        // Add aggregated numeric fields
        for field in &schema.fields {
            if field.can_aggregate {
                let is_numeric = matches!(
                    field.get_value_type(),
                    ValueType::Integer | ValueType::Numeric
                );
                
                if is_numeric {
                    selected_fields.push(SelectedField {
                        field_id: field.id.clone(),
                        aggregate: Some(AggregateFunction::Sum),
                    });
                    enabled_fields.push(field.id.clone());
                }
            }
        }

        let config = DashboardConfig {
            data_source: schema.id.clone(),
            selected_fields,
            groupings,
            display_fields: Vec::new(),
            filters: DashboardFilters::default(),
            sort: DashboardSort::default(),
            enabled_fields,
        };

        let generated_sql = self.generated_sql;
        let sql_loaded = self.sql_loaded;
        let error = self.error;

        spawn_local(async move {
            match api::generate_sql(config).await {
                Ok(response) => {
                    generated_sql.set(Some(response));
                    sql_loaded.set(true);
                }
                Err(e) => {
                    log!("Failed to generate SQL: {}", e);
                    error.set(Some(format!("Ошибка генерации SQL: {}", e)));
                }
            }
        });
    }

    /// Run validation test
    pub fn run_test(&self) {
        let schema_id = self.schema_id.get();
        self.testing.set(true);
        self.test_result.set(None);

        let test_result = self.test_result;
        let testing = self.testing;
        let error = self.error;

        spawn_local(async move {
            match api::validate_schema(&schema_id).await {
                Ok(result) => {
                    test_result.set(Some(result));
                }
                Err(e) => {
                    log!("Failed to validate schema: {}", e);
                    error.set(Some(format!("Ошибка тестирования: {}", e)));
                }
            }
            testing.set(false);
        });
    }

    /// Delete a saved configuration
    pub fn delete_config(&self, config_id: String) {
        let saved_configs = self.saved_configs;
        let error = self.error;

        spawn_local(async move {
            match api::delete_config(&config_id).await {
                Ok(_) => {
                    // Remove from list
                    saved_configs.update(|configs| {
                        configs.retain(|c| c.id != config_id);
                    });
                }
                Err(e) => {
                    log!("Failed to delete config: {}", e);
                    error.set(Some(format!("Ошибка удаления конфигурации: {}", e)));
                }
            }
        });
    }

    /// Get schema name (for display)
    pub fn schema_name(&self) -> impl Fn() -> String + 'static {
        let schema = self.schema;
        move || {
            schema
                .get()
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "...".to_string())
        }
    }
}
