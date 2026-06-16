//! Структуры самодостаточного бандла плагина и хранимого определения.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Runtime — место исполнения кода плагина
// ============================================================================

/// Где исполняется код плагина (универсальная замена «Report/Processor»).
///
/// - `Client` — JavaScript-логика и рендер целиком в браузере.
/// - `Server` — JavaScript-логика на бэкенде через `/api/plugin/:id/invoke`.
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

/// ⚠️ RESERVED / не основной путь. Декларативная привязка к DataView (drilldown
/// через `/api/plugin/:id/data`). Канонический путь вывода плагина —
/// `client_script` + `server_script` + `sql_resources` (см. [`PluginBundle`]).
/// Поле сохраняется ради совместимости и редко используется; новый код опирайся
/// на JS-путь.
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
//
// ⚠️ RESERVED / не основной путь. `ViewSpec`/`Widget`/`custom_html` сейчас не
// рендерятся хостом (всюду `ViewSpec::default()`), серверная санитизация HTML не
// реализована. Канонический путь вывода — `client_script` строит UI сам. Не
// заполняй эти поля без явной необходимости.

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
    /// ⚠️ RESERVED. Декларация capabilities (defense-in-depth) — пока не
    /// enforced'ится движком; носит справочный характер.
    #[serde(default)]
    pub capabilities: Vec<String>,
}

fn default_api_version() -> String {
    "1".to_string()
}

// ============================================================================
// Bundle — самодостаточный артефакт
// ============================================================================

/// Самодостаточный переносимый артефакт плагина.
///
/// **Канонический путь** (используй его): `client_script` (UI в iframe) +
/// `server_script` (логика в QuickJS) + `sql_resources` (именованные SELECT) +
/// `styles`. Поля `data` (DataBinding), `view_spec`, `manifest.capabilities` —
/// RESERVED и в основном потоке не задействованы.
///
/// Единица переноса между экземплярами — именно `PluginBundle`; идентичность —
/// `manifest.code`. Локальное состояние живёт в [`PluginDefinition`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginBundle {
    pub manifest: PluginManifest,
    #[serde(default)]
    pub params: Vec<ParamSpec>,
    /// ⚠️ RESERVED — см. [`DataBinding`].
    #[serde(default)]
    pub data: DataBinding,
    /// ES-модуль, исполняемый в изолированном iframe (для `Client`/`Hybrid`).
    /// Должен экспортировать `mount(root, host)`; `unmount()` опционален.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_script: Option<String>,
    /// ES-модуль, исполняемый QuickJS на сервере (для `Server`/`Hybrid`).
    /// Экспортированные функции вызываются через `/api/plugin/:id/invoke`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_script: Option<String>,
    /// ⚠️ RESERVED — см. [`ViewSpec`] (хостом не рендерится).
    #[serde(default)]
    pub view_spec: ViewSpec,
    /// CSS плагина (scoped под `.plugin-<code>` при инжекте).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub styles: Option<String>,
    /// Именованные SQL-запросы серверной части.
    ///
    /// Скрипт получает к ним доступ через `host.db.queryResource(name, params)`.
    #[serde(default)]
    pub sql_resources: HashMap<String, String>,
    /// Вложения (имя → содержимое / data-URL).
    #[serde(default)]
    pub assets: HashMap<String, String>,
}

/// Является ли SQL читающим (только `SELECT`/`WITH`).
///
/// Та же проверка применяется движком в `host.db.query`/`queryResource`; держим её
/// в контракте, чтобы запрещённый SQL отсекался ещё до сохранения бандла.
pub fn is_read_only_sql(sql: &str) -> bool {
    let upper = sql.trim().to_ascii_uppercase();
    upper.starts_with("SELECT") || upper.starts_with("WITH")
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
        if self.manifest.runtime.runs_on_client() && self.client_script.is_none() {
            return Err("Runtime client/hybrid требует client_script".into());
        }
        if self.manifest.runtime.runs_on_server() && self.server_script.is_none() {
            return Err("Runtime server/hybrid требует server_script".into());
        }
        if self
            .sql_resources
            .iter()
            .any(|(name, sql)| name.trim().is_empty() || sql.trim().is_empty())
        {
            return Err("Имя и текст SQL-ресурса не могут быть пустыми".into());
        }
        if let Some((name, _)) = self
            .sql_resources
            .iter()
            .find(|(_, sql)| !is_read_only_sql(sql))
        {
            return Err(format!(
                "SQL-ресурс '{name}' должен начинаться с SELECT или WITH (разрешено только чтение)"
            ));
        }
        Ok(())
    }
}

// ============================================================================
// Ошибки исполнения / отчёт валидации (для движка, фронта и LLM-инструментов)
// ============================================================================

/// Структурированная ошибка исполнения плагина.
///
/// `stage` указывает этап сбоя для самоисправления (в т.ч. LLM-агентом):
/// `module_eval` | `missing_export` | `invoke` | `runtime` | `sql` | `deserialize` | `timeout`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginError {
    pub stage: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
}

impl PluginError {
    pub fn new(stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            message: message.into(),
            stack: None,
        }
    }

    pub fn with_stack(mut self, stack: Option<String>) -> Self {
        self.stack = stack.filter(|s| !s.trim().is_empty());
        self
    }
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.stage, self.message)
    }
}

impl std::error::Error for PluginError {}

/// Результат проверки бандла без сохранения и без вызова функций плагина.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginValidateReport {
    pub ok: bool,
    /// Имена функций, экспортированных серверным ES-модулем.
    #[serde(default)]
    pub server_exports: Vec<String>,
    #[serde(default)]
    pub errors: Vec<PluginError>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server_bundle(sql_name: &str, sql: &str) -> PluginBundle {
        PluginBundle {
            manifest: PluginManifest {
                code: "T".into(),
                title: "T".into(),
                runtime: PluginRuntime::Server,
                api_version: "2".into(),
                description: None,
                capabilities: vec![],
            },
            params: vec![],
            data: DataBinding::default(),
            client_script: None,
            server_script: Some("export function run() {}".into()),
            view_spec: ViewSpec::default(),
            styles: None,
            sql_resources: [(sql_name.to_string(), sql.to_string())]
                .into_iter()
                .collect(),
            assets: HashMap::new(),
        }
    }

    #[test]
    fn accepts_read_only_sql() {
        assert!(server_bundle("ok", "WITH x AS (SELECT 1) SELECT * FROM x")
            .validate()
            .is_ok());
    }

    #[test]
    fn rejects_non_select_resource() {
        let error = server_bundle("danger", "DELETE FROM a004_nomenclature")
            .validate()
            .unwrap_err();
        assert!(error.contains("danger"), "got: {error}");
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
}

/// Вызов экспортированной функции серверного ES-модуля плагина.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInvokeRequest {
    pub method: String,
    #[serde(default)]
    pub args: serde_json::Value,
    #[serde(default)]
    pub context: PluginRunContext,
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
