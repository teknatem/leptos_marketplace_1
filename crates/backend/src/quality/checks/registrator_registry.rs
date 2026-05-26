//! # Реестр регистраторов для подсистемы контроля качества
//!
//! Хранит статические метаданные о каждом типе регистратора:
//! - читаемое название,
//! - возможность перепроведения,
//! - префикс tab-ключа для открытия карточки документа во фронтенде.
//!
//! Также предоставляет `repost_document` — единую точку вызова
//! процедуры проведения для любого поддерживаемого агрегата.

/// Статические метаданные одного типа регистратора.
pub struct RegistratorMeta {
    /// Читаемое название типа, напр. `"Реклама WB (день)"`.
    pub type_label: &'static str,
    /// `true` — для этого типа доступно перепроведение.
    pub can_post: bool,
    /// Префикс tab-ключа для открытия карточки во фронтенде,
    /// напр. `Some("a026_wb_advert_daily_details")`.
    pub tab_key_prefix: Option<&'static str>,
}

fn document_id_from_registrator_ref(registrator_ref: &str) -> &str {
    registrator_ref
        .split_once(':')
        .map(|(_, id)| id)
        .unwrap_or(registrator_ref)
}

/// Возвращает метаданные для переданного `registrator_type`.
///
/// Для неизвестных типов возвращает `can_post = false` и отсутствие tab-prefix.
pub fn get_meta(registrator_type: &str) -> RegistratorMeta {
    match registrator_type {
        "a012_wb_sales" => RegistratorMeta {
            type_label: "Продажи WB",
            can_post: true,
            tab_key_prefix: Some("a012_wb_sales_details"),
        },
        "a013_ym_order" => RegistratorMeta {
            type_label: "Заказы Яндекс.Маркет",
            can_post: true,
            tab_key_prefix: Some("a013_ym_order_details"),
        },
        "a014_ozon_transactions" => RegistratorMeta {
            type_label: "Транзакции Ozon",
            can_post: true,
            tab_key_prefix: Some("a014_ozon_transactions_details"),
        },
        "a016_ym_returns" => RegistratorMeta {
            type_label: "Возвраты Яндекс.Маркет",
            can_post: true,
            tab_key_prefix: Some("a016_ym_returns_details"),
        },
        "a021_production_output" => RegistratorMeta {
            type_label: "Выпуск продукции",
            can_post: true,
            tab_key_prefix: Some("a021_production_output_details"),
        },
        "a023_purchase_of_goods" => RegistratorMeta {
            type_label: "Закупки товаров",
            can_post: true,
            tab_key_prefix: Some("a023_purchase_of_goods_details"),
        },
        "a026_wb_advert_daily" => RegistratorMeta {
            type_label: "Реклама WB (день)",
            can_post: true,
            tab_key_prefix: Some("a026_wb_advert_daily_details"),
        },
        "a015_wb_orders" => RegistratorMeta {
            type_label: "Заказы Wildberries",
            can_post: true,
            tab_key_prefix: Some("a015_wb_orders_details"),
        },
        _ => RegistratorMeta {
            type_label: "Документ",
            can_post: false,
            tab_key_prefix: None,
        },
    }
}

/// Вызывает процедуру проведения для документа с указанным `registrator_type`.
///
/// Возвращает `Err`, если тип не поддерживает проведение или UUID некорректен.
pub async fn repost_document(registrator_type: &str, registrator_ref: &str) -> anyhow::Result<()> {
    let document_id = document_id_from_registrator_ref(registrator_ref);
    let uuid = uuid::Uuid::parse_str(document_id)
        .map_err(|e| anyhow::anyhow!("Invalid registrator_ref '{}': {}", registrator_ref, e))?;

    match registrator_type {
        "a012_wb_sales" => {
            crate::domain::a012_wb_sales::posting::post_document(uuid).await?;
        }
        "a013_ym_order" => {
            crate::domain::a013_ym_order::posting::post_document(uuid).await?;
        }
        "a014_ozon_transactions" => {
            crate::domain::a014_ozon_transactions::posting::post_document(uuid).await?;
        }
        "a016_ym_returns" => {
            crate::domain::a016_ym_returns::posting::post_document(uuid).await?;
        }
        "a021_production_output" => {
            crate::domain::a021_production_output::service::post_document(uuid).await?;
        }
        "a023_purchase_of_goods" => {
            crate::domain::a023_purchase_of_goods::service::post_document(uuid).await?;
        }
        "a026_wb_advert_daily" => {
            crate::domain::a026_wb_advert_daily::posting::post_document(uuid).await?;
        }
        "a015_wb_orders" => {
            crate::domain::a015_wb_orders::posting::post_document(uuid).await?;
        }
        other => {
            return Err(anyhow::anyhow!(
                "Тип регистратора '{}' не поддерживает перепроведение",
                other
            ));
        }
    }

    Ok(())
}

