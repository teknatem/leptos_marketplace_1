//! Единый реестр измерений для GL-drilldown и UI-каталога измерений.
//!
//! Источник истины по составу измерений оборота хранится в metadata оборотов
//! (`TurnoverClassDef.available_dimensions`). Этот модуль:
//! - даёт UI-ready `GlDimensionDef` по ids измерений
//! - собирает сигнатуру измерений оборота
//! - строит плоский каталог измерений с usage по оборотам

use std::collections::HashMap;

use contracts::general_ledger::{GlDimensionCatalogItem, GlDimensionDef, GlDimensionUsageRef};

use super::turnover_registry::{get_turnover_class, TURNOVER_CLASSES};

const COMMON_DIMENSION_IDS: &[&str] = &[
    "entry_date",
    "connection_mp_ref",
    "registrator_type",
    "layer",
    "entity",
    "registrator_ref",
    // Структурные разрезы проводки — хранятся в самой GL, доступны на всех слоях
    // (как registrator_ref). См. [`STRUCTURAL_DIMENSION_IDS`].
    "turnover_code",
    "debit_account",
    "credit_account",
];

/// Структурные («системные») измерения GL: описывают саму проводку — её оборот и
/// счета. Это полноценные common-измерения (см. [`COMMON_DIMENSION_IDS`]); флаг
/// «системности» нужен лишь для классификации в UI (бейдж, отдельная секция в
/// пикерах) и на доступность/SQL не влияет.
const STRUCTURAL_DIMENSION_IDS: &[&str] = &["turnover_code", "debit_account", "credit_account"];

const NOMENCLATURE_DIMENSION_IDS: &[&str] = &[
    "nomenclature",
    "dim1_category",
    "dim2_line",
    "dim3_model",
    "dim4_format",
    "dim5_sink",
    "dim6_size",
];

/// Измерения слоя `fina`, физически хранящиеся в зеркальной проекции p914
/// (`p914_mp_finance_turnovers`). Доступны через JOIN GL → p914 по
/// `general_ledger_ref`, поэтому работают для любого источника fina (p903/p907).
const FINA_DIMENSION_IDS: &[&str] = &["customer_kind", "fulfillment_type"];

// ─────────────────────────────────────────────────────────────────────────────
// Реестр доступности измерений по слоям: (оборот, слой) → измерения
// ─────────────────────────────────────────────────────────────────────────────
//
// Привязка «(оборот + слой) → проекция» материализуется так: каждую GL-проводку
// слоя зеркалит проекция, которая несёт дополнительные разрезы. Common-измерения
// хранятся в самой `sys_general_ledger` и доступны на любом слое. Остальные
// категории измерений доступны лишь там, где их несёт проекция этого слоя:
//   - nomenclature (a004): oper (p909/p910/p911/p913), fact/prod (p909),
//     fina (p903/p914) — то есть на всех «материальных» слоях;
//   - fina (p914: customer_kind/fulfillment_type) — только на слое `fina`.
//
// Итоговый набор измерений drilldown = `available_dimensions` оборота
// (что оборот поддерживает в принципе) ∩ доступность категории на слое.

#[derive(Debug, Clone, Copy)]
struct LayerDimensionProfile {
    layer: &'static str,
    /// Несёт ли проекция слоя номенклатурную аналитику (a004).
    nomenclature: bool,
    /// Несёт ли проекция слоя fina-разрезы p914 (customer_kind/fulfillment_type).
    fina: bool,
    /// Проекции-зеркала слоя, через которые строится номенклатурная аналитика.
    /// Значения совпадают с ключами `detail_links::descriptor_for_resource_table`.
    nomenclature_projections: &'static [&'static str],
    /// Проекция, несущая fina-разрезы p914 (если слой их материализует).
    fina_projection: Option<&'static str>,
}

const LAYER_DIMENSION_PROFILES: &[LayerDimensionProfile] = &[
    LayerDimensionProfile {
        layer: "oper",
        nomenclature: true,
        fina: false,
        nomenclature_projections: &[
            "p909_mp_order_line_turnovers",
            "p910_mp_unlinked_turnovers",
            "p911_wb_advert_by_items",
            "p913_wb_advert_order_attr",
        ],
        fina_projection: None,
    },
    LayerDimensionProfile {
        layer: "prod",
        nomenclature: true,
        fina: false,
        nomenclature_projections: &["p909_mp_order_line_turnovers"],
        fina_projection: None,
    },
    LayerDimensionProfile {
        layer: "fact",
        nomenclature: true,
        fina: false,
        nomenclature_projections: &["p909_mp_order_line_turnovers"],
        fina_projection: None,
    },
    LayerDimensionProfile {
        layer: "fina",
        nomenclature: true,
        fina: true,
        nomenclature_projections: &["p903_wb_finance_report", "p914_mp_finance_turnovers"],
        fina_projection: Some("p914_mp_finance_turnovers"),
    },
    LayerDimensionProfile {
        layer: "plan",
        nomenclature: false,
        fina: false,
        nomenclature_projections: &[],
        fina_projection: None,
    },
    // Слой «Яндекс бухгалтерия»: официальная выручка a034_ym_realization,
    // проводки на уровне день×кабинет — только common-измерения (дата,
    // кабинет, регистратор), без номенклатурной/fina-аналитики.
    LayerDimensionProfile {
        layer: "ybuh",
        nomenclature: false,
        fina: false,
        nomenclature_projections: &[],
        fina_projection: None,
    },
];

fn layer_profile(layer: &str) -> Option<LayerDimensionProfile> {
    LAYER_DIMENSION_PROFILES
        .iter()
        .copied()
        .find(|profile| profile.layer == layer)
}

