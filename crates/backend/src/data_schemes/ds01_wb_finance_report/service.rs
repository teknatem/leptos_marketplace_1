//! Compatibility facade for the universal data-scheme executor.
//!
//! Query execution is schema-driven, so DS01, DS02 and DS03 deliberately share
//! one implementation. Existing handlers keep importing this module unchanged.

pub use crate::data_schemes::ds02_mp_sales_register::service::*;
