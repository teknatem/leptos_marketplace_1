//! ViewModel for BiIndicator details form (EditDetails MVVM Standard)

use super::model::{self, BiIndicatorSaveDto};
use leptos::prelude::*;

#[derive(Clone)]
pub struct BiIndicatorDetailsVm {
    // === Form fields ===
    pub id: RwSignal<Option<String>>,
    pub code: RwSignal<String>,
    pub description: RwSignal<String>,
    pub comment: RwSignal<String>,
    pub status: RwSignal<String>,
    pub owner_user_id: RwSignal<String>,
    pub is_public: RwSignal<bool>,
    pub version: RwSignal<i64>,

    // DataSpec fields
    pub data_spec_schema_id: RwSignal<String>,
    pub data_spec_sql_artifact_id: RwSignal<String>,
    pub data_spec_query_config_json: RwSignal<String>,

    // Params (whole Vec<ParamDef> as JSON string)
    pub params_json: RwSignal<String>,

    // ViewSpec fields
    pub view_spec_custom_html: RwSignal<String>,
    pub view_spec_custom_css: RwSignal<String>,
    pub view_spec_format_json: RwSignal<String>,
    pub view_spec_thresholds_json: RwSignal<String>,

    // DrillSpec (whole Option<DrillSpec> as JSON string, empty = None)
    pub drill_spec_json: RwSignal<String>,

    // Meta (read-only display)
    pub created_at: RwSignal<String>,
    pub updated_at: RwSignal<String>,
    pub created_by: RwSignal<String>,
    pub updated_by: RwSignal<String>,

    // === UI State ===
    pub active_tab: RwSignal<&'static str>,
    pub loading: RwSignal<bool>,
    pub saving: RwSignal<bool>,
    pub error: RwSignal<Option<String>>,
    pub success: RwSignal<Option<String>>,
}

impl BiIndicatorDetailsVm {
    pub fn new() -> Self {
        Self {
            id: RwSignal::new(None),
            code: RwSignal::new(String::new()),
            description: RwSignal::new(String::new()),
            comment: RwSignal::new(String::new()),
            status: RwSignal::new("draft".to_string()),
            owner_user_id: RwSignal::new(String::new()),
            is_public: RwSignal::new(false),
            version: RwSignal::new(1),

            data_spec_schema_id: RwSignal::new(String::new()),
            data_spec_sql_artifact_id: RwSignal::new(String::new()),
            data_spec_query_config_json: RwSignal::new("{}".to_string()),

            params_json: RwSignal::new("[]".to_string()),

            view_spec_custom_html: RwSignal::new(String::new()),
            view_spec_custom_css: RwSignal::new(String::new()),
            view_spec_format_json: RwSignal::new("{}".to_string()),
            view_spec_thresholds_json: RwSignal::new("[]".to_string()),

            drill_spec_json: RwSignal::new(String::new()),

            created_at: RwSignal::new(String::new()),
            updated_at: RwSignal::new(String::new()),
            created_by: RwSignal::new(String::new()),
            updated_by: RwSignal::new(String::new()),

            active_tab: RwSignal::new("general"),
            loading: RwSignal::new(false),
            saving: RwSignal::new(false),
            error: RwSignal::new(None),
            success: RwSignal::new(None),
        }
    }

    // === Derived signals ===

    pub fn is_edit_mode(&self) -> Signal<bool> {
        let id = self.id;
        Signal::derive(move || id.get().is_some())
    }

    pub fn is_valid(&self) -> Signal<bool> {
        let description = self.description;
        Signal::derive(move || !description.get().trim().is_empty())
    }

    pub fn is_save_disabled(&self) -> Signal<bool> {
        let saving = self.saving;
        let is_valid = self.is_valid();
        Signal::derive(move || saving.get() || !is_valid.get())
    }

    // === Validation ===

    pub fn validate(&self) -> Result<(), String> {
        if self.description.get().trim().is_empty() {
            return Err("Наименование обязательно для заполнения".into());
        }
        Ok(())
    }

    // === Data loading ===

    pub fn load(&self, id: String) {
        let this = self.clone();
        this.loading.set(true);
        this.error.set(None);

        leptos::task::spawn_local(async move {
            match model::fetch_by_id(&id).await {
                Ok(item) => {
                    this.from_raw(&item);
                    this.loading.set(false);
                }
                Err(e) => {
                    this.error.set(Some(e));
                    this.loading.set(false);
                }
            }
        });
    }

    // === Commands ===

    pub fn save(&self, on_saved: Callback<()>) {
        if let Err(msg) = self.validate() {
            self.error.set(Some(msg));
            return;
        }

        let this = self.clone();
        this.saving.set(true);
        this.error.set(None);

        let dto = this.to_dto();

        leptos::task::spawn_local(async move {
            match model::save_indicator(dto).await {
                Ok(new_id) => {
                    if this.id.get().is_none() {
                        this.id.set(Some(new_id));
                    }
                    this.saving.set(false);
                    this.success.set(Some("Сохранено успешно".into()));
                    on_saved.run(());
                }
                Err(e) => {
                    this.saving.set(false);
                    this.error.set(Some(e));
                }
            }
        });
    }

