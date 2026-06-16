//! Frontend implementation of the plugin subsystem.
//!
//! Client JavaScript runs in an isolated iframe and calls exported server
//! functions through the parent window's `postMessage` bridge.

pub mod api;
pub mod editor;
pub mod host;
pub mod list;
pub mod menu;

pub use host::PluginHost;
pub use list::PluginList;
pub use menu::{PluginsMenuCategory, PluginsSidebarGroup};
