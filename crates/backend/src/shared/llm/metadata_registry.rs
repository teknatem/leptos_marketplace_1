//! Реестр метаданных всех сущностей для LLM.
//!
//! Агрегирует `'static` константы из `metadata_gen.rs` каждого домена
//! и предоставляет методы для исследования схемы через инструменты LLM.

use contracts::shared::metadata::{EntityMetadataInfo, FieldMetadata, FieldSource};
use once_cell::sync::Lazy;
use serde_json::{json, Value};

// ─── Импорт всех метаданных из контрактов ───────────────────────────────────

// Импортируем re-exported константы из каждого домена (metadata_gen.rs приватный)
use contracts::domain::a001_connection_1c::{ENTITY_METADATA as A001_META, FIELDS as A001_FIELDS};
use contracts::domain::a002_organization::{ENTITY_METADATA as A002_META, FIELDS as A002_FIELDS};
use contracts::domain::a004_nomenclature::{ENTITY_METADATA as A004_META, FIELDS as A004_FIELDS};
use contracts::domain::a005_marketplace::{ENTITY_METADATA as A005_META, FIELDS as A005_FIELDS};
use contracts::domain::a006_connection_mp::{ENTITY_METADATA as A006_META, FIELDS as A006_FIELDS};
use contracts::domain::a012_wb_sales::{ENTITY_METADATA as A012_META, FIELDS as A012_FIELDS};
use contracts::domain::a013_ym_order::{ENTITY_METADATA as A013_META, FIELDS as A013_FIELDS};
use contracts::domain::a017_llm_agent::{ENTITY_METADATA as A017_META, FIELDS as A017_FIELDS};
use contracts::domain::a018_llm_chat::{ENTITY_METADATA as A018_META, FIELDS as A018_FIELDS};
use contracts::domain::a019_llm_artifact::{ENTITY_METADATA as A019_META, FIELDS as A019_FIELDS};
use contracts::domain::a020_wb_promotion::{ENTITY_METADATA as A020_META, FIELDS as A020_FIELDS};

// ─── Структуры ──────────────────────────────────────────────────────────────

struct RegistryEntry {
    meta: &'static EntityMetadataInfo,
    fields: &'static [FieldMetadata],
    /// Тематические теги для фильтрации (например: "wb", "ozon", "ym", "ref", "llm")
    tags: &'static [&'static str],
}

pub struct MetadataRegistry {
    entries: Vec<RegistryEntry>,
}

// ─── Глобальный экземпляр ───────────────────────────────────────────────────

pub static METADATA_REGISTRY: Lazy<MetadataRegistry> = Lazy::new(MetadataRegistry::build);

// ─── Реализация ─────────────────────────────────────────────────────────────

impl MetadataRegistry {
    fn build() -> Self {
        Self {
            entries: vec![
                RegistryEntry { meta: &A001_META, fields: A001_FIELDS, tags: &["ref", "1c"] },
                RegistryEntry { meta: &A002_META, fields: A002_FIELDS, tags: &["ref"] },
                RegistryEntry { meta: &A004_META, fields: A004_FIELDS, tags: &["ref", "1c"] },
                RegistryEntry { meta: &A005_META, fields: A005_FIELDS, tags: &["ref"] },
                RegistryEntry { meta: &A006_META, fields: A006_FIELDS, tags: &["ref", "wb", "ozon", "ym"] },
                RegistryEntry { meta: &A012_META, fields: A012_FIELDS, tags: &["wb", "sales"] },
                RegistryEntry { meta: &A020_META, fields: A020_FIELDS, tags: &["wb", "promotion"] },
                RegistryEntry { meta: &A013_META, fields: A013_FIELDS, tags: &["ym", "sales"] },
                RegistryEntry { meta: &A017_META, fields: A017_FIELDS, tags: &["llm"] },
                RegistryEntry { meta: &A018_META, fields: A018_FIELDS, tags: &["llm"] },
                RegistryEntry { meta: &A019_META, fields: A019_FIELDS, tags: &["llm"] },
            ],
        }
    }

    // ─── list_entities ────────────────────────────────────────────────────

    /// Вернуть список всех сущностей с кратким описанием.
    /// `category` — необязательный фильтр: "wb" | "ozon" | "ym" | "ref" | "llm"
    pub fn list_entities(&self, category: Option<&str>) -> Value {
        let items: Vec<Value> = self
            .entries
            .iter()
            .filter(|e| category.map_or(true, |cat| e.tags.contains(&cat)))
            .map(|e| {
                let table = e.meta.table_name.unwrap_or(e.meta.collection_name);
                json!({
                    "index":       e.meta.entity_index,
                    "table":       table,
                    "name":        e.meta.ui.element_name,
                    "description": e.meta.ai.description,
                    "questions":   e.meta.ai.questions,
                    "tags":        e.tags,
                })
            })
            .collect();

        let total = items.len();
        json!({
            "entities": items,
            "total": total,
            "hint": "Используй get_entity_schema(entity_index) для получения полей конкретной таблицы."
        })
    }

    // ─── get_entity_schema ────────────────────────────────────────────────

