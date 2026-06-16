//! Backend implementation of the plugin subsystem.
//!
//! Exported `server_script` functions run in QuickJS and receive a small host
//! API for database queries and invocation logging.

pub mod engine;
pub mod package;
pub mod repository;
pub mod runs;
pub mod service;
