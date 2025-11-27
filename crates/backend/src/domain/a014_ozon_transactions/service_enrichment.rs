use contracts::domain::a014_ozon_transactions::aggregate::OzonTransactions;
use uuid::Uuid;

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
    let lines = if posting_ref.1 == "A010" {
        // FBS Posting
        let posting = crate::domain::a010_ozon_fbs_posting::repository::get_by_id(posting_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("FBS Posting not found: {}", posting_id))?;
        
        posting.lines.into_iter().map(|line| {
            (
                line.offer_id.clone(),
                line.price_effective,
                line.product_id.clone(),
                None::<String>, // FBS пока не имеет nomenclature_ref в lines
            )
        }).collect::<Vec<_>>()
    } else if posting_ref.1 == "A011" {
        // FBO Posting
        let posting = crate::domain::a011_ozon_fbo_posting::repository::get_by_id(posting_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("FBO Posting not found: {}", posting_id))?;
        
        posting.lines.into_iter().map(|line| {
            (
                line.offer_id.clone(),
                line.price_effective,
                line.product_id.clone(),
                None::<String>, // FBO пока не имеет nomenclature_ref in lines
            )
        }).collect::<Vec<_>>()
    } else {
        tracing::warn!("Unknown posting type: {}", posting_ref.1);
        return Ok(());
    };

    // Создаем lookup map: sku -> (price, product_id, nomenclature_ref)
    // SKU в транзакциях соответствует offer_id в постингах, но нужно проверить по числовому значению
    let mut price_map: std::collections::HashMap<i64, (Option<f64>, String, Option<String>)> = 
        std::collections::HashMap::new();
    
    for (offer_id, price, product_id, nomenclature_ref) in lines {
        // Пробуем распарсить offer_id как число (SKU)
        if let Ok(sku) = offer_id.parse::<i64>() {
            price_map.insert(sku, (price, product_id, nomenclature_ref));
        }
    }

    // Обогащаем items
    let mut total_price = 0.0;
    let mut enriched_items = Vec::new();

    for item in &transaction.items {
        let mut enriched = item.clone();
        
        if let Some((price_opt, product_id, nomenclature_ref)) = price_map.get(&item.sku) {
            enriched.price = *price_opt;
            enriched.marketplace_product_ref = Some(product_id.clone());
            enriched.nomenclature_ref = nomenclature_ref.clone();
            
            if let Some(price) = price_opt {
                total_price += price;
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

