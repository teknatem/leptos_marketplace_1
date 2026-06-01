//! Реестр слоёв учёта General Ledger (`GlLayerClassDef`, DTO для каталога).
//!
//! Единый источник истины по составу слоёв — `TurnoverLayer`. Здесь к каждому
//! слою добавляются отображаемые метаданные: человекочитаемое имя, описание,
//! ключ цвета (для бейджей) и порядок сортировки. По аналогии с реестрами
//! измерений (`general_ledger_dimensions`) и оборотов (`general_ledger_turnovers`).

use serde::{Deserialize, Serialize};

/// Статическое определение слоя: код + отображаемые метаданные.
#[derive(Debug, Clone, Copy)]
pub struct GlLayerClassDef {
    /// Код слоя (совпадает с `TurnoverLayer::as_str`).
    pub code: &'static str,
    /// Человекочитаемое имя.
    pub name: &'static str,
    /// Описание назначения слоя.
    pub description: &'static str,
    /// Ключ цвета для бейджа (`gl-layer-badge--{color_key}`). Совпадает с `code`.
    pub color_key: &'static str,
    /// Порядок сортировки в каталоге и фильтрах.
    pub sort_order: usize,
}

/// Полный реестр слоёв учёта GL — одна запись на каждый вариант `TurnoverLayer`.
pub const GL_LAYER_CLASSES: &[GlLayerClassDef] = &[
    GlLayerClassDef {
        code: "oper",
        name: "Операционный",
        description: "Оперативный учёт по событиям заказов: проводки строятся \
            синхронно с операционными проекциями (p909/p910).",
        color_key: "oper",
        sort_order: 0,
    },
    GlLayerClassDef {
        code: "fact",
        name: "Фактический",
        description: "Фактический слой по данным маркетплейса. Постепенно \
            замещается слоем `fina` для источников p903/p907.",
        color_key: "fact",
        sort_order: 1,
    },
    GlLayerClassDef {
        code: "fina",
        name: "Финансовый",
        description: "Финансовый слой: обороты, построенные из финансовых отчётов \
            МП (p903/p907) синхронно с GL. Замещает `fact` для этих источников.",
        color_key: "fina",
        sort_order: 2,
    },
    GlLayerClassDef {
        code: "prod",
        name: "Производственный",
        description: "Производственный слой себестоимости и движения товаров.",
        color_key: "prod",
        sort_order: 3,
    },
    GlLayerClassDef {
        code: "plan",
        name: "Плановый",
        description: "Плановые обороты для сопоставления с фактом.",
        color_key: "plan",
        sort_order: 4,
    },
];

/// Поиск определения слоя по коду.
pub fn get_layer_class(code: &str) -> Option<&'static GlLayerClassDef> {
    GL_LAYER_CLASSES.iter().find(|item| item.code == code)
}

/// DTO одного слоя для каталога (`/api/general-ledger/layers`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlLayerDto {
    pub code: String,
    pub name: String,
    pub description: String,
    pub color_key: String,
    pub sort_order: usize,
    pub gl_entries_count: i64,
}

/// Ответ каталога слоёв.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlLayersResponse {
    pub items: Vec<GlLayerDto>,
    pub total: usize,
}