fn is_common_dimension_id(dimension_id: &str) -> bool {
    COMMON_DIMENSION_IDS.contains(&dimension_id)
}

/// Системное (структурное) измерение GL — оборот/счета проводки. Используется
/// только для классификации в UI; на доступность для drilldown не влияет.
pub fn is_structural_dimension(dimension_id: &str) -> bool {
    STRUCTURAL_DIMENSION_IDS.contains(&dimension_id)
}

#[derive(Debug, Clone, Copy)]
struct DimensionSeed {
    id: &'static str,
    label: &'static str,
    code_main: &'static str,
    code_suffix: Option<&'static str>,
    parent_id: Option<&'static str>,
    sort_order: usize,
    /// Квалифицированное имя поля в БД: таблица.колонка.
    db_field: &'static str,
    /// Человекочитаемое описание измерения (абзац для UI-каталога).
    description: &'static str,
}

const DIMENSION_SEEDS: &[DimensionSeed] = &[
    DimensionSeed {
        id: "entry_date",
        label: "По дням",
        code_main: "Day",
        code_suffix: None,
        parent_id: None,
        sort_order: 10,
        db_field: "sys_gl.entry_date",
        description: "Дата проводки в журнале GL. Разбивает обороты по дням — \
            основной временной разрез для динамики и сверки периодов.",
    },
    DimensionSeed {
        id: "connection_mp_ref",
        label: "По кабинету МП",
        code_main: "Cab",
        code_suffix: None,
        parent_id: None,
        sort_order: 20,
        db_field: "sys_gl.connection_mp_ref",
        description: "Кабинет (подключение) маркетплейса, к которому относится \
            проводка. Позволяет разделить обороты по продавцам/площадкам.",
    },
    DimensionSeed {
        id: "registrator_type",
        label: "По типу регистратора",
        code_main: "RegType",
        code_suffix: None,
        parent_id: None,
        sort_order: 30,
        db_field: "sys_gl.registrator_type",
        description: "Тип документа-регистратора, породившего проводку (напр. отчёт \
            WB, платёж YM). Группирует обороты по виду первичного документа.",
    },
    DimensionSeed {
        id: "layer",
        label: "По слою",
        code_main: "Layer",
        code_suffix: None,
        parent_id: None,
        sort_order: 40,
        db_field: "sys_gl.layer",
        description: "Слой учёта проводки (oper / prod / fact / fina / plan). Один и \
            тот же оборот может существовать на разных слоях с разной семантикой.",
    },
    DimensionSeed {
        id: "entity",
        label: "По субъекту",
        code_main: "Entity",
        code_suffix: None,
        parent_id: None,
        sort_order: 45,
        db_field: "sys_gl.entity",
        description: "Субъект учёта (контур): маркетплейс (ym/wb/ozon) или собственная \
            организация (san/sts/upr). Весь финансовый отчёт маркетплейса отражается как \
            операции его субъекта; сальдо счёта расчётов = наши деньги у маркетплейса.",
    },
    DimensionSeed {
        id: "registrator_ref",
        label: "По регистратору",
        code_main: "RegRef",
        code_suffix: None,
        parent_id: None,
        sort_order: 50,
        db_field: "sys_gl.registrator_ref",
        description: "Конкретный документ-регистратор (ссылка на экземпляр). Самый \
            детальный разрез идентичности — до отдельного отчёта/платежа.",
    },
    DimensionSeed {
        id: "nomenclature",
        label: "По номенклатуре",
        code_main: "Nom",
        code_suffix: None,
        parent_id: None,
        sort_order: 60,
        // Номенклатура берётся из проекции-зеркала (поле nomenclature_ref), а
        // a004_nomenclature — лишь справочник для подписи. В самой GL колонки нет.
        db_field: "проекция.nomenclature_ref → a004_nomenclature",
        description: "Товарная позиция (a004). В самой GL не хранится — берётся из \
            проекции-зеркала слоя, поэтому доступна только на материальных слоях.",
    },
    DimensionSeed {
        id: "dim1_category",
        label: "По категории",
        code_main: "Nom",
        code_suffix: Some("01"),
        parent_id: Some("nomenclature"),
        sort_order: 61,
        db_field: "a004_nm.dim1_category",
        description: "Категория товара — товарный разрез номенклатуры (a004). \
            Доступен там же, где номенклатура.",
    },
    DimensionSeed {
        id: "dim2_line",
        label: "По линейке",
        code_main: "Nom",
        code_suffix: Some("02"),
        parent_id: Some("nomenclature"),
        sort_order: 62,
        db_field: "a004_nm.dim2_line",
        description: "Линейка товара — товарный разрез номенклатуры (a004). \
            Доступен там же, где номенклатура.",
    },
    DimensionSeed {
        id: "dim3_model",
        label: "По модели",
        code_main: "Nom",
        code_suffix: Some("03"),
        parent_id: Some("nomenclature"),
        sort_order: 63,
        db_field: "a004_nm.dim3_model",
        description: "Модель товара — товарный разрез номенклатуры (a004). \
            Доступен там же, где номенклатура.",
    },
    DimensionSeed {
        id: "dim4_format",
        label: "По формату",
        code_main: "Nom",
        code_suffix: Some("04"),
        parent_id: Some("nomenclature"),
        sort_order: 64,
        db_field: "a004_nm.dim4_format",
        description: "Формат товара — товарный разрез номенклатуры (a004). \
            Доступен там же, где номенклатура.",
    },
    DimensionSeed {
        id: "dim5_sink",
        label: "По назначению",
        code_main: "Nom",
        code_suffix: Some("05"),
        parent_id: Some("nomenclature"),
        sort_order: 65,
        db_field: "a004_nm.dim5_sink",
        description: "Назначение товара — товарный разрез номенклатуры (a004). \
            Доступен там же, где номенклатура.",
    },
    DimensionSeed {
        id: "dim6_size",
        label: "По размеру",
        code_main: "Nom",
        code_suffix: Some("06"),
        parent_id: Some("nomenclature"),
        sort_order: 66,
        db_field: "a004_nm.dim6_size",
        description: "Размер товара — товарный разрез номенклатуры (a004). \
            Доступен там же, где номенклатура.",
    },
    DimensionSeed {
        id: "customer_kind",
        label: "По типу покупателя",
        code_main: "uf",
        code_suffix: None,
        parent_id: None,
        sort_order: 70,
        db_field: "p914_mp_finance_turnovers.customer_kind",
        description: "Тип покупателя. Разрез слоя fina, хранится в зеркальной \
            проекции p914 — доступен только на слое fina.",
    },
    DimensionSeed {
        id: "fulfillment_type",
        label: "По модели продаж",
        code_main: "fulf",
        code_suffix: None,
        parent_id: None,
        sort_order: 71,
        db_field: "p914_mp_finance_turnovers.fulfillment_type",
        description: "Модель продаж (фулфилмент: FBO / FBS и т.п.). Разрез слоя fina, \
            хранится в зеркальной проекции p914 — доступен только на слое fina.",
    },
    DimensionSeed {
        id: "turnover_code",
        label: "По обороту",
        code_main: "Turn",
        code_suffix: None,
        parent_id: None,
        sort_order: 80,
        db_field: "sys_gl.turnover_code",
        description: "Вид оборота самой проводки. В рамках drilldown одного оборота \
            даёт одну группу (самоссылка), но полезен в сводных/межоборотных контекстах. \
            Колонка sys_general_ledger.turnover_code.",
    },
    DimensionSeed {
        id: "debit_account",
        label: "По счёту Дт",
        code_main: "Dr",
        code_suffix: None,
        parent_id: None,
        sort_order: 81,
        db_field: "sys_gl.debit_account",
        description: "Счёт дебета проводки. Обычно фиксирован правилом проводки оборота, \
            поэтому самостоятелен лишь на уровне всего журнала / ведомости по счёту. \
            Колонка sys_general_ledger.debit_account.",
    },
    DimensionSeed {
        id: "credit_account",
        label: "По счёту Кт",
        code_main: "Cr",
        code_suffix: None,
        parent_id: None,
        sort_order: 82,
        db_field: "sys_gl.credit_account",
        description: "Счёт кредита проводки. Обычно фиксирован правилом проводки оборота, \
            поэтому самостоятелен лишь на уровне всего журнала / ведомости по счёту. \
            Колонка sys_general_ledger.credit_account.",
    },
];

