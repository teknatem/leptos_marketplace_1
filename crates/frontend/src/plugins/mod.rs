//! Подсистема **Plugins** (frontend) — надстройка над платформой.
//!
//! Отдельная верхнеуровневая ветка. `PluginHost` (движок) монтируется один раз и
//! по ключу вкладки `plugin__<id>` рендерит страницу плагина; список плагинов в
//! меню строится динамически из `GET /api/plugin`.
//!
//! Фаза 1 — декларативный табличный отчёт на DataView (без Rhai). Клиентский
//! Rhai-движок и редактор кода добавляются в следующих фазах.

pub mod api;
pub mod engine;
pub mod host;
pub mod list;
pub mod menu;

pub use host::PluginHost;
pub use list::PluginList;
pub use menu::{PluginsMenuCategory, PluginsSidebarGroup};
