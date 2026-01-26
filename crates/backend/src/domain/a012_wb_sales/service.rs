use super::repository;
use anyhow::Result;
use contracts::domain::a012_wb_sales::aggregate::WbSales;
use contracts::domain::common::AggregateId;
use uuid::Uuid;

/// Расчёт финансовых полей (план/факт) на основе данных P903
pub async fn calculate_financial_fields(document: &mut WbSales) -> Result<()> {
    // Получаем srid из document_no
    let srid = &document.header.document_no;

    // Запрашиваем данные из P903 по srid
    let p903_entries = crate::projections::p903_wb_finance_report::repository::search_by_srid(srid)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to query P903 for srid {}: {}", srid, e);
            Vec::new()
        });

    // Фильтруем только записи с supplier_oper_name = "Продажа"
    let sales_entries: Vec<_> = p903_entries
        .iter()
        .filter(|entry| entry.supplier_oper_name.as_deref() == Some("Продажа"))
        .collect();

    // Получаем acquiring_fee_pro из маркетплейса
    let acquiring_fee_pro =
        if let Ok(marketplace_uuid) = Uuid::parse_str(&document.header.marketplace_id) {
            crate::domain::a005_marketplace::service::get_by_id(marketplace_uuid)
                .await
                .ok()
                .flatten()
                .map(|m| m.acquiring_fee_pro)
                .unwrap_or(0.0)
        } else {
            tracing::warn!(
                "Invalid marketplace_id UUID: {}",
                document.header.marketplace_id
            );
            0.0
        };

    // Получаем базовые значения из документа
    let finished_price = document.line.finished_price.unwrap_or(0.0);
    let amount_line = document.line.amount_line.unwrap_or(0.0);
    let cost_of_production = document.line.cost_of_production.unwrap_or(0.0);

    // Проверяем наличие фактических данных в P903
    let has_fact_data = !sales_entries.is_empty()
        && sales_entries
            .iter()
            .any(|e| e.retail_amount.unwrap_or(0.0) > 0.0);

    // Устанавливаем флаг is_fact
    document.line.is_fact = Some(has_fact_data);

    // ПЛАН: заполняем ВСЕГДА (и в режиме план, и в режиме факт)
    let acquiring_fee_plan = acquiring_fee_pro * finished_price / 100.0;
    let commission_plan = finished_price - amount_line;
    let other_fee_plan = 0.0;

    document.line.sell_out_plan = Some(finished_price);
    document.line.acquiring_fee_plan = Some(acquiring_fee_plan);
    document.line.other_fee_plan = Some(other_fee_plan);
    document.line.commission_plan = Some(commission_plan);
    document.line.supplier_payout_plan = Some(amount_line - acquiring_fee_plan);
    document.line.profit_plan = Some(
        finished_price - acquiring_fee_plan - commission_plan - other_fee_plan - cost_of_production,
    );

    // ФАКТ: заполняем только если есть данные P903
    if has_fact_data {
        // Агрегируем значения из всех записей продаж
        let retail_amount: f64 = sales_entries.iter().filter_map(|e| e.retail_amount).sum();
        let acquiring_fee: f64 = sales_entries.iter().filter_map(|e| e.acquiring_fee).sum();
        let rebill_logistic_cost: f64 = sales_entries
            .iter()
            .filter_map(|e| e.rebill_logistic_cost)
            .sum();
        let ppvz_vw: f64 = sales_entries.iter().filter_map(|e| e.ppvz_vw).sum();
        let ppvz_vw_nds: f64 = sales_entries.iter().filter_map(|e| e.ppvz_vw_nds).sum();
        let ppvz_for_pay: f64 = sales_entries.iter().filter_map(|e| e.ppvz_for_pay).sum();

        document.line.sell_out_fact = Some(retail_amount);
        document.line.acquiring_fee_fact = Some(acquiring_fee);
        document.line.other_fee_fact = Some(rebill_logistic_cost);
        document.line.commission_fact = Some(ppvz_vw + ppvz_vw_nds);
        document.line.supplier_payout_fact = Some(ppvz_for_pay);

        let commission_fact = ppvz_vw + ppvz_vw_nds;
        document.line.profit_fact = Some(
            retail_amount
                - acquiring_fee
                - commission_fact
                - rebill_logistic_cost
                - cost_of_production,
        );

        tracing::info!(
            "Calculated PLAN and FACT fields for document {} (srid: {})",
            document.base.id.as_string(),
            srid
        );
    } else {
        tracing::info!(
            "Calculated PLAN fields only for document {} (srid: {}) - no P903 data",
            document.base.id.as_string(),
            srid
        );
    }

    Ok(())
}

