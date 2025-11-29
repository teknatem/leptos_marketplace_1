use contracts::domain::a014_ozon_transactions::aggregate::OzonTransactions;
use uuid::Uuid;

/// Структура для хранения данных линии из постинга
struct PostingLineData {
    product_id: String,
    offer_id: String,
    price_effective: Option<f64>,
    barcode: Option<String>,
    name: String,
}

/// Обогатить items данными из связанного постинга (FBS или FBO)
pub async fn enrich_items_from_posting(
    transaction: &mut OzonTransactions,
) -> anyhow::Result<()> {
    // Проверяем наличие ссылки на постинг
    let posting_ref = match (&transaction.posting_ref, &transaction.posting_ref_type) {
        (Some(ref_id), Some(ref_type)) => (ref_id.clone(), ref_type.clone()),
        _ => {
            tracing::warn!("No posting reference found for transaction, skipping enrichment");
            return Ok(());
        }
    };

    let posting_id = Uuid::parse_str(&posting_ref.0)
        .map_err(|e| anyhow::anyhow!("Invalid posting_ref UUID: {}", e))?;

    // Получаем постинг в зависимости от типа (A010 или A011)
    let lines: Vec<PostingLineData> = if posting_ref.1 == "A010" {
        // FBS Posting
        let posting = crate::domain::a010_ozon_fbs_posting::repository::get_by_id(posting_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("FBS Posting not found: {}", posting_id))?;
        
        posting.lines.into_iter().map(|line| PostingLineData {
            product_id: line.product_id,
            offer_id: line.offer_id,
            price_effective: line.price_effective,
            barcode: line.barcode,
            name: line.name,
        }).collect()
    } else if posting_ref.1 == "A011" {
        // FBO Posting
        let posting = crate::domain::a011_ozon_fbo_posting::repository::get_by_id(posting_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("FBO Posting not found: {}", posting_id))?;
        
        posting.lines.into_iter().map(|line| PostingLineData {
            product_id: line.product_id,
            offer_id: line.offer_id,
            price_effective: line.price_effective,
            barcode: line.barcode,
            name: line.name,
        }).collect()
    } else {
        tracing::warn!("Unknown posting type: {}", posting_ref.1);
        return Ok(());
    };

    // Создаем lookup map: sku (product_id) -> PostingLineData
    let mut line_map: std::collections::HashMap<i64, PostingLineData> = 
        std::collections::HashMap::new();
    
    for line_data in lines {
        // Используем product_id для сопоставления (это числовой SKU в OZON)
        if let Ok(sku) = line_data.product_id.parse::<i64>() {
            line_map.insert(sku, line_data);
        }
    }

    // Обогащаем items
    let mut total_price = 0.0;
    let mut enriched_items = Vec::new();

    // Получаем connection и marketplace из заголовка транзакции
    let connection_mp_ref = transaction.header.connection_id.clone();
    let marketplace_ref = transaction.header.marketplace_id.clone();

    for item in &transaction.items {
        let mut enriched = item.clone();
        
        if let Some(line_data) = line_map.get(&item.sku) {
            enriched.price = line_data.price_effective;
            
            if let Some(price) = line_data.price_effective {
                total_price += price;
            }

            // Устанавливаем marketplace_product_ref как артикул (offer_id)
            enriched.marketplace_product_ref = Some(line_data.offer_id.clone());

            // Используем find_or_create_for_sale для получения a007 и nomenclature
            let marketplace_sku = item.sku.to_string();
            match crate::domain::a007_marketplace_product::service::find_or_create_for_sale(
                crate::domain::a007_marketplace_product::service::FindOrCreateParams {
                    marketplace_ref: marketplace_ref.clone(),
                    connection_mp_ref: connection_mp_ref.clone(),
                    marketplace_sku: marketplace_sku.clone(),
                    barcode: line_data.barcode.clone(),
                    title: line_data.name.clone(),
                },
            ).await {
                Ok(mp_uuid) => {
                    // Получаем nomenclature_ref из найденного a007
                    if let Ok(Some(product)) = 
                        crate::domain::a007_marketplace_product::service::get_by_id(mp_uuid).await 
                    {
                        // Если есть nomenclature_ref в a007, получаем код 1С из a004
                        if let Some(ref nom_uuid_str) = product.nomenclature_ref {
                            if let Ok(nom_uuid) = Uuid::parse_str(nom_uuid_str) {
                                if let Ok(Some(nomenclature)) = 
                                    crate::domain::a004_nomenclature::service::get_by_id(nom_uuid).await 
                                {
                                    // Устанавливаем код 1С как nomenclature_ref
                                    enriched.nomenclature_ref = Some(nomenclature.base.code.clone());
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to find/create marketplace product for SKU {}: {}",
                        marketplace_sku, e
                    );
                }
            }
        } else {
            tracing::warn!("No matching line found in posting for SKU: {}", item.sku);
        }
        
        enriched_items.push(enriched);
    }

    // Рассчитываем ratio для каждого item
    if total_price > 0.0 {
        for item in &mut enriched_items {
            if let Some(price) = item.price {
                item.ratio = Some(price / total_price);
            }
        }
    }

    // Обновляем items в транзакции
    transaction.items = enriched_items;

    tracing::info!(
        "Enriched {} items from posting {} (total_price: {:.2})",
        transaction.items.len(),
        posting_ref.0,
        total_price
    );

    Ok(())
}