/// Возвращает колонки исходного документа для отображения в drill-down,
/// смоделированные по списку соответствующего агрегата.
///
/// Для типов без шаблона возвращает пустой вектор — UI покажет базовые колонки.
pub async fn source_columns(
    registrator_type: &str,
    registrator_ref: &str,
) -> Vec<contracts::quality::SourceColumn> {
    match registrator_type {
        "a026_wb_advert_daily" => a026_source_columns(registrator_ref)
            .await
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Колонки для `a026_wb_advert_daily`, как в списке «Статистика рекламы WB»:
/// Дата · Документ · Кампания · Кабинет · Расход.
async fn a026_source_columns(
    registrator_ref: &str,
) -> anyhow::Result<Vec<contracts::quality::SourceColumn>> {
    use contracts::quality::SourceColumn;
    use sea_orm::{ConnectionTrait, Statement};

    let document_id = document_id_from_registrator_ref(registrator_ref);
    let conn = crate::shared::data::db::get_connection();
    let sql = r#"
        SELECT
            a.document_date,
            a.document_no,
            a.advert_id,
            a.total_sum,
            COALESCE(c.description, a.connection_id) AS connection_name
        FROM a026_wb_advert_daily a
        LEFT JOIN a006_connection_mp c ON a.connection_id = c.id
        WHERE a.id = ?
        LIMIT 1
    "#;
    let rows = conn
        .query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            sql,
            [document_id.into()],
        ))
        .await?;
    let Some(row) = rows.first() else {
        return Ok(Vec::new());
    };

    let document_date: String = row.try_get("", "document_date").unwrap_or_default();
    let document_no: String = row.try_get("", "document_no").unwrap_or_default();
    let advert_id: i64 = row.try_get("", "advert_id").unwrap_or(0);
    let total_sum: f64 = row.try_get("", "total_sum").unwrap_or(0.0);
    let connection_name: String = row.try_get("", "connection_name").unwrap_or_default();

    // Дата — только дата без времени.
    let date_only = document_date
        .split(['T', ' '])
        .next()
        .unwrap_or(&document_date)
        .to_string();
    let campaign = if advert_id > 0 {
        advert_id.to_string()
    } else {
        "—".to_string()
    };

    Ok(vec![
        SourceColumn {
            label: "Дата".to_string(),
            value: date_only,
            align_right: false,
        },
        SourceColumn {
            label: "Документ".to_string(),
            value: document_no,
            align_right: false,
        },
        SourceColumn {
            label: "Кампания".to_string(),
            value: campaign,
            align_right: true,
        },
        SourceColumn {
            label: "Кабинет".to_string(),
            value: connection_name,
            align_right: false,
        },
        SourceColumn {
            label: "Расход".to_string(),
            value: format!("{total_sum:.2}"),
            align_right: true,
        },
    ])
}

/// Проверяет, существует ли исходный документ регистратора.
pub async fn source_document_exists(
    registrator_type: &str,
    registrator_ref: &str,
) -> anyhow::Result<bool> {
    use sea_orm::{ConnectionTrait, Statement};

    let table = match registrator_type {
        "a012_wb_sales" => "a012_wb_sales",
        "a013_ym_order" => "a013_ym_order",
        "a014_ozon_transactions" => "a014_ozon_transactions",
        "a016_ym_returns" => "a016_ym_returns",
        "a021_production_output" => "a021_production_output",
        "a023_purchase_of_goods" => "a023_purchase_of_goods",
        "a026_wb_advert_daily" => "a026_wb_advert_daily",
        "a015_wb_orders" => "a015_wb_orders",
        _ => return Ok(false),
    };

    let document_id = document_id_from_registrator_ref(registrator_ref);
    let sql = format!("SELECT COUNT(*) AS cnt FROM {table} WHERE id = ?");
    let conn = crate::shared::data::db::get_connection();
    let rows = conn
        .query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            &sql,
            [document_id.into()],
        ))
        .await?;
    let count: i64 = rows
        .first()
        .and_then(|r| r.try_get("", "cnt").ok())
        .unwrap_or(0);

    Ok(count > 0)
}