pub fn common_dimensions() -> Vec<GlDimensionDef> {
    dimensions_by_ids(COMMON_DIMENSION_IDS)
}

pub fn nomenclature_dimensions() -> Vec<GlDimensionDef> {
    dimensions_by_ids(NOMENCLATURE_DIMENSION_IDS)
}

pub fn fina_dimensions() -> Vec<GlDimensionDef> {
    dimensions_by_ids(FINA_DIMENSION_IDS)
}

pub fn dimensions_catalog() -> Vec<GlDimensionCatalogItem> {
    let usage_map = dimension_usage_map();
    let mut items = DIMENSION_SEEDS
        .iter()
        .map(|seed| build_catalog_item(*seed, &usage_map))
        .collect::<Vec<_>>();

    items.sort_by(|left, right| {
        left.sort_order
            .cmp(&right.sort_order)
            .then_with(|| left.code.cmp(&right.code))
    });

    items
}

fn dimensions_by_ids(ids: &[&str]) -> Vec<GlDimensionDef> {
    ids.iter()
        .filter_map(|id| dimension_def(id))
        .collect::<Vec<_>>()
}

fn dimension_def(dimension_id: &str) -> Option<GlDimensionDef> {
    let seed = dimension_seed(dimension_id)?;
    Some(GlDimensionDef {
        id: seed.id.to_string(),
        label: seed.label.to_string(),
        code: seed.code().to_string(),
        code_main: seed.code_main.to_string(),
        code_suffix: seed.code_suffix.map(ToString::to_string),
        parent_id: seed.parent_id.map(ToString::to_string),
        db_field: seed.db_field.to_string(),
    })
}

/// Возвращает список доступных измерений для конкретного вида оборота.
///
/// Источник истины — `TurnoverClassDef.available_dimensions`.
/// Если оборот неизвестен, оставляем безопасный fallback только на common dimensions.
pub fn dimensions_for_turnover(turnover_code: &str) -> Vec<GlDimensionDef> {
    get_turnover_class(turnover_code)
        .map(|tc| dimensions_by_ids(tc.available_dimensions))
        .unwrap_or_else(common_dimensions)
}

/// Доступно ли измерение на конкретном слое (без привязки к обороту).
/// Common-измерения хранятся в `sys_general_ledger` и доступны всегда;
/// проекционные — только если категория материализована на слое.
pub fn dimension_available_at_layer(dimension_id: &str, layer: &str) -> bool {
    if is_common_dimension_id(dimension_id) {
        return true;
    }
    let Some(profile) = layer_profile(layer) else {
        // Неизвестный слой — безопасно отдаём только common-измерения.
        return false;
    };
    if is_fina_dimension(dimension_id) {
        return profile.fina;
    }
    if is_nomenclature_dimension(dimension_id) {
        return profile.nomenclature;
    }
    false
}

