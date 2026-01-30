//! Custom pivot schemas
//!
//! This module contains manually defined schemas for complex reports
//! that require JOINs or custom field definitions.

pub mod s001_wb_finance;

pub use s001_wb_finance::S001_WB_FINANCE_SCHEMA;
