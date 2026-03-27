#![allow(dead_code)]

use crate::shared::metadata::{
    EntityAiMetadata, EntityMetadataInfo, EntityType, EntityUiMetadata, FieldMetadata, FieldSource,
    FieldType, FieldUiMetadata, ValidationRules,
};

pub const ENTITY_METADATA: EntityMetadataInfo = EntityMetadataInfo {
    schema_version: "1.0",
    entity_type: EntityType::Projection,
    entity_name: "WbAdvertByItem",
    entity_index: "p911",
    collection_name: "wb_advert_by_items",
    table_name: Some("p911_wb_advert_by_items"),
    ui: EntityUiMetadata {
        element_name: "WB Advert By Items",
        element_name_en: Some("WB Advert By Items"),
        list_name: "WB Advert By Items",
        list_name_en: Some("WB Advert By Items"),
        icon: Some("database"),
    },
    ai: EntityAiMetadata {
        description: "Номенклатурные обороты рекламных расходов WB. Каждая строка хранит рекламный расход по одной номенклатуре за день и кабинет.",
        questions: &[
            "Какие рекламные расходы WB распределены по номенклатурам?",
            "Какие документы a026 сформировали рекламные расходы по конкретной номенклатуре?",
            "Из каких строк состоит одна сводная проводка по рекламе WB?",
        ],
        related: &[
            "a026_wb_advert_daily",
            "general_ledger",
            "a004_nomenclature",
        ],
    },
    access: None,
};

pub const FIELDS: &[FieldMetadata] = &[
    field(
        "id",
        "String",
        "ID",
        true,
        true,
        Some("Стабильный business key строки проекции."),
        None,
    ),
    field(
        "connection_mp_ref",
        "String",
        "Connection",
        true,
        true,
        Some("UUID кабинета маркетплейса."),
        Some("a006_connection_mp"),
    ),
    field(
        "entry_date",
        "String",
        "Entry Date",
        true,
        true,
        Some("Бизнес-дата расхода на рекламу."),
        None,
    ),
    field(
        "layer",
        "String",
        "Layer",
        true,
        true,
        Some("Слой оборота, для a026 используется oper."),
        None,
    ),
    field(
        "turnover_code",
        "String",
        "Turnover Code",
        true,
        true,
        Some("Код оборота из классификатора turnovers."),
        None,
    ),
    field(
        "amount",
        "f64",
        "Amount",
        true,
        true,
        Some("Сумма рекламного расхода по номенклатуре."),
        None,
    ),
    field(
        "nomenclature_ref",
        "Option<String>",
        "Nomenclature",
        true,
        false,
        Some("Ссылка на a004_nomenclature."),
        Some("a004_nomenclature"),
    ),
    field(
        "registrator_type",
        "String",
        "Registrator Type",
        false,
        true,
        Some("Тип документа-источника, для a026 = a026_wb_advert_daily."),
        None,
    ),
    field(
        "registrator_ref",
        "String",
        "Registrator Ref",
        false,
        true,
        Some("Ссылка на документ-источник a026."),
        None,
    ),
    field(
        "general_ledger_ref",
        "Option<String>",
        "General Ledger Ref",
        false,
        false,
        Some("Связь со сводной записью general_ledger."),
        None,
    ),
];

const fn field(
    name: &'static str,
    rust_type: &'static str,
    label: &'static str,
    visible_in_list: bool,
    required: bool,
    ai_hint: Option<&'static str>,
    ref_aggregate: Option<&'static str>,
) -> FieldMetadata {
    FieldMetadata {
        name,
        rust_type,
        field_type: if ref_aggregate.is_some() {
            FieldType::AggregateRef
        } else {
            FieldType::Primitive
        },
        source: FieldSource::Specific,
        ui: FieldUiMetadata {
            label,
            label_en: None,
            placeholder: None,
            hint: None,
            visible_in_list,
            visible_in_form: false,
            widget: None,
            column_width: None,
        },
        validation: ValidationRules {
            required,
            min: None,
            max: None,
            min_length: None,
            max_length: None,
            pattern: None,
            custom_error: None,
        },
        ai_hint,
        nested_fields: None,
        ref_aggregate,
        enum_values: None,
    }
}
