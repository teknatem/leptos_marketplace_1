//! Aggregate Picker System
//!
//! Универсальная система выбора агрегатов с модальными окнами и табличным отображением.
//!
//! ## Использование
//!
//! ```rust
//! use crate::shared::picker_aggregate::{
//!     GenericAggregatePicker,
//!     AggregatePickerResult, TableDisplayable
//! };
//!
//! // 2. Реализуйте трейты для вашего типа
//! impl AggregatePickerResult for MyItem { ... }
//! impl TableDisplayable for MyItem { ... }
//!
//! // 3. Используйте GenericAggregatePicker
//! view! {
//!         <GenericAggregatePicker
//!             items=items_signal
//!             on_confirm=|item| { /* ... */ }
//!             on_cancel=|_| { /* ... */ }
//!         />
//! }
//! ```

pub mod component;
pub mod traits;

pub use component::GenericAggregatePicker;
pub use traits::{AggregatePickerResult, TableDisplayable};
