//! Tab management module
//!
//! Содержит:
//! - `page` - компонент TabPage для обёртки контента таба
//! - `registry` - маппинг tab.key → View (единственный источник правды)

pub mod page;
pub mod registry;

pub use page::TabPage;