/// Список измерений оборота, доступных на конкретном слое.
///
/// Это пересечение `available_dimensions` оборота (что он поддерживает в
/// принципе) и доступности категории измерения на слое (какая проекция
/// зеркалит этот слой). Источник истины по слоям — [`LAYER_DIMENSION_PROFILES`].
pub fn dimensions_for_turnover_at_layer(turnover_code: &str, layer: &str) -> Vec<GlDimensionDef> {
    let ids = get_turnover_class(turnover_code)
        .map(|tc| tc.available_dimensions)
        .unwrap_or(COMMON_DIMENSION_IDS);
    ids.iter()
        .filter(|id| dimension_available_at_layer(id, layer))
        .filter_map(|id| dimension_def(id))
        .collect()
}

/// Можно ли строить drilldown по измерению для оборота с учётом (опционального)
/// слоя запроса.
///
/// Common-измерения хранятся в GL и допустимы на любом (в т.ч. отсутствующем)
/// слое. Проекционные измерения (номенклатура, fina) требуют конкретного слоя —
/// иначе drilldown смешал бы проводки разных слоёв (oper/fact/fina/prod),
/// которые экономически не складываются. Поэтому без слоя они недопустимы.
pub fn dimension_available_for_drilldown(
    turnover_code: &str,
    dimension_id: &str,
    layer: Option<&str>,
) -> bool {
    let supported = match get_turnover_class(turnover_code) {
        Some(tc) => tc.available_dimensions.contains(&dimension_id),
        // Неизвестный оборот: допускаем только common-измерения (как и прежде).
        None => COMMON_DIMENSION_IDS.contains(&dimension_id),
    };
    if !supported {
        return false;
    }
    if is_common_dimension_id(dimension_id) {
        return true;
    }
    match layer.map(str::trim).filter(|value| !value.is_empty()) {
        Some(layer) => dimension_available_at_layer(dimension_id, layer),
        None => false,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Реестр физического наличия измерений в источниках (GL и проекции-зеркала)
// ─────────────────────────────────────────────────────────────────────────────
//
// Отвечает на вопрос «где взять данные в нужном разрезе»: для каждого источника
// (sys_general_ledger и проекции) перечислены измерения, которые там физически
// доступны как колонка. Состав сверен с Model-ами проекций (см. соответствующие
// repository.rs). Важно:
//   - GL (sys_general_ledger) НЕ хранит номенклатуру — она берётся из проекций;
//     a004_nomenclature — лишь справочник для подписи, не источник разреза;
//   - p911/p913 не имеют колонки `layer`;
//   - p903 не имеет `registrator_type`/`registrator_ref`/`layer`.

/// Идентификаторы измерений идентичности (хранятся в самой GL-проводке).
const SRC_GL_IDS: &[&str] = &[
    "entry_date",
    "connection_mp_ref",
    "registrator_type",
    "registrator_ref",
    "layer",
    // Структурные разрезы — физические колонки самой проводки.
    "turnover_code",
    "debit_account",
    "credit_account",
];

/// Проекция с полной идентичностью + номенклатурой (p909/p910).
const SRC_NOM_FULL_IDS: &[&str] = &[
    "entry_date",
    "connection_mp_ref",
    "registrator_type",
    "registrator_ref",
    "layer",
    "nomenclature",
    "dim1_category",
    "dim2_line",
    "dim3_model",
    "dim4_format",
    "dim5_sink",
    "dim6_size",
];

/// Проекция с номенклатурой, но без колонки `layer` (p911/p913).
const SRC_NOM_NO_LAYER_IDS: &[&str] = &[
    "entry_date",
    "connection_mp_ref",
    "registrator_type",
    "registrator_ref",
    "nomenclature",
    "dim1_category",
    "dim2_line",
    "dim3_model",
    "dim4_format",
    "dim5_sink",
    "dim6_size",
];

/// p914: идентичность + номенклатура + fina-разрезы.
const SRC_P914_IDS: &[&str] = &[
    "entry_date",
    "connection_mp_ref",
    "registrator_type",
    "registrator_ref",
    "layer",
    "nomenclature",
    "dim1_category",
    "dim2_line",
    "dim3_model",
    "dim4_format",
    "dim5_sink",
    "dim6_size",
    "customer_kind",
    "fulfillment_type",
];

/// p903 (внешняя): кабинет + дата + номенклатура; без registrator/layer.
const SRC_P903_IDS: &[&str] = &[
    "entry_date",
    "connection_mp_ref",
    "nomenclature",
    "dim1_category",
    "dim2_line",
    "dim3_model",
    "dim4_format",
    "dim5_sink",
    "dim6_size",
];

struct DimensionSourceTable {
    table: &'static str,
    dimension_ids: &'static [&'static str],
}

const DIMENSION_SOURCE_TABLES: &[DimensionSourceTable] = &[
    DimensionSourceTable {
        table: "sys_general_ledger",
        dimension_ids: SRC_GL_IDS,
    },
    DimensionSourceTable {
        table: "p909_mp_order_line_turnovers",
        dimension_ids: SRC_NOM_FULL_IDS,
    },
    DimensionSourceTable {
        table: "p910_mp_unlinked_turnovers",
        dimension_ids: SRC_NOM_FULL_IDS,
    },
    DimensionSourceTable {
        table: "p911_wb_advert_by_items",
        dimension_ids: SRC_NOM_NO_LAYER_IDS,
    },
    DimensionSourceTable {
        table: "p913_wb_advert_order_attr",
        dimension_ids: SRC_NOM_NO_LAYER_IDS,
    },
    DimensionSourceTable {
        table: "p914_mp_finance_turnovers",
        dimension_ids: SRC_P914_IDS,
    },
    DimensionSourceTable {
        table: "p903_wb_finance_report",
        dimension_ids: SRC_P903_IDS,
    },
];

fn table_dimension_ids(table: &str) -> &'static [&'static str] {
    DIMENSION_SOURCE_TABLES
        .iter()
        .find(|source| source.table == table)
        .map(|source| source.dimension_ids)
        .unwrap_or(&[])
}