/// Автозаполнение marketplace_product_ref и nomenclature_ref
pub async fn auto_fill_references(document: &mut WbSales) -> Result<()> {
    // Автозаполнение marketplace_product_ref если пустой
    if document.marketplace_product_ref.is_none() {
        // Ищем по connection_mp_ref и supplier_article (используем как marketplace_sku)
        let marketplace_product =
            crate::domain::a007_marketplace_product::service::get_by_connection_and_sku(
                &document.header.connection_id,
                &document.line.supplier_article,
            )
            .await?;

        let mp_id = if let Some(existing) = marketplace_product {
            existing.base.id.as_string()
        } else {
            // Создаем новый a007_marketplace_product
            let dto =
                contracts::domain::a007_marketplace_product::aggregate::MarketplaceProductDto {
                    id: None,
                    code: Some(format!("WB-AUTO-{}", uuid::Uuid::new_v4())),
                    description: if document.line.name.trim().is_empty() {
                        format!("Артикул: {}", document.line.supplier_article)
                    } else {
                        document.line.name.clone()
                    },
                    marketplace_ref: document.header.marketplace_id.clone(),
                    connection_mp_ref: document.header.connection_id.clone(),
                    marketplace_sku: document.line.supplier_article.clone(),
                    barcode: Some(document.line.barcode.clone()),
                    article: document.line.supplier_article.clone(),
                    brand: None,
                    category_id: None,
                    category_name: None,
                    last_update: Some(chrono::Utc::now()),
                    nomenclature_ref: None,
                    comment: Some(format!(
                        "Автоматически создано при импорте WB Sales [{}]",
                        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
                    )),
                };

            let created_id = crate::domain::a007_marketplace_product::service::create(dto).await?;
            tracing::info!("Created new marketplace_product with id: {}", created_id);
            created_id.to_string()
        };

        document.marketplace_product_ref = Some(mp_id);
    }

    // Автозаполнение nomenclature_ref из marketplace_product
    if document.nomenclature_ref.is_none() {
        if let Some(ref mp_ref) = document.marketplace_product_ref {
            if let Ok(mp_uuid) = uuid::Uuid::parse_str(mp_ref) {
                if let Ok(Some(mp)) =
                    crate::domain::a007_marketplace_product::service::get_by_id(mp_uuid).await
                {
                    if let Some(nom_ref) = mp.nomenclature_ref {
                        document.nomenclature_ref = Some(nom_ref);
                    }
                }
            }
        }
    }

    Ok(())
}

pub async fn store_document_with_raw(mut document: WbSales, raw_json: &str) -> Result<Uuid> {
    let raw_ref = crate::shared::data::raw_storage::save_raw_json(
        "WB",
        "WB_Sales",
        &document.header.document_no,
        raw_json,
        document.source_meta.fetched_at,
    )
    .await?;

    document.source_meta.raw_payload_ref = raw_ref;

    // Автозаполнение ссылок
    auto_fill_references(&mut document).await?;

    document
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;
    document.before_write();

    let id = repository::upsert_document(&document).await?;

    // Проводим документ если is_posted = true
    if document.is_posted {
        if let Err(e) = super::posting::post_document(id).await {
            tracing::error!("Failed to post WB Sales document: {}", e);
            // Не останавливаем выполнение, т.к. документ уже сохранен
        }
    } else {
        // Если is_posted = false, удаляем проекции (если были)
        if let Err(e) = crate::projections::p900_mp_sales_register::service::delete_by_registrator(
            &id.to_string(),
        )
        .await
        {
            tracing::error!("Failed to delete projections for WB Sales document: {}", e);
        }
    }

    Ok(id)
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbSales>> {
    repository::get_by_id(id).await
}

pub async fn get_by_document_no(document_no: &str) -> Result<Option<WbSales>> {
    repository::get_by_document_no(document_no).await
}

/// Get by sale_id (saleID from WB API) - used for deduplication
pub async fn get_by_sale_id(sale_id: &str) -> Result<Option<WbSales>> {
    repository::get_by_sale_id(sale_id).await
}

pub async fn list_all() -> Result<Vec<WbSales>> {
    repository::list_all().await
}

pub async fn delete(id: Uuid) -> Result<bool> {
    repository::soft_delete(id).await
}
