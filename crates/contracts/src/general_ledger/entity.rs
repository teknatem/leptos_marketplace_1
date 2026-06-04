//! Реестр субъектов учёта General Ledger (`GlEntityClassDef`, DTO для каталога).
//!
//! Измерение «субъект учёта» (`entity`) — контур, к которому относится проводка:
//! маркетплейс (виртуальная фирма-агент) или собственная организация. По образцу
//! реестра слоёв (`layer`) — статический источник истины, в перспективе может
//! переехать в таблицу БД. Идея: весь финансовый отчёт маркетплейса отражается
//! как операции отдельного субъекта, и сальдо счёта расчётов (7609) при таком
//! субъекте = «наши деньги у этого маркетплейса».

use serde::{Deserialize, Serialize};

/// Код субъекта учёта. Совпадает с `code` в `GL_ENTITY_CLASSES` и хранится в
/// колонке `sys_general_ledger.entity` / `a002_organization.entity_ref`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlEntity {
    /// Яндекс Маркет (контур маркетплейса).
    Ym,
    /// Wildberries (контур маркетплейса).
    Wb,
    /// OZON (контур маркетплейса).
    Ozon,
    /// Собственная организация SAN.
    San,
    /// Собственная организация STS.
    Sts,
    /// Собственная организация UPR.
    Upr,
}

impl GlEntity {
    pub fn as_str(&self) -> &'static str {
        match self {
            GlEntity::Ym => "ym",
            GlEntity::Wb => "wb",
            GlEntity::Ozon => "ozon",
            GlEntity::San => "san",
            GlEntity::Sts => "sts",
            GlEntity::Upr => "upr",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ym" => Some(GlEntity::Ym),
            "wb" => Some(GlEntity::Wb),
            "ozon" => Some(GlEntity::Ozon),
            "san" => Some(GlEntity::San),
            "sts" => Some(GlEntity::Sts),
            "upr" => Some(GlEntity::Upr),
            _ => None,
        }
    }
}

/// Вид субъекта: маркетплейс (агентский контур) или собственная организация.
pub const ENTITY_KIND_MARKETPLACE: &str = "marketplace";
pub const ENTITY_KIND_OWN: &str = "own";

/// Статическое определение субъекта: код + отображаемые метаданные.
#[derive(Debug, Clone, Copy)]
pub struct GlEntityClassDef {
    /// Код субъекта (совпадает с `GlEntity::as_str`).
    pub code: &'static str,
    /// Человекочитаемое имя.
    pub name: &'static str,
    /// Описание назначения субъекта.
    pub description: &'static str,
    /// Вид субъекта: `marketplace` | `own`.
    pub kind: &'static str,
    /// Ключ цвета для бейджа (`gl-entity-badge--{color_key}`). Совпадает с `code`.
    pub color_key: &'static str,
    /// Порядок сортировки в каталоге и фильтрах.
    pub sort_order: usize,
}

/// Полный реестр субъектов учёта GL — 3 маркетплейса + 3 собственные организации.
pub const GL_ENTITY_CLASSES: &[GlEntityClassDef] = &[
    GlEntityClassDef {
        code: "ym",
        name: "Яндекс Маркет",
        description: "Контур маркетплейса Яндекс Маркет: операции платёжного \
            отчёта YM (p907) отражаются как операции этого субъекта. Сальдо счёта \
            расчётов = наши деньги у Yandex.",
        kind: ENTITY_KIND_MARKETPLACE,
        color_key: "ym",
        sort_order: 0,
    },
    GlEntityClassDef {
        code: "wb",
        name: "Wildberries",
        description: "Контур маркетплейса Wildberries: операции финансового отчёта \
            WB (p903) как операции этого субъекта.",
        kind: ENTITY_KIND_MARKETPLACE,
        color_key: "wb",
        sort_order: 1,
    },
    GlEntityClassDef {
        code: "ozon",
        name: "OZON",
        description: "Контур маркетплейса OZON.",
        kind: ENTITY_KIND_MARKETPLACE,
        color_key: "ozon",
        sort_order: 2,
    },
    GlEntityClassDef {
        code: "san",
        name: "SAN",
        description: "Собственная организация SAN.",
        kind: ENTITY_KIND_OWN,
        color_key: "san",
        sort_order: 3,
    },
    GlEntityClassDef {
        code: "sts",
        name: "STS",
        description: "Собственная организация STS.",
        kind: ENTITY_KIND_OWN,
        color_key: "sts",
        sort_order: 4,
    },
    GlEntityClassDef {
        code: "upr",
        name: "UPR",
        description: "Собственная организация UPR.",
        kind: ENTITY_KIND_OWN,
        color_key: "upr",
        sort_order: 5,
    },
];

/// Поиск определения субъекта по коду.
pub fn get_entity_class(code: &str) -> Option<&'static GlEntityClassDef> {
    GL_ENTITY_CLASSES.iter().find(|item| item.code == code)
}

/// DTO одного субъекта для каталога (`/api/general-ledger/entities`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlEntityDto {
    pub code: String,
    pub name: String,
    pub description: String,
    pub kind: String,
    pub color_key: String,
    pub sort_order: usize,
    pub gl_entries_count: i64,
}

/// Ответ каталога субъектов.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlEntitiesResponse {
    pub items: Vec<GlEntityDto>,
    pub total: usize,
}
