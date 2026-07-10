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
use contracts::domain::a015_wb_orders::{ENTITY_METADATA as A015_META, FIELDS as A015_FIELDS};
use contracts::domain::a017_llm_agent::{ENTITY_METADATA as A017_META, FIELDS as A017_FIELDS};
use contracts::domain::a018_llm_chat::{ENTITY_METADATA as A018_META, FIELDS as A018_FIELDS};
use contracts::domain::a019_llm_artifact::{ENTITY_METADATA as A019_META, FIELDS as A019_FIELDS};
use contracts::domain::a020_wb_promotion::{ENTITY_METADATA as A020_META, FIELDS as A020_FIELDS};
use contracts::domain::a024_bi_indicator::{ENTITY_METADATA as A024_META, FIELDS as A024_FIELDS};
use contracts::domain::a025_bi_dashboard::{ENTITY_METADATA as A025_META, FIELDS as A025_FIELDS};
use contracts::domain::a026_wb_advert_daily::{
    ENTITY_METADATA as A026_META, FIELDS as A026_FIELDS,
};
use contracts::domain::a027_wb_documents::{ENTITY_METADATA as A027_META, FIELDS as A027_FIELDS};
use contracts::domain::a032_wb_returns_claims::{
    ENTITY_METADATA as A032_META, FIELDS as A032_FIELDS,
};
use contracts::domain::a034_ym_realization::{ENTITY_METADATA as A034_META, FIELDS as A034_FIELDS};
use contracts::domain::a035_ym_settlement_recon::{
    ENTITY_METADATA as A035_META, FIELDS as A035_FIELDS,
};
use contracts::domain::a036_wb_sales_funnel_daily::{
    ENTITY_METADATA as A036_META, FIELDS as A036_FIELDS,
};
use contracts::domain::a037_wb_product_snapshot::{
    ENTITY_METADATA as A037_META, FIELDS as A037_FIELDS,
};
use contracts::general_ledger::{ENTITY_METADATA as GL_META, FIELDS as GL_FIELDS};
use contracts::projections::p909_mp_order_line_turnovers::{
    ENTITY_METADATA as P909_META, FIELDS as P909_FIELDS,
};
use contracts::projections::p910_mp_unlinked_turnovers::{
    ENTITY_METADATA as P910_META, FIELDS as P910_FIELDS,
};
use contracts::projections::p914_mp_finance_turnovers::{
    ENTITY_METADATA as P914_META, FIELDS as P914_FIELDS,
};

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
    /// Реестр сущностей, видимых LLM (`list_entities` / `get_entity_schema` / `get_join_hint`).
    /// Источник полей — сгенерированные `metadata_gen` контрактов. При добавлении metadata
    /// в новый домен впиши его сюда И в `tests::EXPECTED_INDICES` — тест ловит «тихий»
    /// дрейф покрытия (иначе для raw SQL LLM «слепа» к колонкам новой таблицы).
    fn build() -> Self {
        Self {
            entries: vec![
                RegistryEntry {
                    meta: &A001_META,
                    fields: A001_FIELDS,
                    tags: &["ref", "1c"],
                },
                RegistryEntry {
                    meta: &A002_META,
                    fields: A002_FIELDS,
                    tags: &["ref"],
                },
                RegistryEntry {
                    meta: &A004_META,
                    fields: A004_FIELDS,
                    tags: &["ref", "1c"],
                },
                RegistryEntry {
                    meta: &A005_META,
                    fields: A005_FIELDS,
                    tags: &["ref"],
                },
                RegistryEntry {
                    meta: &A006_META,
                    fields: A006_FIELDS,
                    tags: &["ref", "wb", "ozon", "ym"],
                },
                RegistryEntry {
                    meta: &A012_META,
                    fields: A012_FIELDS,
                    tags: &["wb", "sales"],
                },
                RegistryEntry {
                    meta: &A015_META,
                    fields: A015_FIELDS,
                    tags: &["wb", "orders"],
                },
                RegistryEntry {
                    meta: &A020_META,
                    fields: A020_FIELDS,
                    tags: &["wb", "promotion"],
                },
                RegistryEntry {
                    meta: &A026_META,
                    fields: A026_FIELDS,
                    tags: &["wb", "advertising"],
                },
                RegistryEntry {
                    meta: &A036_META,
                    fields: A036_FIELDS,
                    tags: &["wb", "analytics"],
                },
                RegistryEntry {
                    meta: &A037_META,
                    fields: A037_FIELDS,
                    tags: &["wb", "analytics", "stocks"],
                },
                RegistryEntry {
                    meta: &A027_META,
                    fields: A027_FIELDS,
                    tags: &["wb", "accounting"],
                },
                RegistryEntry {
                    meta: &A032_META,
                    fields: A032_FIELDS,
                    tags: &["wb", "returns"],
                },
                RegistryEntry {
                    meta: &A013_META,
                    fields: A013_FIELDS,
                    tags: &["ym", "sales"],
                },
                RegistryEntry {
                    meta: &A034_META,
                    fields: A034_FIELDS,
                    tags: &["ym", "accounting", "sales", "ybuh"],
                },
                RegistryEntry {
                    meta: &A035_META,
                    fields: A035_FIELDS,
                    tags: &["ym", "accounting"],
                },
                RegistryEntry {
                    meta: &A017_META,
                    fields: A017_FIELDS,
                    tags: &["llm"],
                },
                RegistryEntry {
                    meta: &A018_META,
                    fields: A018_FIELDS,
                    tags: &["llm"],
                },
                RegistryEntry {
                    meta: &A019_META,
                    fields: A019_FIELDS,
                    tags: &["llm"],
                },
                RegistryEntry {
                    meta: &A024_META,
                    fields: A024_FIELDS,
                    tags: &["bi", "dashboard"],
                },
                RegistryEntry {
                    meta: &A025_META,
                    fields: A025_FIELDS,
                    tags: &["bi", "dashboard"],
                },
                RegistryEntry {
                    meta: &P909_META,
                    fields: P909_FIELDS,
                    tags: &["wb", "ym", "bi", "projection"],
                },
                RegistryEntry {
                    meta: &P910_META,
                    fields: P910_FIELDS,
                    tags: &["wb", "ym", "bi", "projection"],
                },
                RegistryEntry {
                    meta: &P914_META,
                    fields: P914_FIELDS,
                    tags: &["wb", "ym", "bi", "projection", "fina"],
                },
                RegistryEntry {
                    meta: &GL_META,
                    fields: GL_FIELDS,
                    tags: &["gl", "accounting", "journal"],
                },
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

    // ─── architecture_overview ────────────────────────────────────────────

    /// Компактная карта всех сущностей и их связей в одном вызове.
    /// Заменяет серию `list_entities` — LLM сразу видит граф системы.
    /// `category` — необязательный фильтр по тегу ("wb", "gl", "bi", ...).
    pub fn architecture_overview(&self, category: Option<&str>) -> Value {
        let entities: Vec<Value> = self
            .entries
            .iter()
            .filter(|e| category.map_or(true, |cat| e.tags.contains(&cat)))
            .map(|e| {
                let table = e.meta.table_name.unwrap_or(e.meta.collection_name);
                json!({
                    "index":   e.meta.entity_index,
                    "name":    e.meta.ui.element_name,
                    "table":   table,
                    "tags":    e.tags,
                    "related": e.meta.ai.related,
                })
            })
            .collect();

        // Перечень всех доступных категорий (тегов) для навигации.
        let mut categories: Vec<&str> = self
            .entries
            .iter()
            .flat_map(|e| e.tags.iter().copied())
            .collect();
        categories.sort_unstable();
        categories.dedup();

        let total = entities.len();
        json!({
            "entities":   entities,
            "total":      total,
            "categories": categories,
            "hint": "Карта сущностей и связей (related = индексы связанных таблиц). \
                     Детали полей — get_entity_schema(index); SQL JOIN — get_join_hint(from, to); \
                     учёт — get_chart_of_accounts и list_gl_turnovers."
        })
    }

    // ─── get_entity_schema ────────────────────────────────────────────────

    /// Вернуть детальную схему сущности: таблица, поля, типы, ai_hint, FK.
    /// `entity_index` — индекс сущности, например "a012" или "a004".
    pub fn get_entity_schema(&self, entity_index: &str) -> Value {
        let Some(entry) = self.find_by_index(entity_index) else {
            let available: Vec<(&str, Option<&str>)> = self
                .entries
                .iter()
                .map(|e| (e.meta.entity_index, e.meta.table_name))
                .collect();
            let hint_list: Vec<String> = available
                .iter()
                .map(|(idx, tbl)| {
                    if let Some(t) = tbl {
                        format!("'{}' (table: {})", idx, t)
                    } else {
                        format!("'{}'", idx)
                    }
                })
                .collect();
            tracing::warn!(
                "[get_entity_schema] '{}' not found. Available: {:?}",
                entity_index,
                available
            );
            return json!({
                "error": format!(
                    "Entity '{}' not found. Use the short index, not the table name. Available: {}",
                    entity_index,
                    hint_list.join(", ")
                )
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

        // Список колонок для быстрого копирования в SQL (без фильтрации)
        let columns_for_sql: Vec<&str> = entry
            .fields
            .iter()
            .filter(|f| !Self::is_internal_field(f))
            .map(|f| f.name)
            .collect();

        tracing::info!(
            "[get_entity_schema] index='{}' table='{}' fields={}",
            entity_index,
            table,
            columns_for_sql.len()
        );

        json!({
            "index":           entity_index,
            "table":           table,
            "name":            entry.meta.ui.element_name,
            "description":     entry.meta.ai.description,
            "fields":          fields,
            "columns_for_sql": columns_for_sql,
            "related":         entry.meta.ai.related,
            "sql_hint":        format!(
                "SELECT {} FROM {} WHERE is_deleted = 0 LIMIT 100",
                columns_for_sql[..columns_for_sql.len().min(5)].join(", "),
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

        let from_table = from_entry
            .meta
            .table_name
            .unwrap_or(from_entry.meta.collection_name);
        let to_table = to_entry
            .meta
            .table_name
            .unwrap_or(to_entry.meta.collection_name);

        // ref_aggregate может хранить как краткий индекс ("a005"),
        // так и полное имя таблицы ("a005_marketplace") или collection_name.
        // Сравниваем со всеми формами идентификаторов to_entry.
        let matches_to = |ref_agg: Option<&str>| -> bool {
            let Some(ra) = ref_agg else {
                return false;
            };
            ra == to_entry.meta.entity_index
                || Some(ra) == to_entry.meta.table_name
                || ra == to_entry.meta.collection_name
        };

        let matches_from = |ref_agg: Option<&str>| -> bool {
            let Some(ra) = ref_agg else {
                return false;
            };
            ra == from_entry.meta.entity_index
                || Some(ra) == from_entry.meta.table_name
                || ra == from_entry.meta.collection_name
        };

        // Ищем FK-поля в from_entry, ссылающиеся на to_entry
        let fk_fields: Vec<_> = from_entry
            .fields
            .iter()
            .filter(|f| matches_to(f.ref_aggregate))
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
            .filter(|f| matches_from(f.ref_aggregate))
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

    /// Найти сущность по entity_index ("a006"), table_name ("a006_connection_mp")
    /// или collection_name ("connection_mp"). Нечувствительность к формату.
    fn find_by_index(&self, index: &str) -> Option<&RegistryEntry> {
        self.entries.iter().find(|e| {
            e.meta.entity_index == index
                || e.meta.table_name == Some(index)
                || e.meta.collection_name == index
        })
    }

    /// Служебные поля, не нужные LLM
    fn is_internal_field(f: &FieldMetadata) -> bool {
        // Скрываем поля из EntityMetadata (version, is_posted, events и т.п.)
        // кроме созданных/обновлённых дат
        if f.source == FieldSource::Metadata {
            return matches!(f.name, "version" | "is_posted" | "events");
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Сущности/проекции, которые ДОЛЖНЫ быть видимы LLM через `get_entity_schema`.
    /// Это явный «контракт покрытия»: если домен с `metadata_gen` забыли
    /// зарегистрировать в `build()`, тест краснеет, а не молча оставляет LLM
    /// слепой к колонкам таблицы при raw SQL.
    const EXPECTED_INDICES: &[&str] = &[
        "a001", "a002", "a004", "a005", "a006", "a012", "a013", "a015", "a017", "a018", "a019",
        "a020", "a024", "a025", "a026", "a027", "a032", "a034", "a035", "a036", "a037", "p909",
        "p910", "p914", "gl",
    ];

    #[test]
    fn registry_exposes_every_expected_entity_with_fields() {
        for index in EXPECTED_INDICES {
            let schema = METADATA_REGISTRY.get_entity_schema(index);
            assert!(
                schema.get("error").is_none(),
                "entity '{index}' must be reachable via get_entity_schema, got: {schema}"
            );
            let has_fields = schema
                .get("fields")
                .and_then(Value::as_array)
                .is_some_and(|fields| !fields.is_empty());
            assert!(has_fields, "entity '{index}' must expose non-empty fields");
        }
    }

    #[test]
    fn finance_tables_for_raw_sql_are_available() {
        // Регрессия на главную дыру: официальный слой реализации YM (ybuh, a034),
        // сверка расчётов (a035) и WB-возвраты (a032) должны быть видимы для raw SQL.
        for index in ["a034", "a035", "a032", "a027"] {
            let schema = METADATA_REGISTRY.get_entity_schema(index);
            assert!(schema.get("error").is_none(), "{index} missing: {schema}");
        }
    }

    #[test]
    fn ym_realization_links_to_payment_report() {
        // a034 объявляет related → p907_ym_payment_report; карта связей не должна теряться.
        let schema = METADATA_REGISTRY.get_entity_schema("a034");
        let related = schema.get("related").and_then(Value::as_array);
        assert!(
            related.is_some_and(|items| items
                .iter()
                .any(|item| item.as_str().is_some_and(|name| name.contains("p907")))),
            "a034 related must include p907_ym_payment_report, got: {schema}"
        );
    }
}