/// Хранит ли источник (GL/проекция) данный разрез как колонку.
pub fn table_provides_dimension(table: &str, dimension_id: &str) -> bool {
    table_dimension_ids(table).contains(&dimension_id)
}

/// Короткая метка источника для UI: «GL», «p903», «p914», …
pub fn source_short_label(table: &str) -> String {
    if table == "sys_general_ledger" {
        return "GL".to_string();
    }
    table.split('_').next().unwrap_or(table).to_string()
}

/// Источники, где для оборота на слое доступен данный разрез аналитики.
/// Кандидаты — зеркала ячейки ([`projections_for_cell`], включая GL); из них
/// оставляем только те, что физически содержат измерение. Так popover отвечает
/// «где получить данные в этом разрезе» (напр. кабинет+номенклатура → p914).
pub fn dimension_sources(turnover_code: &str, layer: &str, dimension_id: &str) -> Vec<String> {
    projections_for_cell(turnover_code, layer)
        .into_iter()
        .filter(|table| table_provides_dimension(table, dimension_id))
        .map(source_short_label)
        .collect()
}

/// Проекции-зеркала, через которые на слое `layer` строятся измерения оборота
/// `turnover_code`. Объединяет источники только тех категорий, которые реально
/// присутствуют среди доступных измерений ячейки:
///   - common-измерения → `sys_general_ledger` (поле самой проводки);
///   - номенклатурные → проекции слоя (`nomenclature_projections`);
///   - fina-разрезы → проекция p914 (`fina_projection`).
///
/// Источник истины — [`LAYER_DIMENSION_PROFILES`] (декларативно), значения
/// `resource_table` совпадают с реестром `detail_links`.
pub fn projections_for_cell(turnover_code: &str, layer: &str) -> Vec<&'static str> {
    let dimensions = dimensions_for_turnover_at_layer(turnover_code, layer);
    let profile = layer_profile(layer);

    let mut projections: Vec<&'static str> = Vec::new();
    let push = |value: &'static str, out: &mut Vec<&'static str>| {
        if !out.contains(&value) {
            out.push(value);
        }
    };

    let has_common = dimensions
        .iter()
        .any(|dim| is_common_dimension_id(&dim.id));
    if has_common {
        push("sys_general_ledger", &mut projections);
    }

    if let Some(profile) = profile {
        let has_nomenclature = dimensions
            .iter()
            .any(|dim| is_nomenclature_dimension(&dim.id));
        if has_nomenclature {
            for projection in profile.nomenclature_projections {
                push(projection, &mut projections);
            }
        }

        let has_fina = dimensions.iter().any(|dim| is_fina_dimension(&dim.id));
        if has_fina {
            if let Some(projection) = profile.fina_projection {
                push(projection, &mut projections);
            }
        }
    }

    projections
}

pub fn dimension_signature_for_turnover(turnover_code: &str) -> String {
    get_turnover_class(turnover_code)
        .map(|tc| dimension_signature_from_ids(tc.available_dimensions))
        .unwrap_or_else(|| dimension_signature_from_ids(COMMON_DIMENSION_IDS))
}

pub fn dimension_signature_from_ids(ids: &[&str]) -> String {
    ids.iter()
        .filter_map(|id| dimension_seed(id).map(DimensionSeed::code))
        .collect::<Vec<_>>()
        .join(".")
}

/// Метка измерения по id (для заголовка drilldown-таблицы).
pub fn dimension_label(dimension_id: &str) -> Option<&'static str> {
    dimension_seed(dimension_id).map(|seed| seed.label)
}

/// Определяет, является ли измерение номенклатурным (требует JOIN a004).
pub fn is_nomenclature_dimension(dimension_id: &str) -> bool {
    dimension_seed(dimension_id)
        .map(|seed| seed.code_main == "Nom")
        .unwrap_or(false)
}

/// Определяет, является ли измерение разрезом слоя `fina`, который хранится
/// в зеркальной проекции p914 (требует JOIN GL → p914 по `general_ledger_ref`).
pub fn is_fina_dimension(dimension_id: &str) -> bool {
    FINA_DIMENSION_IDS.contains(&dimension_id)
}

fn dimension_seed(dimension_id: &str) -> Option<&'static DimensionSeed> {
    DIMENSION_SEEDS.iter().find(|seed| seed.id == dimension_id)
}

fn build_catalog_item(
    seed: DimensionSeed,
    usage_map: &HashMap<&'static str, Vec<GlDimensionUsageRef>>,
) -> GlDimensionCatalogItem {
    let path = build_seed_path(seed);
    let root = path.first().copied().unwrap_or(seed);
    let used_by_turnovers = usage_map.get(seed.id).cloned().unwrap_or_default();

    GlDimensionCatalogItem {
        id: seed.id.to_string(),
        label: seed.label.to_string(),
        code: seed.code().to_string(),
        code_main: seed.code_main.to_string(),
        code_suffix: seed.code_suffix.map(ToString::to_string),
        parent_id: seed.parent_id.map(ToString::to_string),
        root_id: root.id.to_string(),
        depth: path.len().saturating_sub(1),
        sort_order: seed.sort_order,
        path_ids: path.iter().map(|item| item.id.to_string()).collect(),
        path_codes: path.iter().map(|item| item.code().to_string()).collect(),
        turnover_count: used_by_turnovers.len(),
        used_by_turnovers,
        db_field: seed.db_field.to_string(),
        description: seed.description.to_string(),
        is_system: is_structural_dimension(seed.id),
    }
}

