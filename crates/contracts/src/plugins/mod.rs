//! Подсистема **Plugins** — надстройка (extension layer) над платформой.
//!
//! Плагин — самодостаточный интерпретируемый артефакт (`PluginBundle`): манифест,
//! параметры, привязка к данным, опциональные Rhai-скрипты (client/server), описание
//! вывода (`ViewSpec`) и стили. Загружается и исполняется в рантайме без пересборки.
//!
//! Это НЕ агрегат `a0XX`, а отдельная верхнеуровневая ветка, следующая базовым
//! конвенциям агрегатов (CRUD, JSON-хранение, soft-delete, MVVM на фронте).
//!
//! Терминология (универсальная, по месту исполнения):
//! - `PluginRuntime` = `Client` | `Server` | `Hybrid` — где исполняется код.
//! - функциональная категория плагина НЕ фиксирована.

pub mod bundle;

pub use bundle::{
    DataBinding, ParamSpec, ParamType, PluginBundle, PluginDefinition, PluginListItem,
    PluginManifest, PluginRunContext, PluginRuntime, PluginStatus, PluginUpsert, ViewSpec, Widget,
    WidgetKind,
};
