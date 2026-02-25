//! Tab management module
//!
//! Содержит:
//! - `page` - компонент TabPage для обёртки контента таба
//! - `registry` - маппинг tab.key → View (единственный источник правды)
//! - `tab_labels` - единственный источник правды для заголовков табов

pub mod page;
pub mod registry;
pub mod tab_labels;

pub use page::TabPage;
pub use tab_labels::{detail_tab_label, pick_identifier, tab_label_for_key};