fn build_seed_path(seed: DimensionSeed) -> Vec<DimensionSeed> {
    let mut path = vec![seed];
    let mut current = seed;

    while let Some(parent_id) = current.parent_id {
        let parent = dimension_seed(parent_id)
            .copied()
            .unwrap_or_else(|| panic!("Unknown parent dimension id: {parent_id}"));
        path.push(parent);
        current = parent;
    }

    path.reverse();
    path
}

fn dimension_usage_map() -> HashMap<&'static str, Vec<GlDimensionUsageRef>> {
    let mut usage_map: HashMap<&'static str, Vec<GlDimensionUsageRef>> = HashMap::new();

    for turnover in TURNOVER_CLASSES {
        let usage_ref = GlDimensionUsageRef {
            turnover_code: turnover.code.to_string(),
            turnover_name: turnover.name.to_string(),
            report_group: turnover.report_group.as_str().to_string(),
        };

        for dimension_id in turnover.available_dimensions {
            usage_map
                .entry(*dimension_id)
                .or_default()
                .push(usage_ref.clone());
        }
    }

    for usages in usage_map.values_mut() {
        usages.sort_by(|left, right| left.turnover_code.cmp(&right.turnover_code));
    }

    usage_map
}

impl DimensionSeed {
    fn code(&self) -> &'static str {
        match self.id {
            "entry_date" => "Day",
            "connection_mp_ref" => "Cab",
            "registrator_type" => "RegType",
            "layer" => "Layer",
            "entity" => "Entity",
            "registrator_ref" => "RegRef",
            "nomenclature" => "Nom",
            "dim1_category" => "Nom01",
            "dim2_line" => "Nom02",
            "dim3_model" => "Nom03",
            "dim4_format" => "Nom04",
            "dim5_sink" => "Nom05",
            "dim6_size" => "Nom06",
            "customer_kind" => "uf",
            "fulfillment_type" => "fulf",
            "turnover_code" => "Turn",
            "debit_account" => "Dr",
            "credit_account" => "Cr",
            _ => unreachable!("dimension registry must cover all supported ids"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn explicit_dimensions_distinguish_nm_and_no_nm_turnovers() {
        let base = dimensions_for_turnover("mp_commission_adjustment");
        let base_ids = base.iter().map(|item| item.id.as_str()).collect::<Vec<_>>();
        assert!(!base_ids.contains(&"nomenclature"));
        assert!(base_ids.contains(&"registrator_ref"));

        let with_nm = dimensions_for_turnover("mp_commission_adjustment_nm");
        let with_nm_ids = with_nm
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>();
        assert!(with_nm_ids.contains(&"nomenclature"));
        assert!(with_nm_ids.contains(&"dim1_category"));
    }

    #[test]
    fn fina_dimensions_are_available_for_turnovers_and_classified() {
        // Разрезы fina присутствуют у оборота с зеркалом p914 (как у common,
        // так и у nomenclature-профиля).
        let common = dimensions_for_turnover("mp_commission_adjustment");
        let common_ids = common.iter().map(|d| d.id.as_str()).collect::<Vec<_>>();
        assert!(common_ids.contains(&"customer_kind"));
        assert!(common_ids.contains(&"fulfillment_type"));

        let with_nm = dimensions_for_turnover("customer_revenue");
        let with_nm_ids = with_nm.iter().map(|d| d.id.as_str()).collect::<Vec<_>>();
        assert!(with_nm_ids.contains(&"customer_kind"));
        assert!(with_nm_ids.contains(&"fulfillment_type"));

        assert!(is_fina_dimension("customer_kind"));
        assert!(is_fina_dimension("fulfillment_type"));
        assert!(!is_fina_dimension("nomenclature"));
        assert!(!is_nomenclature_dimension("customer_kind"));

        // Коды-сокращения совпадают с заявленными.
        let codes = fina_dimensions()
            .into_iter()
            .map(|d| (d.id, d.code))
            .collect::<HashMap<_, _>>();
        assert_eq!(codes.get("customer_kind").map(String::as_str), Some("uf"));
        assert_eq!(
            codes.get("fulfillment_type").map(String::as_str),
            Some("fulf")
        );
    }

    #[test]
    fn fina_dimensions_only_available_at_fina_layer() {
        // fina-разрезы есть только на слое fina.
        assert!(dimension_available_at_layer("customer_kind", "fina"));
        assert!(dimension_available_at_layer("fulfillment_type", "fina"));
        assert!(!dimension_available_at_layer("customer_kind", "oper"));
        assert!(!dimension_available_at_layer("fulfillment_type", "fact"));

        // Номенклатура доступна на материальных слоях, но не на plan.
        assert!(dimension_available_at_layer("nomenclature", "oper"));
        assert!(dimension_available_at_layer("nomenclature", "fina"));
        assert!(!dimension_available_at_layer("nomenclature", "plan"));

        // Common доступны всегда, в т.ч. на неизвестном слое.
        assert!(dimension_available_at_layer("entry_date", "plan"));
        assert!(dimension_available_at_layer("registrator_ref", "whatever"));
    }

    #[test]
    fn dimensions_for_turnover_at_layer_intersects_turnover_and_layer() {
        // customer_revenue (профиль с номенклатурой) на oper: есть номенклатура,
        // нет fina-разрезов.
        let oper = dimensions_for_turnover_at_layer("customer_revenue", "oper");
        let oper_ids = oper.iter().map(|d| d.id.as_str()).collect::<Vec<_>>();
        assert!(oper_ids.contains(&"nomenclature"));
        assert!(!oper_ids.contains(&"customer_kind"));
        assert!(!oper_ids.contains(&"fulfillment_type"));

        // На fina добавляются fina-разрезы.
        let fina = dimensions_for_turnover_at_layer("customer_revenue", "fina");
        let fina_ids = fina.iter().map(|d| d.id.as_str()).collect::<Vec<_>>();
        assert!(fina_ids.contains(&"nomenclature"));
        assert!(fina_ids.contains(&"customer_kind"));
        assert!(fina_ids.contains(&"fulfillment_type"));
    }

    #[test]
    fn drilldown_availability_requires_layer_for_projection_dims() {
        // Common-измерение допустимо без слоя.
        assert!(dimension_available_for_drilldown(
            "customer_revenue",
            "entry_date",
            None
        ));
        // Проекционное измерение без слоя — недопустимо (смешало бы слои).
        assert!(!dimension_available_for_drilldown(
            "customer_revenue",
            "nomenclature",
            None
        ));
        assert!(!dimension_available_for_drilldown(
            "customer_revenue",
            "customer_kind",
            None
        ));
        // С корректным слоем — допустимо.
        assert!(dimension_available_for_drilldown(
            "customer_revenue",
            "nomenclature",
            Some("oper")
        ));
        assert!(dimension_available_for_drilldown(
            "customer_revenue",
            "customer_kind",
            Some("fina")
        ));
        // fina-разрез на не-fina слое — недопустимо.
        assert!(!dimension_available_for_drilldown(
            "customer_revenue",
            "customer_kind",
            Some("oper")
        ));
        // Пустой слой трактуется как отсутствие слоя.
        assert!(!dimension_available_for_drilldown(
            "customer_revenue",
            "nomenclature",
            Some("  ")
        ));
    }

    #[test]
    fn projections_for_cell_reflect_layer_and_present_categories() {
        // fina × customer_revenue: GL + номенклатурные проекции (p903, p914) + p914.
        let fina = projections_for_cell("customer_revenue", "fina");
        assert!(fina.contains(&"sys_general_ledger"));
        assert!(fina.contains(&"p903_wb_finance_report"));
        assert!(fina.contains(&"p914_mp_finance_turnovers"));

        // oper × customer_revenue: GL + oper-семейство проекций, без p903/p914.
        let oper = projections_for_cell("customer_revenue", "oper");
        assert!(oper.contains(&"sys_general_ledger"));
        assert!(oper.contains(&"p909_mp_order_line_turnovers"));
        assert!(!oper.contains(&"p903_wb_finance_report"));

        // mp_commission_adjustment (без номенклатуры) на oper: только GL.
        let common_only = projections_for_cell("mp_commission_adjustment", "oper");
        assert_eq!(common_only, vec!["sys_general_ledger"]);

        // plan: только GL (нет материальных проекций).
        let plan = projections_for_cell("customer_revenue", "plan");
        assert_eq!(plan, vec!["sys_general_ledger"]);
    }

    #[test]
    fn declared_layer_projections_exist_in_detail_links() {
        use crate::general_ledger::detail_links::descriptor_for_resource_table;
        for profile in LAYER_DIMENSION_PROFILES {
            for resource_table in profile.nomenclature_projections {
                assert!(
                    descriptor_for_resource_table(resource_table).is_some(),
                    "nomenclature projection '{resource_table}' for layer '{}' missing in detail_links",
                    profile.layer
                );
            }
            if let Some(resource_table) = profile.fina_projection {
                assert!(
                    descriptor_for_resource_table(resource_table).is_some(),
                    "fina projection '{resource_table}' for layer '{}' missing in detail_links",
                    profile.layer
                );
            }
        }
    }

    #[test]
    fn table_source_registry_matches_projection_columns() {
        // GL хранит идентичность, но НЕ номенклатуру и НЕ fina-разрезы.
        assert!(table_provides_dimension("sys_general_ledger", "registrator_ref"));
        assert!(table_provides_dimension("sys_general_ledger", "connection_mp_ref"));
        assert!(!table_provides_dimension("sys_general_ledger", "nomenclature"));
        assert!(!table_provides_dimension("sys_general_ledger", "customer_kind"));

        // p914 несёт всё: идентичность + номенклатуру + fina.
        assert!(table_provides_dimension("p914_mp_finance_turnovers", "registrator_ref"));
        assert!(table_provides_dimension("p914_mp_finance_turnovers", "nomenclature"));
        assert!(table_provides_dimension("p914_mp_finance_turnovers", "customer_kind"));

        // p903 — без registrator/layer, но с кабинетом и номенклатурой.
        assert!(table_provides_dimension("p903_wb_finance_report", "connection_mp_ref"));
        assert!(table_provides_dimension("p903_wb_finance_report", "nomenclature"));
        assert!(!table_provides_dimension("p903_wb_finance_report", "registrator_ref"));
        assert!(!table_provides_dimension("p903_wb_finance_report", "layer"));

        // p911/p913 не имеют колонки layer.
        assert!(!table_provides_dimension("p911_wb_advert_by_items", "layer"));
        assert!(table_provides_dimension("p911_wb_advert_by_items", "registrator_ref"));

        // Все измерения реестра источников существуют как seed.
        for source in DIMENSION_SOURCE_TABLES {
            for id in source.dimension_ids {
                assert!(
                    dimension_seed(id).is_some(),
                    "unknown dimension '{id}' in source '{}'",
                    source.table
                );
            }
        }
    }

    #[test]
    fn dimension_sources_answer_where_to_get_breakdown() {
        // Кабинет на fina: GL + проекции-зеркала (p903, p914).
        let cab = dimension_sources("customer_revenue", "fina", "connection_mp_ref");
        assert!(cab.contains(&"GL".to_string()));
        assert!(cab.contains(&"p914".to_string()));
        assert!(cab.contains(&"p903".to_string()));

        // Номенклатура на fina: только проекции, без GL.
        let nom = dimension_sources("customer_revenue", "fina", "nomenclature");
        assert!(!nom.contains(&"GL".to_string()));
        assert!(nom.contains(&"p903".to_string()));
        assert!(nom.contains(&"p914".to_string()));

        // Регистратор на fina: GL + p914 (p903 не хранит registrator).
        let reg = dimension_sources("customer_revenue", "fina", "registrator_ref");
        assert_eq!(reg, vec!["GL".to_string(), "p914".to_string()]);

        // fina-разрез — только p914.
        let uf = dimension_sources("customer_revenue", "fina", "customer_kind");
        assert_eq!(uf, vec!["p914".to_string()]);

        assert_eq!(source_short_label("sys_general_ledger"), "GL");
        assert_eq!(source_short_label("p914_mp_finance_turnovers"), "p914");
    }

    #[test]
    fn structural_dimensions_are_common_system_and_drillable() {
        for id in ["turnover_code", "debit_account", "credit_account"] {
            assert!(is_structural_dimension(id), "{id} must be structural");
            // Структурные — это common-измерения (хранятся в GL, на всех слоях).
            assert!(is_common_dimension_id(id), "{id} must be common");
            assert!(!is_nomenclature_dimension(id));
            assert!(!is_fina_dimension(id));

            // В каталоге присутствуют с пометкой системности.
            let item = dimensions_catalog()
                .into_iter()
                .find(|item| item.id == id)
                .unwrap_or_else(|| panic!("{id} missing in catalog"));
            assert!(item.is_system, "{id} must be is_system in catalog");

            // Доступны для drilldown у оборота на любом слое (как common).
            for layer in ["oper", "prod", "fact", "fina", "plan"] {
                assert!(
                    dimension_available_for_drilldown("customer_revenue", id, Some(layer)),
                    "{id} must be drillable at layer {layer}"
                );
            }
            // GL физически несёт эти колонки.
            assert!(table_provides_dimension("sys_general_ledger", id));
        }
    }

    #[test]
    fn dimension_codes_are_unique() {
        let items = dimensions_catalog();
        let codes = items
            .iter()
            .map(|item| item.code.as_str())
            .collect::<HashSet<_>>();
        assert_eq!(codes.len(), items.len());
    }

    #[test]
    fn nomenclature_children_have_global_fixed_codes() {
        let items = dimensions_catalog();
        let codes_by_id = items
            .iter()
            .map(|item| (item.id.as_str(), item.code.as_str()))
            .collect::<HashMap<_, _>>();

        assert_eq!(codes_by_id.get("nomenclature"), Some(&"Nom"));
        assert_eq!(codes_by_id.get("dim1_category"), Some(&"Nom01"));
        assert_eq!(codes_by_id.get("dim2_line"), Some(&"Nom02"));
        assert_eq!(codes_by_id.get("dim3_model"), Some(&"Nom03"));
        assert_eq!(codes_by_id.get("dim4_format"), Some(&"Nom04"));
        assert_eq!(codes_by_id.get("dim5_sink"), Some(&"Nom05"));
        assert_eq!(codes_by_id.get("dim6_size"), Some(&"Nom06"));
    }

    #[test]
    fn dimension_catalog_builds_hierarchy_and_paths() {
        let item = dimensions_catalog()
            .into_iter()
            .find(|item| item.id == "dim3_model")
            .expect("dim3_model not found");

        assert_eq!(item.parent_id.as_deref(), Some("nomenclature"));
        assert_eq!(item.root_id, "nomenclature");
        assert_eq!(item.depth, 1);
        assert_eq!(item.path_ids, vec!["nomenclature", "dim3_model"]);
        assert_eq!(item.path_codes, vec!["Nom", "Nom03"]);
    }

    #[test]
    fn dimension_signature_matches_profiles() {
        assert_eq!(
            dimension_signature_from_ids(COMMON_DIMENSION_IDS),
            "Day.Cab.RegType.Layer.Entity.RegRef.Turn.Dr.Cr"
        );
        assert_eq!(
            dimension_signature_from_ids(&[
                "entry_date",
                "connection_mp_ref",
                "registrator_type",
                "layer",
                "registrator_ref",
                "nomenclature",
                "dim1_category",
                "dim2_line",
                "dim3_model",
                "dim4_format",
                "dim5_sink",
                "dim6_size",
            ]),
            "Day.Cab.RegType.Layer.RegRef.Nom.Nom01.Nom02.Nom03.Nom04.Nom05.Nom06"
        );
    }

    #[test]
    fn dimension_catalog_contains_turnover_usage() {
        let item = dimensions_catalog()
            .into_iter()
            .find(|item| item.id == "entry_date")
            .expect("entry_date not found");

        assert!(item.turnover_count > 0);
        assert!(item
            .used_by_turnovers
            .iter()
            .any(|usage| usage.turnover_code == "qty_ordered"));
    }
}