    // === Tab helpers ===

    pub fn set_tab(&self, tab: &'static str) {
        self.active_tab.set(tab);
    }

    // === Private helpers ===

    fn to_dto(&self) -> BiIndicatorSaveDto {
        let drill_spec_raw = self.drill_spec_json.get();
        let drill_spec_json = if drill_spec_raw.trim().is_empty() || drill_spec_raw.trim() == "null" {
            None
        } else {
            Some(drill_spec_raw)
        };

        BiIndicatorSaveDto {
            id: self.id.get(),
            code: self.code.get(),
            description: self.description.get(),
            comment: {
                let c = self.comment.get();
                if c.trim().is_empty() { None } else { Some(c) }
            },
            status: self.status.get(),
            owner_user_id: self.owner_user_id.get(),
            is_public: self.is_public.get(),
            version: self.version.get(),
            data_spec_schema_id: self.data_spec_schema_id.get(),
            data_spec_sql_artifact_id: {
                let s = self.data_spec_sql_artifact_id.get();
                if s.trim().is_empty() { None } else { Some(s) }
            },
            data_spec_query_config_json: self.data_spec_query_config_json.get(),
            params_json: self.params_json.get(),
            view_spec_custom_html: {
                let h = self.view_spec_custom_html.get();
                if h.trim().is_empty() { None } else { Some(h) }
            },
            view_spec_custom_css: {
                let c = self.view_spec_custom_css.get();
                if c.trim().is_empty() { None } else { Some(c) }
            },
            view_spec_format_json: self.view_spec_format_json.get(),
            view_spec_thresholds_json: self.view_spec_thresholds_json.get(),
            drill_spec_json,
        }
    }

    fn from_raw(&self, v: &serde_json::Value) {
        self.id.set(v["id"].as_str().map(|s| s.to_string()));
        self.code.set(v["code"].as_str().unwrap_or("").to_string());
        self.description.set(v["description"].as_str().unwrap_or("").to_string());
        self.comment.set(v["comment"].as_str().unwrap_or("").to_string());
        self.status.set(v["status"].as_str().unwrap_or("draft").to_string());
        self.owner_user_id.set(v["owner_user_id"].as_str().unwrap_or("").to_string());
        self.is_public.set(v["is_public"].as_bool().unwrap_or(false));
        self.version.set(v["version"].as_i64().unwrap_or(1));

        // DataSpec
        if let Some(ds) = v.get("data_spec") {
            self.data_spec_schema_id.set(ds["schema_id"].as_str().unwrap_or("").to_string());
            self.data_spec_sql_artifact_id.set(
                ds["sql_artifact_id"].as_str().unwrap_or("").to_string()
            );
            self.data_spec_query_config_json.set(
                serde_json::to_string_pretty(ds.get("query_config").unwrap_or(&serde_json::json!({})))
                    .unwrap_or_else(|_| "{}".to_string())
            );
        }

        // Params
        if let Some(params) = v.get("params") {
            self.params_json.set(
                serde_json::to_string_pretty(params).unwrap_or_else(|_| "[]".to_string())
            );
        }

        // ViewSpec
        if let Some(vs) = v.get("view_spec") {
            self.view_spec_custom_html.set(vs["custom_html"].as_str().unwrap_or("").to_string());
            self.view_spec_custom_css.set(vs["custom_css"].as_str().unwrap_or("").to_string());
            self.view_spec_format_json.set(
                serde_json::to_string_pretty(vs.get("format").unwrap_or(&serde_json::json!({})))
                    .unwrap_or_else(|_| "{}".to_string())
            );
            self.view_spec_thresholds_json.set(
                serde_json::to_string_pretty(vs.get("thresholds").unwrap_or(&serde_json::json!([])))
                    .unwrap_or_else(|_| "[]".to_string())
            );
        }

        // DrillSpec
        if let Some(drill) = v.get("drill_spec") {
            if drill.is_null() {
                self.drill_spec_json.set(String::new());
            } else {
                self.drill_spec_json.set(
                    serde_json::to_string_pretty(drill).unwrap_or_default()
                );
            }
        } else {
            self.drill_spec_json.set(String::new());
        }

        // Meta
        self.created_at.set(v["created_at"].as_str().unwrap_or("").to_string());
        self.updated_at.set(v["updated_at"].as_str().unwrap_or("").to_string());
        self.created_by.set(v["created_by"].as_str().unwrap_or("").to_string());
        self.updated_by.set(v["updated_by"].as_str().unwrap_or("").to_string());
    }
}

impl Default for BiIndicatorDetailsVm {
    fn default() -> Self {
        Self::new()
    }
}
