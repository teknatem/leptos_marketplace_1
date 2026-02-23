use contracts::shared::indicators::*;

/// Well-known indicator IDs (constants to avoid typos).
pub mod ids {
    use super::*;

    pub fn sales_revenue() -> IndicatorId {
        IndicatorId::new("sales_revenue")
    }
    pub fn sales_order_count() -> IndicatorId {
        IndicatorId::new("sales_order_count")
    }
    pub fn sales_avg_check() -> IndicatorId {
        IndicatorId::new("sales_avg_check")
    }
    pub fn sales_returns_sum() -> IndicatorId {
        IndicatorId::new("sales_returns_sum")
    }
}

/// Build the full catalogue of indicators + sets.
pub fn build_catalog() -> IndicatorCatalogResponse {
    let indicators = vec![
        IndicatorMeta {
            id: ids::sales_revenue(),
            label: "Выручка".into(),
            short_label: None,
            icon: "dollar-sign".into(),
            format: ValueFormat::Money {
                currency: "₽".into(),
            },
            description: Some("Сумма продаж за период (customer_in из p904)".into()),
        },
        IndicatorMeta {
            id: ids::sales_order_count(),
            label: "Заказов".into(),
            short_label: None,
            icon: "orders".into(),
            format: ValueFormat::Integer,
            description: Some("Количество уникальных документов продаж".into()),
        },
        IndicatorMeta {
            id: ids::sales_avg_check(),
            label: "Средний чек".into(),
            short_label: Some("Ср. чек".into()),
            icon: "invoices".into(),
            format: ValueFormat::Money {
                currency: "₽".into(),
            },
            description: Some("Выручка / Количество заказов".into()),
        },
        IndicatorMeta {
            id: ids::sales_returns_sum(),
            label: "Возвраты".into(),
            short_label: None,
            icon: "package-x".into(),
            format: ValueFormat::Money {
                currency: "₽".into(),
            },
            description: Some("Сумма возвратов за период (customer_out из p904)".into()),
        },
    ];

    let sets = vec![IndicatorSetMeta {
        id: IndicatorSetId::new("sales_overview"),
        label: "Обзор продаж".into(),
        indicators: vec![
            ids::sales_revenue(),
            ids::sales_order_count(),
            ids::sales_avg_check(),
            ids::sales_returns_sum(),
        ],
        columns: 4,
    }];

    IndicatorCatalogResponse { indicators, sets }
}
