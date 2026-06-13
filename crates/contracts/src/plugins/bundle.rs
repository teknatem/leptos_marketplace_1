//! Структуры самодостаточного бандла плагина и хранимого определения.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Runtime — место исполнения кода плагина
// ============================================================================

/// Где исполняется код плагина (универсальная замена «Report/Processor»).
///
/// - `Client` — Rhai-логика и рендер целиком в браузере (WASM).
/// - `Server` — Rhai-логика на бэкенде (в т.ч. мутации) через `/api/plugin/:id/run`.
/// - `Hybrid` — есть и `client_script`, и `server_script`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginRuntime {
    Client,
    Server,
    Hybrid,
}

impl PluginRuntime {
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginRuntime::Client => "client",
            PluginRuntime::Server => "server",
            PluginRuntime::Hybrid => "hybrid",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "server" => PluginRuntime::Server,
            "hybrid" => PluginRuntime::Hybrid,
            _ => PluginRuntime::Client,
        }
    }

    /// Нужно ли исполнять код на сервере.
    pub fn runs_on_server(&self) -> bool {
        matches!(self, PluginRuntime::Server | PluginRuntime::Hybrid)
    }

    /// Нужно ли исполнять код в браузере.
    pub fn runs_on_client(&self) -> bool {
        matches!(self, PluginRuntime::Client | PluginRuntime::Hybrid)
    }
}

// ============================================================================
// Status
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginStatus {
    Draft,
    Active,
    Disabled,
}

impl PluginStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginStatus::Draft => "draft",
            PluginStatus::Active => "active",
            PluginStatus::Disabled => "disabled",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "active" => PluginStatus::Active,
            "disabled" => PluginStatus::Disabled,
            _ => PluginStatus::Draft,
        }
    }
}

// ============================================================================
// Params — типизированная форма параметров (привязывается к FilterBar)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamType {
    Date,
    DateRange,
    String,
    Integer,
    Float,
    Boolean,
    /// Ссылка/мультивыбор (напр. кабинеты МП).
    Ref,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamSpec {
    /// Уникальный ключ параметра.
    pub key: String,
    pub param_type: ParamType,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    #[serde(default)]
    pub required: bool,
    /// Привязка к глобальному фильтру дашборда (date_from / date_to / connection_ids).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_filter_key: Option<String>,
}

// ============================================================================
// DataBinding — декларативная привязка к DataView
// ============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataBinding {
    /// ID DataView (напр. "dv001_revenue") для compute/drilldown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view_id: Option<String>,
    /// Метрика внутри DataView.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metric_id: Option<String>,
    /// Разрез по умолчанию для табличного вывода.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
}

// ============================================================================
// ViewSpec / Widget — описание вывода (совместимо по смыслу с a024 view_spec)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WidgetKind {
    Table,
    Cards,
    Chart,
    Html,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Widget {
    pub kind: WidgetKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Произвольная конфигурация блока (колонки таблицы, оси графика, html-шаблон …).
    #[serde(default)]
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ViewSpec {
    #[serde(default)]
    pub widgets: Vec<Widget>,
    /// Опциональный пользовательский HTML-шаблон (санитизируется на сервере).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_html: Option<String>,
}

// ============================================================================
// Manifest — шапка плагина
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Свободный человекочитаемый код-идентификатор (как имя внешнего отчёта).
    pub code: String,
    pub title: String,
    pub runtime: PluginRuntime,
    /// Версия API движка плагинов, на которую рассчитан бандл.
    #[serde(default = "default_api_version")]
    pub api_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Декларация capabilities (defense-in-depth; полный доступ — admin-only).
    #[serde(default)]
    pub capabilities: Vec<String>,
}

fn default_api_version() -> String {
    "1".to_string()
}

// ============================================================================
// Bundle — самодостаточный артефакт
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginBundle {
    pub manifest: PluginManifest,
    #[serde(default)]
    pub params: Vec<ParamSpec>,
    #[serde(default)]
    pub data: DataBinding,
    /// Rhai-скрипт, исполняемый в браузере (для `Client`/`Hybrid`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_script: Option<String>,
    /// Rhai-скрипт, исполняемый на сервере (для `Server`/`Hybrid`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_script: Option<String>,
    #[serde(default)]
    pub view_spec: ViewSpec,
    /// CSS плагина (scoped под `.plugin-<code>` при инжекте).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub styles: Option<String>,
    /// Вложения (имя → содержимое / data-URL).
    #[serde(default)]
    pub assets: HashMap<String, String>,
}

impl PluginBundle {
    /// Базовая валидация бандла (без сохранения).
    pub fn validate(&self) -> Result<(), String> {
        if self.manifest.code.trim().is_empty() {
            return Err("Код плагина не может быть пустым".into());
        }
        if self.manifest.title.trim().is_empty() {
            return Err("Название плагина не может быть пустым".into());
        }
        if self.manifest.runtime.runs_on_server() && self.server_script.is_none() {
            return Err("Runtime server/hybrid требует server_script".into());
        }
        Ok(())
    }
}

// ============================================================================
// Definition — хранимое определение (бандл + локальное состояние)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDefinition {
    pub id: String,
    pub bundle: PluginBundle,
    pub status: PluginStatus,
    pub is_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_user_id: Option<String>,
    /// Если плагин создан LLM-агентом — его id (a017).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by_agent_id: Option<String>,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// DTO для списка / меню
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginListItem {
    pub id: String,
    pub code: String,
    pub title: String,
    pub runtime: String,
    pub status: String,
    pub is_enabled: bool,
    pub updated_at: DateTime<Utc>,
}

impl From<&PluginDefinition> for PluginListItem {
    fn from(def: &PluginDefinition) -> Self {
        Self {
            id: def.id.clone(),
            code: def.bundle.manifest.code.clone(),
            title: def.bundle.manifest.title.clone(),
            runtime: def.bundle.manifest.runtime.as_str().to_string(),
            status: def.status.as_str().to_string(),
            is_enabled: def.is_enabled,
            updated_at: def.updated_at,
        }
    }
}

// ============================================================================
// DTO для создания/обновления через API (и через LLM-инструменты)
// ============================================================================

// ============================================================================
// Контекст запуска плагина (параметры формы + период + кабинеты)
// ============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginRunContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_to: Option<String>,
    #[serde(default)]
    pub connection_mp_refs: Vec<String>,
    /// Разрез табличного вывода (перекрывает DataBinding.group_by).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    /// Прочие параметры формы (key → value).
    #[serde(default)]
    pub params: HashMap<String, String>,
    /// Имя серверной функции для вызова (call_server("name") с клиента).
    /// Если задано — движок исполняет server_script (определяет функции) и вызывает её.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    /// Инлайн-исходник server_script вместо сохранённого в БД — для запуска
    /// отредактированного кода без сохранения (быстрая итерация).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_override: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginUpsert {
    /// None — создание; Some — обновление.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub bundle: PluginBundle,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_user_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by_agent_id: Option<String>,
    /// Ожидаемая версия для оптимистичной блокировки (опц.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<i32>,
}
