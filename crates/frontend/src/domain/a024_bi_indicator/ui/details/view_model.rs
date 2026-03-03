//! ViewModel for BiIndicator details form (EditDetails MVVM Standard)

use super::model::{self, BiIndicatorSaveDto};
use crate::shared::bi_card::{render_srcdoc, IndicatorCardParams};
use crate::shared::code_format;
use leptos::prelude::*;

/// Read the current app theme from localStorage (key "app_theme").
/// Falls back to "dark". Maps "forest" → "dark" since it is a dark-based theme.
fn get_app_theme() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let theme = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item("app_theme").ok().flatten())
            .unwrap_or_else(|| "dark".to_string());
        // forest is a dark-base theme
        if theme == "light" { "light".to_string() } else { "dark".to_string() }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "dark".to_string()
    }
}

#[derive(Clone, Debug)]
pub struct LlmGenerationEntry {
    pub prompt: String,
    pub html: String,
    pub css: String,
    pub explanation: String,
}

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
    pub view_spec_style_name: RwSignal<String>,
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

    // === Preview test data ===
    pub preview_title: RwSignal<String>,
    pub preview_value: RwSignal<String>,
    pub preview_delta: RwSignal<String>,
    pub preview_delta_dir: RwSignal<String>,
    pub preview_status: RwSignal<String>,
    pub preview_chip: RwSignal<String>,
    pub preview_progress: RwSignal<u8>,
    pub preview_size: RwSignal<String>,

    // === LLM generation state ===
    pub llm_prompt: RwSignal<String>,
    pub llm_generating: RwSignal<bool>,
    pub llm_error: RwSignal<Option<String>>,
    pub llm_history: RwSignal<Vec<LlmGenerationEntry>>,
    pub llm_panel_open: RwSignal<bool>,
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

            view_spec_style_name: RwSignal::new("classic".to_string()),
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

            preview_title: RwSignal::new("Выручка".to_string()),
            preview_value: RwSignal::new("₽2.40M".to_string()),
            preview_delta: RwSignal::new("+12.5%".to_string()),
            preview_delta_dir: RwSignal::new("up".to_string()),
            preview_status: RwSignal::new("ok".to_string()),
            preview_chip: RwSignal::new("Выручка".to_string()),
            preview_progress: RwSignal::new(82u8),
            preview_size: RwSignal::new("1x1".to_string()),

            llm_prompt: RwSignal::new(String::new()),
            llm_generating: RwSignal::new(false),
            llm_error: RwSignal::new(None),
            llm_history: RwSignal::new(Vec::new()),
            llm_panel_open: RwSignal::new(false),
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

    /// Build the iframe srcdoc from current ViewSpec + test data
    pub fn build_preview_srcdoc(&self) -> Signal<String> {
        let style_sig = self.view_spec_style_name;
        let html_sig = self.view_spec_custom_html;
        let css_sig = self.view_spec_custom_css;
        let name_sig = self.preview_title;
        let value_sig = self.preview_value;
        let delta_sig = self.preview_delta;
        let delta_dir_sig = self.preview_delta_dir;
        let status_sig = self.preview_status;
        let chip_sig = self.preview_chip;
        let progress_sig = self.preview_progress;

        Signal::derive(move || {
            let style_name = style_sig.get();
            let params = IndicatorCardParams {
                style_name: style_name.clone(),
                theme: get_app_theme(),
                name: name_sig.get(),
                value: value_sig.get(),
                unit: String::new(),
                delta: delta_sig.get(),
                delta_dir: delta_dir_sig.get(),
                status: status_sig.get(),
                chip: chip_sig.get(),
                col_class: String::new(),
                progress: progress_sig.get(),
                spark_points: vec![],
                meta_1: "Период: 30 дней".to_string(),
                meta_2: "Обновлено: 5 мин".to_string(),
                hint: format!("до цели: {}%", progress_sig.get()),
                footer_1: "Источник: Sales".to_string(),
                footer_2: String::new(),
                custom_html: {
                    let h = html_sig.get();
                    if h.trim().is_empty() {
                        None
                    } else {
                        Some(h)
                    }
                },
                custom_css: {
                    let c = css_sig.get();
                    if c.trim().is_empty() {
                        None
                    } else {
                        Some(c)
                    }
                },
            };
            render_srcdoc(&params)
        })
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

    /// Generate view via LLM
    pub fn generate_view(&self) {
        let prompt = self.llm_prompt.get();
        if prompt.trim().is_empty() {
            self.llm_error.set(Some("Введите описание".into()));
            return;
        }

        let this = self.clone();
        this.llm_generating.set(true);
        this.llm_error.set(None);

        let current_html = {
            let h = this.view_spec_custom_html.get();
            if h.trim().is_empty() {
                None
            } else {
                Some(h)
            }
        };
        let current_css = {
            let c = this.view_spec_custom_css.get();
            if c.trim().is_empty() {
                None
            } else {
                Some(c)
            }
        };
        let indicator_description = this.description.get();

        leptos::task::spawn_local(async move {
            match model::generate_view(
                &prompt,
                current_html.as_deref(),
                current_css.as_deref(),
                &indicator_description,
            )
            .await
            {
                Ok(resp) => {
                    this.llm_history.update(|h| {
                        h.push(LlmGenerationEntry {
                            prompt: prompt.clone(),
                            html: resp.custom_html.clone(),
                            css: resp.custom_css.clone(),
                            explanation: resp.explanation.clone(),
                        });
                        if h.len() > 10 {
                            h.remove(0);
                        }
                    });
                    this.llm_generating.set(false);
                    this.llm_prompt.set(String::new());
                }
                Err(e) => {
                    this.llm_error.set(Some(e));
                    this.llm_generating.set(false);
                }
            }
        });
    }

    /// Apply a specific LLM generation entry to the ViewSpec fields
    pub fn apply_generation(&self, entry: &LlmGenerationEntry) {
        self.view_spec_custom_html
            .set(code_format::format_html(&entry.html));
        self.view_spec_custom_css
            .set(code_format::format_css(&entry.css));
    }

    // === Tab helpers ===

    pub fn set_tab(&self, tab: &'static str) {
        self.active_tab.set(tab);
    }

    // === Private helpers ===

    fn to_dto(&self) -> BiIndicatorSaveDto {
        let query_config =
            serde_json::from_str::<serde_json::Value>(&self.data_spec_query_config_json.get())
                .unwrap_or(serde_json::json!({}));

        let sql_artifact_id = {
            let s = self.data_spec_sql_artifact_id.get();
            if s.trim().is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(s)
            }
        };

        let data_spec = serde_json::json!({
            "schema_id": self.data_spec_schema_id.get(),
            "query_config": query_config,
            "sql_artifact_id": sql_artifact_id,
        });

        let params = serde_json::from_str::<serde_json::Value>(&self.params_json.get())
            .unwrap_or(serde_json::json!([]));

        let format = serde_json::from_str::<serde_json::Value>(&self.view_spec_format_json.get())
            .unwrap_or(serde_json::json!({}));

        let thresholds =
            serde_json::from_str::<serde_json::Value>(&self.view_spec_thresholds_json.get())
                .unwrap_or(serde_json::json!([]));

        let custom_html = {
            let h = self.view_spec_custom_html.get();
            if h.trim().is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(h)
            }
        };
        let custom_css = {
            let c = self.view_spec_custom_css.get();
            if c.trim().is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(c)
            }
        };

        let view_spec = serde_json::json!({
            "style_name": self.view_spec_style_name.get(),
            "custom_html": custom_html,
            "custom_css": custom_css,
            "format": format,
            "thresholds": thresholds,
        });

        let drill_spec_raw = self.drill_spec_json.get();
        let drill_spec = if drill_spec_raw.trim().is_empty() || drill_spec_raw.trim() == "null" {
            serde_json::Value::Null
        } else {
            serde_json::from_str::<serde_json::Value>(&drill_spec_raw)
                .unwrap_or(serde_json::Value::Null)
        };

        BiIndicatorSaveDto {
            id: self.id.get(),
            code: self.code.get(),
            description: self.description.get(),
            comment: {
                let c = self.comment.get();
                if c.trim().is_empty() {
                    None
                } else {
                    Some(c)
                }
            },
            status: self.status.get(),
            owner_user_id: self.owner_user_id.get(),
            is_public: self.is_public.get(),
            version: self.version.get(),
            data_spec,
            params,
            view_spec,
            drill_spec,
        }
    }

    fn from_raw(&self, v: &serde_json::Value) {
        self.id.set(v["id"].as_str().map(|s| s.to_string()));
        self.code.set(v["code"].as_str().unwrap_or("").to_string());
        self.description
            .set(v["description"].as_str().unwrap_or("").to_string());
        self.comment
            .set(v["comment"].as_str().unwrap_or("").to_string());
        self.status
            .set(v["status"].as_str().unwrap_or("draft").to_string());
        self.owner_user_id
            .set(v["owner_user_id"].as_str().unwrap_or("").to_string());
        self.is_public
            .set(v["is_public"].as_bool().unwrap_or(false));
        self.version.set(v["version"].as_i64().unwrap_or(1));

        // DataSpec
        if let Some(ds) = v.get("data_spec") {
            self.data_spec_schema_id
                .set(ds["schema_id"].as_str().unwrap_or("").to_string());
            self.data_spec_sql_artifact_id
                .set(ds["sql_artifact_id"].as_str().unwrap_or("").to_string());
            self.data_spec_query_config_json.set(
                serde_json::to_string_pretty(
                    ds.get("query_config").unwrap_or(&serde_json::json!({})),
                )
                .unwrap_or_else(|_| "{}".to_string()),
            );
        }

        // Params
        if let Some(params) = v.get("params") {
            self.params_json
                .set(serde_json::to_string_pretty(params).unwrap_or_else(|_| "[]".to_string()));
        }

        // ViewSpec
        if let Some(vs) = v.get("view_spec") {
            self.view_spec_style_name
                .set(vs["style_name"].as_str().unwrap_or("classic").to_string());
            let raw_html = vs["custom_html"].as_str().unwrap_or("").to_string();
            let raw_css = vs["custom_css"].as_str().unwrap_or("").to_string();
            self.view_spec_custom_html
                .set(code_format::format_html(&raw_html));
            self.view_spec_custom_css
                .set(code_format::format_css(&raw_css));
            self.view_spec_format_json.set(
                serde_json::to_string_pretty(vs.get("format").unwrap_or(&serde_json::json!({})))
                    .unwrap_or_else(|_| "{}".to_string()),
            );
            self.view_spec_thresholds_json.set(
                serde_json::to_string_pretty(
                    vs.get("thresholds").unwrap_or(&serde_json::json!([])),
                )
                .unwrap_or_else(|_| "[]".to_string()),
            );
        }

        // DrillSpec
        if let Some(drill) = v.get("drill_spec") {
            if drill.is_null() {
                self.drill_spec_json.set(String::new());
            } else {
                self.drill_spec_json
                    .set(serde_json::to_string_pretty(drill).unwrap_or_default());
            }
        } else {
            self.drill_spec_json.set(String::new());
        }

        // Meta
        self.created_at
            .set(v["created_at"].as_str().unwrap_or("").to_string());
        self.updated_at
            .set(v["updated_at"].as_str().unwrap_or("").to_string());
        self.created_by
            .set(v["created_by"].as_str().unwrap_or("").to_string());
        self.updated_by
            .set(v["updated_by"].as_str().unwrap_or("").to_string());
    }
}

impl Default for BiIndicatorDetailsVm {
    fn default() -> Self {
        Self::new()
    }
}
