use serde::{de::DeserializeOwned, Serialize};
use std::hash::Hash;

/// Трейт для типов идентификаторов агрегатов
pub trait AggregateId:
    Clone + Copy + PartialEq + Eq + Hash + Serialize + DeserializeOwned + std::fmt::Debug
{
    /// Преобразовать ID в строку
    fn as_string(&self) -> String;

    /// Создать ID из строки
    fn from_string(s: &str) -> Result<Self, String>;
}

// Реализация для базовых типов

impl AggregateId for i32 {
    fn as_string(&self) -> String {
        ToString::to_string(self)
    }

    fn from_string(s: &str) -> Result<Self, String> {
        s.parse::<i32>().map_err(|e| format!("Invalid i32: {}", e))
    }
}

impl AggregateId for i64 {
    fn as_string(&self) -> String {
        ToString::to_string(self)
    }

    fn from_string(s: &str) -> Result<Self, String> {
        s.parse::<i64>().map_err(|e| format!("Invalid i64: {}", e))
    }
}

impl AggregateId for uuid::Uuid {
    fn as_string(&self) -> String {
        ToString::to_string(self)
    }

    fn from_string(s: &str) -> Result<Self, String> {
        uuid::Uuid::parse_str(s).map_err(|e| format!("Invalid UUID: {}", e))
    }
}