    /// Вернуть детальную схему сущности: таблица, поля, типы, ai_hint, FK.
    /// `entity_index` — индекс сущности, например "a012" или "a004".
    pub fn get_entity_schema(&self, entity_index: &str) -> Value {
        let Some(entry) = self.find_by_index(entity_index) else {
            return json!({
                "error": format!("Entity '{}' not found. Use list_entities() to see available entities.", entity_index)
            });
        };

        let table = entry.meta.table_name.unwrap_or(entry.meta.collection_name);

        let fields: Vec<Value> = entry
            .fields
            .iter()
            .filter(|f| !Self::is_internal_field(f))
            .map(|f| {
                let mut field = json!({
                    "column":   f.name,
                    "type":     Self::rust_to_sql_type(f.rust_type),
                    "label":    f.ui.label,
                    "required": f.validation.required,
                });

                if let Some(hint) = f.ai_hint {
                    field["hint"] = hint.into();
                } else if let Some(ui_hint) = f.ui.hint {
                    field["hint"] = ui_hint.into();
                }

                if let Some(values) = f.enum_values {
                    field["enum_values"] = json!(values);
                }

                if let Some(ref_entity) = f.ref_aggregate {
                    field["fk_to"] = ref_entity.into();
                }

                field
            })
            .collect();

        json!({
            "index":       entity_index,
            "table":       table,
            "name":        entry.meta.ui.element_name,
            "description": entry.meta.ai.description,
            "fields":      fields,
            "related":     entry.meta.ai.related,
            "sql_hint":    format!(
                "SELECT ... FROM {} WHERE is_deleted = 0 LIMIT 100",
                table
            ),
        })
    }

    // ─── get_join_hint ────────────────────────────────────────────────────

    /// Подсказка как соединить две таблицы через JOIN.
    /// Ищет FK в полях `from_entity`, ссылающиеся на `to_entity`.
    pub fn get_join_hint(&self, from_index: &str, to_index: &str) -> Value {
        let Some(from_entry) = self.find_by_index(from_index) else {
            return json!({ "error": format!("Entity '{}' not found", from_index) });
        };
        let Some(to_entry) = self.find_by_index(to_index) else {
            return json!({ "error": format!("Entity '{}' not found", to_index) });
        };

        let from_table = from_entry.meta.table_name.unwrap_or(from_entry.meta.collection_name);
        let to_table = to_entry.meta.table_name.unwrap_or(to_entry.meta.collection_name);

        // Ищем FK-поля в from_entry, ссылающиеся на to_index
        let fk_fields: Vec<_> = from_entry
            .fields
            .iter()
            .filter(|f| f.ref_aggregate == Some(to_index))
            .collect();

        if let Some(fk) = fk_fields.first() {
            let hint_text = fk.ai_hint.or(fk.ui.hint).unwrap_or("");
            return json!({
                "from_table": from_table,
                "to_table":   to_table,
                "join_sql":   format!(
                    "JOIN {} ON {}.{} = {}.id",
                    to_table, from_table, fk.name, to_table
                ),
                "fk_column":  fk.name,
                "fk_label":   fk.ui.label,
                "hint":       hint_text,
            });
        }

        // Ищем в обратном направлении
        let reverse_fk: Vec<_> = to_entry
            .fields
            .iter()
            .filter(|f| f.ref_aggregate == Some(from_index))
            .collect();

        if let Some(fk) = reverse_fk.first() {
            let hint_text = fk.ai_hint.or(fk.ui.hint).unwrap_or("");
            return json!({
                "from_table": from_table,
                "to_table":   to_table,
                "join_sql":   format!(
                    "JOIN {} ON {}.{} = {}.id",
                    from_table, to_table, fk.name, from_table
                ),
                "fk_column":  format!("{}.{}", to_table, fk.name),
                "hint":       hint_text,
                "note":       "Reversed: FK is in the to_table side",
            });
        }

        // FK не найден — предложить через промежуточную таблицу
        json!({
            "from_table": from_table,
            "to_table":   to_table,
            "error":      "No direct FK found between these entities.",
            "suggestion": format!(
                "Check if there is an intermediate table. Use list_entities() and get_entity_schema() to explore.",
            ),
            "related_from": from_entry.meta.ai.related,
            "related_to":   to_entry.meta.ai.related,
        })
    }

    // ─── Вспомогательные ──────────────────────────────────────────────────

    fn find_by_index(&self, index: &str) -> Option<&RegistryEntry> {
        self.entries
            .iter()
            .find(|e| e.meta.entity_index == index)
    }

    /// Служебные поля, не нужные LLM
    fn is_internal_field(f: &FieldMetadata) -> bool {
        // Скрываем поля из EntityMetadata (version, is_posted, events и т.п.)
        // кроме созданных/обновлённых дат
        if f.source == FieldSource::Metadata {
            return matches!(
                f.name,
                "version" | "is_posted" | "events"
            );
        }
        // Скрываем пароли
        matches!(f.ui.widget, Some("password"))
    }

    /// Преобразовать Rust-тип в SQL-тип для контекста LLM
    fn rust_to_sql_type(rust_type: &str) -> &'static str {
        match rust_type {
            t if t.contains("bool") => "INTEGER (0/1)",
            t if t.contains("i32") || t.contains("i64") || t.contains("u32") => "INTEGER",
            t if t.contains("f32") || t.contains("f64") => "REAL",
            t if t.contains("DateTime") => "TEXT (ISO 8601)",
            t if t.ends_with("Id") || t.contains("Option<String>") => "TEXT (UUID or NULL)",
            _ => "TEXT",
        }
    }
}
