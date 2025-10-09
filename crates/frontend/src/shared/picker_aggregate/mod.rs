//! Aggregate Picker System
//!
//! Универсальная система выбора агрегатов с модальными окнами и табличным отображением.
//!
//! ## Использование
//!
//! ```rust
//! use crate::shared::picker_aggregate::{
//!     GenericAggregatePicker, Modal, ModalService,
//!     AggregatePickerResult, TableDisplayable
//! };
//!
//! // 1. Предоставьте ModalService в контексте приложения
//! let modal = ModalService::new();
//! provide_context(modal);
//!
//! // 2. Реализуйте трейты для вашего типа
//! impl AggregatePickerResult for MyItem { ... }
//! impl TableDisplayable for MyItem { ... }
//!
//! // 3. Используйте GenericAggregatePicker
//! view! {
//!     <Modal>
//!         <GenericAggregatePicker
//!             items=items_signal
//!             on_confirm=|item| { /* ... */ }
//!             on_cancel=|_| { /* ... */ }
//!         />
//!     </Modal>
//! }
//! ```

pub mod component;
pub mod modal;
pub mod traits;

pub use component::GenericAggregatePicker;
pub use modal::{Modal, ModalRenderer, ModalService};
pub use traits::{AggregatePickerResult, TableDisplayable};
