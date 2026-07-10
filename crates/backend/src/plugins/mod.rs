//! Backend implementation of the plugin subsystem.
//!
//! Exported `server_script` functions run in QuickJS and receive a small host
//! API for database queries and invocation logging.

pub mod change_token;
pub mod data;
pub mod demo;
pub mod engine;
pub mod funnel;
pub mod package;
pub mod publish;
pub mod repository;
pub mod runs;
pub mod service;
