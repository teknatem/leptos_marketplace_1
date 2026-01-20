//! LLM Artifact Domain Module
//!
//! Артефакты, создаваемые LLM агентами в процессе работы с чатами.
//! Включает SQL запросы, параметры, конфигурации визуализации.

pub mod aggregate;
pub mod metadata_gen;

pub use aggregate::{
    ArtifactStatus, ArtifactType, LlmArtifact, LlmArtifactId, LlmArtifactListItem,
};
