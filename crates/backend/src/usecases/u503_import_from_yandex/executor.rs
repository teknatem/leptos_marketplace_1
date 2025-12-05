use super::{progress_tracker::ProgressTracker, yandex_api_client::YandexApiClient};
use crate::domain::a007_marketplace_product;
use anyhow::Result;
use contracts::domain::common::AggregateId;
use contracts::usecases::u503_import_from_yandex::{
    progress::ImportStatus,
    request::ImportRequest,
    response::{ImportResponse, ImportStartStatus},
};
use std::sync::Arc;
use uuid::Uuid;

/// Executor для UseCase импорта из Yandex Market
pub struct ImportExecutor {
    api_client: Arc<YandexApiClient>,
    progress_tracker: Arc<ProgressTracker>,
}

impl ImportExecutor {
    pub fn new(progress_tracker: Arc<ProgressTracker>) -> Self {
        Self {
            api_client: Arc::new(YandexApiClient::new()),
            progress_tracker,
        }
    }

    /// Запустить импорт (создает async task и возвращает session_id)
    pub async fn start_import(&self, request: ImportRequest) -> Result<ImportResponse> {
        // Валидация запроса
        let connection_id = Uuid::parse_str(&request.connection_id)
            .map_err(|_| anyhow::anyhow!("Invalid connection_id"))?;

        // Получить подключение к маркетплейсу
        let connection = crate::domain::a006_connection_mp::service::get_by_id(connection_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;

        // Создать сессию импорта
        let session_id = Uuid::new_v4().to_string();
        self.progress_tracker.create_session(session_id.clone());

        // Добавить агрегаты для отслеживания
        for aggregate_index in &request.target_aggregates {
            let aggregate_name = match aggregate_index.as_str() {
                "a007_marketplace_product" => "Товары маркетплейса",
                "a013_ym_order" => "Заказы Yandex Market",
                "a016_ym_returns" => "Возвраты Yandex Market",
                _ => "Unknown",
            };
            self.progress_tracker.add_aggregate(
                &session_id,
                aggregate_index.clone(),
                aggregate_name.to_string(),
            );
        }

        // Запустить импорт в фоне
        let self_clone = Arc::new(self.clone());
        let session_id_clone = session_id.clone();
        let request_clone = request.clone();
        let connection_clone = connection.clone();

        tokio::spawn(async move {
            if let Err(e) = self_clone
                .run_import(&session_id_clone, &request_clone, &connection_clone)
                .await
            {
                tracing::error!("Import failed: {}", e);
                self_clone.progress_tracker.add_error(
                    &session_id_clone,
                    None,
                    format!("Import failed: {}", e),
                    None,
                );
                self_clone
                    .progress_tracker
                    .complete_session(&session_id_clone, ImportStatus::Failed);
            }
        });

        Ok(ImportResponse {
            session_id,
            status: ImportStartStatus::Started,
            message: "Импорт запущен".to_string(),
        })
    }

    /// Получить текущий прогресс импорта
    pub fn get_progress(
        &self,
        session_id: &str,
    ) -> Option<contracts::usecases::u503_import_from_yandex::progress::ImportProgress> {
        self.progress_tracker.get_progress(session_id)
    }

    /// Выполнить импорт
    async fn run_import(
        &self,
        session_id: &str,
        request: &ImportRequest,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
    ) -> Result<()> {
        tracing::info!("Starting Yandex Market import for session: {}", session_id);

        for aggregate_index in &request.target_aggregates {
            match aggregate_index.as_str() {
                "a007_marketplace_product" => {
                    self.import_marketplace_products(session_id, connection)
                        .await?;
                }
                "a013_ym_order" => {
                    // Import YM orders with date period
                    self.import_ym_orders(
                        session_id,
                        connection,
                        request.date_from,
                        request.date_to,
                    )
                    .await?;
                }
                "a016_ym_returns" => {
                    // Import YM returns with date period
                    self.import_ym_returns(
                        session_id,
                        connection,
                        request.date_from,
                        request.date_to,
                    )
                    .await?;
                }
                _ => {
                    let msg = format!("Unknown aggregate: {}", aggregate_index);
                    tracing::warn!("{}", msg);
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.clone()),
                        msg,
                        None,
                    );
                }
            }
        }

        // Завершить импорт
        let final_status = if self
            .progress_tracker
            .get_progress(session_id)
            .map(|p| p.total_errors > 0)
            .unwrap_or(false)
        {
            ImportStatus::CompletedWithErrors
        } else {
            ImportStatus::Completed
        };

        self.progress_tracker
            .complete_session(session_id, final_status);
        tracing::info!("Import completed for session: {}", session_id);

        Ok(())
    }

    /// Импорт товаров из Yandex Market
    async fn import_marketplace_products(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
    ) -> Result<()> {
        tracing::info!("Importing marketplace products for session: {}", session_id);

        let aggregate_index = "a007_marketplace_product";
        let page_size = 100;
        let mut total_processed = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;
        let mut total_barcodes_imported = 0;
        let mut page_token: Option<String> = None;
        let mut expected_total: Option<i32> = None;

        // Получаем товары страницами через Yandex Market API
        loop {
            tracing::info!(
                "Fetching page with page_token: {:?}",
                page_token.as_ref().map(|t| &t[..t.len().min(50)])
            );

            let list_response = self
                .api_client
                .fetch_product_list(connection, page_size, page_token.clone())
                .await?;

            // Сохраняем paging ДО того, как заберем entries
            let next_page_token = list_response.result.paging.next_page_token.clone();

            // Если API вернул total, сохраняем его (только при первом запросе)
            if expected_total.is_none() {
                if let Some(total) = list_response.result.paging.total {
                    expected_total = Some(total as i32);
                    tracing::info!("API returned total count: {}", total);
                }
            }

            let entries = list_response.result.offer_mapping_entries;
            let first_ids_preview: Vec<String> = entries
                .iter()
                .take(3)
                .map(|e| e.offer.offer_id.clone())
                .collect();
            tracing::info!(
                "Page stats: token_in={:?}, token_out={:?}, batch_size={}, first_ids={:?}",
                page_token.as_ref().map(|t| &t[..t.len().min(50)]),
                next_page_token.as_ref().map(|t| &t[..t.len().min(50)]),
                entries.len(),
                first_ids_preview
            );
            if entries.is_empty() {
                tracing::info!("Received empty batch, stopping pagination");
                break;
            }

            let batch_size = entries.len();
            tracing::info!(
                "Processing batch: {} items, total so far: {}",
                batch_size,
                total_processed
            );

            // Обрабатываем каждый товар напрямую из offer-mappings
            // (они уже содержат все данные, включая barcodes)
            for offer_mapping_entry in entries {
                let offer = &offer_mapping_entry.offer;
                let mapping = &offer_mapping_entry.mapping;

                // Логируем информацию о штрихкодах для отладки
                tracing::debug!(
                    "Product {} has {} barcode(s): {:?}",
                    offer.offer_id,
                    offer.barcodes.len(),
                    offer.barcodes
                );

                let product_name = mapping
                    .as_ref()
                    .and_then(|m| m.market_sku_name.clone())
                    .or_else(|| offer.name.clone())
                    .unwrap_or_else(|| "Без названия".to_string());

                let display_name = format!("{} - {}", offer.offer_id, product_name);

                self.progress_tracker.set_current_item(
                    session_id,
                    aggregate_index,
                    Some(display_name),
                );

                match self
                    .process_product_from_offer(connection, offer, mapping)
                    .await
                {
                    Ok((is_new, barcodes_count)) => {
                        total_processed += 1;
                        total_barcodes_imported += barcodes_count as i32;
                        if is_new {
                            total_inserted += 1;
                        } else {
                            total_updated += 1;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to process product {}: {}", offer.offer_id, e);
                        self.progress_tracker.add_error(
                            session_id,
                            Some(aggregate_index.to_string()),
                            format!("Failed to process product {}", offer.offer_id),
                            Some(e.to_string()),
                        );
                    }
                }

                // Обновить прогресс (используем expected_total если API его вернул)
                self.progress_tracker.update_aggregate(
                    session_id,
                    aggregate_index,
                    total_processed,
                    expected_total,
                    total_inserted,
                    total_updated,
                );

                // Обновить счетчик штрихкодов
                self.progress_tracker.update_barcodes_count(
                    session_id,
                    aggregate_index,
                    total_barcodes_imported,
                );
            }

            // Очистить текущий элемент после страницы
            self.progress_tracker
                .set_current_item(session_id, aggregate_index, None);

            // Обновляем page_token для следующей страницы
            let old_token = page_token.clone();
            page_token = next_page_token;

            tracing::info!(
                "Page token updated: old={:?}, new={:?}",
                old_token.as_ref().map(|t| &t[..t.len().min(50)]),
                page_token.as_ref().map(|t| &t[..t.len().min(50)])
            );

            // Если нет next_page_token, значит это последняя страница
            if page_token.is_none() {
                tracing::info!("No next_page_token, stopping pagination");
                break;
            }

            // Защита от зацикливания: если токен не изменился, прекращаем
            if old_token.is_some() && old_token == page_token {
                tracing::warn!(
                    "Page token did not change, stopping to prevent infinite loop. Token: {:?}",
                    page_token.as_ref().map(|t| &t[..t.len().min(50)])
                );
                self.progress_tracker.add_error(
                    session_id,
                    Some(aggregate_index.to_string()),
                    "Pagination stopped".to_string(),
                    Some(
                        "API returned the same page token, possible API issue or server caching"
                            .to_string(),
                    ),
                );
                break;
            }
        }

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);
        tracing::info!(
            "Marketplace products import completed: processed={}, inserted={}, updated={}",
            total_processed,
            total_inserted,
            total_updated
        );

        Ok(())
    }

    /// Обработать один товар из YandexOffer (offer-mappings endpoint)
    /// Возвращает (is_new, barcodes_count)
    async fn process_product_from_offer(
        &self,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        offer: &super::yandex_api_client::YandexOffer,
        _mapping: &Option<super::yandex_api_client::YandexMapping>,
    ) -> Result<(bool, usize)> {
        use contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct;

        // Используем offer_id как marketplace_sku
        let marketplace_sku = offer.offer_id.clone();
        let existing = a007_marketplace_product::repository::get_by_connection_and_sku(
            &connection.base.id.as_string(),
            &marketplace_sku,
        )
        .await?;

        // Берем первый barcode из списка
        let barcode = offer.barcodes.first().cloned();

        // Получаем category_id и category_name - YandexMapping не содержит категории
        // Используем данные из offer.category если есть
        let (category_id, category_name) = (None, offer.category.clone());

        // Получаем название товара из offer.name для description
        let product_title = offer
            .name
            .clone()
            .unwrap_or_else(|| "Без названия".to_string());

        if let Some(mut existing_product) = existing {
            // Обновляем существующий товар
            tracing::debug!("Updating existing product: {}", marketplace_sku);

            existing_product.base.code = offer.offer_id.clone();
            existing_product.base.description = product_title.clone();
            existing_product.marketplace_sku = marketplace_sku;
            existing_product.barcode = barcode.clone();
            existing_product.article = offer.offer_id.clone();
            existing_product.brand = offer.vendor.clone();
            existing_product.category_id = category_id;
            existing_product.category_name = category_name;
            existing_product.last_update = Some(chrono::Utc::now());
            existing_product.before_write();

            a007_marketplace_product::repository::update(&existing_product).await?;

            // Импорт всех штрихкодов в проекцию p901
            let barcodes_count = self
                .import_barcodes_to_p901(
                    &offer.barcodes,
                    &offer.offer_id,
                    &existing_product.nomenclature_ref,
                )
                .await?;

            Ok((false, barcodes_count))
        } else {
            // Создаем новый товар
            tracing::debug!("Inserting new product: {}", marketplace_sku);

            let mut new_product = MarketplaceProduct::new_for_insert(
                offer.offer_id.clone(),
                product_title.clone(),
                connection.marketplace_id.clone(),
                connection.base.id.as_string(),
                marketplace_sku,
                barcode,
                offer.offer_id.clone(),
                offer.vendor.clone(),
                category_id,
                category_name,
                Some(chrono::Utc::now()),
                None, // nomenclature_ref
                None, // comment
            );

            // Автоматический поиск номенклатуры по артикулу
            let _ =
                a007_marketplace_product::service::search_and_set_nomenclature(&mut new_product)
                    .await;

            a007_marketplace_product::repository::insert(&new_product).await?;

            // Импорт всех штрихкодов в проекцию p901
            let barcodes_count = self
                .import_barcodes_to_p901(
                    &offer.barcodes,
                    &offer.offer_id,
                    &new_product.nomenclature_ref,
                )
                .await?;

            Ok((true, barcodes_count))
        }
    }

    /// Импортировать все штрихкоды из Yandex в проекцию p901_nomenclature_barcodes
    /// Автоматически ищет соответствие nomenclature_ref из 1C по штрихкоду
    async fn import_barcodes_to_p901(
        &self,
        barcodes: &[String],
        article: &str,
        product_nomenclature_id: &Option<String>,
    ) -> Result<usize> {
        use crate::projections::p901_nomenclature_barcodes::{repository, service};

        if barcodes.is_empty() {
            tracing::info!(
                "Product {} has no barcodes, skipping barcode import",
                article
            );
            return Ok(0);
        }

        tracing::info!(
            "Importing {} barcode(s) for product {} (source: YM)",
            barcodes.len(),
            article
        );

        let mut imported_count = 0;

        for barcode in barcodes {
            // Пропускаем пустые штрихкоды
            if barcode.trim().is_empty() {
                tracing::warn!("Empty barcode found for product {}, skipping", article);
                continue;
            }

            // Определяем nomenclature_ref:
            // 1. Если товар уже связан с номенклатурой - использовать её
            // 2. Если нет - попытаться найти по штрихкоду в базе 1C
            let nomenclature_ref = if let Some(ref nom_id) = product_nomenclature_id {
                Some(nom_id.clone())
            } else {
                // Ищем nomenclature_ref по штрихкоду из источника 1C
                match service::find_nomenclature_ref_by_barcode_from_1c(barcode).await {
                    Ok(found_ref) => {
                        if found_ref.is_some() {
                            tracing::debug!(
                                "Found nomenclature_ref for barcode {} from 1C: {:?}",
                                barcode,
                                found_ref
                            );
                        }
                        found_ref
                    }
                    Err(e) => {
                        tracing::warn!("Failed to lookup barcode {} in 1C: {}", barcode, e);
                        None
                    }
                }
            };

            // Создаем entry для штрихкода
            let entry = match service::create_entry(
                barcode.clone(),
                "YM".to_string(),
                nomenclature_ref.clone(),
                Some(article.to_string()),
            ) {
                Ok(e) => e,
                Err(e) => {
                    tracing::error!("Failed to create entry for barcode {}: {}", barcode, e);
                    continue;
                }
            };

            // Upsert в базу
            match repository::upsert_entry(&entry).await {
                Ok(_) => {
                    imported_count += 1;
                    tracing::info!(
                        "✓ Imported barcode {} (source: YM, article: {}, nomenclature_ref: {:?})",
                        barcode,
                        article,
                        nomenclature_ref
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to upsert barcode {} to database: {}", barcode, e);
                }
            }
        }

        tracing::info!(
            "Finished importing barcodes for product {}: {} barcode(s) imported",
            article,
            imported_count
        );
        Ok(imported_count)
    }

    /// Импорт заказов Yandex Market
    async fn import_ym_orders(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<()> {
        use crate::domain::a002_organization;
        use crate::domain::a013_ym_order;
        use contracts::domain::a013_ym_order::aggregate::{
            YmOrder, YmOrderHeader, YmOrderLine, YmOrderSourceMeta, YmOrderState,
        };

        tracing::info!("Importing Yandex Market orders for session: {}", session_id);

        let aggregate_index = "a013_ym_order";
        let mut total_processed = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;

        // 1. Resolve organization
        let organization_id =
            match a002_organization::repository::get_by_description(&connection.organization)
                .await?
            {
                Some(org) => org.base.id.as_string(),
                None => {
                    let msg = format!("Organization '{}' not found", connection.organization);
                    tracing::error!("{}", msg);
                    anyhow::bail!("{}", msg);
                }
            };

        // 2. Fetch orders from API with date period
        tracing::info!(
            "Fetching YM orders for period {} to {}",
            date_from.format("%Y-%m-%d"),
            date_to.format("%Y-%m-%d")
        );

        let orders = self
            .api_client
            .fetch_orders(connection, date_from, date_to)
            .await?;

        tracing::info!("Received {} orders from YM API", orders.len());

        // 3. Process each order
        for order in orders {
            let order_id_str = order.id.to_string();
            self.progress_tracker.set_current_item(
                session_id,
                aggregate_index,
                Some(format!("YM Order {}", order_id_str)),
            );

            // Check if exists
            let existing = a013_ym_order::service::get_by_document_no(&order_id_str).await?;
            let is_new = existing.is_none();

            // Fetch detailed order info to get realDeliveryDate
            let order_details = match self
                .api_client
                .fetch_order_details(connection, order.id)
                .await
            {
                Ok(details) => {
                    // Log full delivery structure for debugging
                    if let Some(delivery) = &details.delivery {
                        let delivery_json =
                            serde_json::to_string_pretty(delivery).unwrap_or_default();
                        tracing::info!(
                            "Order {} delivery structure:\n{}",
                            order_id_str,
                            delivery_json
                        );

                        if let Some(dates) = &delivery.dates {
                            tracing::info!(
                                "Order {} has dates.realDeliveryDate: {:?}",
                                order_id_str,
                                dates.real_delivery_date
                            );
                            tracing::info!(
                                "Order {} has dates.fromDate: {:?}",
                                order_id_str,
                                dates.from_date
                            );
                            tracing::info!(
                                "Order {} has dates.toDate: {:?}",
                                order_id_str,
                                dates.to_date
                            );
                        } else {
                            tracing::warn!("Order {} delivery has NO dates field", order_id_str);
                        }
                    } else {
                        tracing::warn!("Order {} has NO delivery field", order_id_str);
                    }
                    details
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to fetch details for order {}: {}, using basic data",
                        order_id_str,
                        e
                    );
                    order.clone() // Use original order if details fetch fails
                }
            };

            // Map lines (use order_details for accurate data)
            let lines: Vec<YmOrderLine> = order_details
                .items
                .iter()
                .map(|item| {
                    let price_list = item.price;
                    let discount = item.subsidy.unwrap_or(0.0);
                    let price_effective = price_list.map(|p| p - discount);

                    // Calculate amount_line as price * count
                    let amount_line = price_list.map(|p| p * item.count as f64);

                    // Serialize item subsidies to JSON if present
                    let subsidies_json = item
                        .subsidies
                        .as_ref()
                        .and_then(|s| serde_json::to_string(s).ok());

                    YmOrderLine {
                        line_id: item.id.to_string(),
                        shop_sku: item.shop_sku.clone().unwrap_or_default(),
                        offer_id: item.offer_id.clone().unwrap_or_default(),
                        name: item.name.clone().unwrap_or_default(),
                        qty: item.count as f64,
                        price_list,
                        discount_total: item.subsidy,
                        price_effective,
                        amount_line,
                        currency_code: order_details.currency.clone(),
                        buyer_price: item.buyer_price,
                        subsidies_json,
                        status: item.status.clone(),
                        price_plan: Some(0.0),
                        marketplace_product_ref: None,
                        nomenclature_ref: None,
                    }
                })
                .collect();

            // Skip orders with no items
            if lines.is_empty() {
                tracing::warn!("Order {} has no items, skipping", order_id_str);
                continue;
            }

            // Parse dates with multiple formats
            let status_changed_at = order_details
                .status_update_date
                .as_ref()
                .and_then(|s| parse_ym_date(s));

            // Extract realDeliveryDate from detailed order info
            let delivery_date = order_details
                .delivery
                .as_ref()
                .and_then(|d| d.dates.as_ref())
                .and_then(|dates| dates.real_delivery_date.as_ref())
                .and_then(|s| parse_ym_date(s));

            let creation_date = order_details
                .creation_date
                .as_ref()
                .and_then(|s| parse_ym_date(s));

            // Clone status before consuming it
            let status_raw = order_details
                .status
                .clone()
                .unwrap_or_else(|| "UNKNOWN".to_string());
            let status_norm = normalize_ym_status(&status_raw);

            // Serialize order-level subsidies to JSON if present
            let subsidies_json = order_details
                .subsidies
                .as_ref()
                .and_then(|s| serde_json::to_string(s).ok());

            // Create aggregate
            let header = YmOrderHeader {
                document_no: order_id_str.clone(),
                connection_id: connection.base.id.as_string(),
                organization_id: organization_id.clone(),
                marketplace_id: connection.marketplace_id.clone(),
                campaign_id: connection
                    .supplier_id
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                total_amount: order_details.total,
                currency: order_details.currency.clone(),
                items_total: order_details.items_total,
                delivery_total: order_details.delivery_total,
                subsidies_json,
            };

            let state = YmOrderState {
                status_raw,
                substatus_raw: order_details.substatus.clone(),
                status_norm,
                status_changed_at,
                updated_at_source: status_changed_at,
                creation_date,
                delivery_date,
            };

            let source_meta = YmOrderSourceMeta {
                raw_payload_ref: String::new(), // Will be filled by service
                fetched_at: chrono::Utc::now(),
                document_version: 1,
            };

            let document = YmOrder::new_for_insert(
                order_id_str.clone(),
                format!("YM Order {}", order_id_str),
                header,
                lines,
                state,
                source_meta,
                true, // is_posted = true при загрузке через API
            );

            // Save with raw JSON (use detailed order data for raw storage)
            let raw_json = serde_json::to_string(&order_details)?;
            match a013_ym_order::service::store_document_with_raw(document, &raw_json).await {
                Ok(_) => {
                    total_processed += 1;
                    if is_new {
                        total_inserted += 1;
                    } else {
                        total_updated += 1;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to process YM order {}: {}", order_id_str, e);
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        format!("Failed to process order {}", order_id_str),
                        Some(e.to_string()),
                    );
                }
            }

            self.progress_tracker.update_aggregate(
                session_id,
                aggregate_index,
                total_processed,
                None,
                total_inserted,
                total_updated,
            );
        }

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);
        tracing::info!(
            "YM orders import completed: processed={}, inserted={}, updated={}",
            total_processed,
            total_inserted,
            total_updated
        );

        Ok(())
    }

    /// Импорт возвратов Yandex Market
    async fn import_ym_returns(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<()> {
        use crate::domain::a002_organization;
        use crate::domain::a016_ym_returns;
        use contracts::domain::a016_ym_returns::aggregate::{
            YmReturn, YmReturnDecision, YmReturnHeader, YmReturnLine, YmReturnSourceMeta,
            YmReturnState,
        };

        tracing::info!(
            "Importing Yandex Market returns for session: {}",
            session_id
        );

        let aggregate_index = "a016_ym_returns";
        let mut total_processed = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;

        // 1. Resolve organization
        let organization_id =
            match a002_organization::repository::get_by_description(&connection.organization)
                .await?
            {
                Some(org) => org.base.id.as_string(),
                None => {
                    let msg = format!("Organization '{}' not found", connection.organization);
                    tracing::error!("{}", msg);
                    anyhow::bail!("{}", msg);
                }
            };

        // 2. Fetch returns from API with date period
        tracing::info!(
            "Fetching YM returns for period {} to {}",
            date_from.format("%Y-%m-%d"),
            date_to.format("%Y-%m-%d")
        );

        let returns = self
            .api_client
            .fetch_returns(connection, date_from, date_to)
            .await?;

        tracing::info!("Received {} returns from YM API", returns.len());

        // 3. Process each return
        for return_item in returns {
            let return_id = return_item.id;
            let return_id_str = return_id.to_string();
            self.progress_tracker.set_current_item(
                session_id,
                aggregate_index,
                Some(format!("YM Return {}", return_id_str)),
            );

            // Check if exists
            let existing = a016_ym_returns::service::get_by_return_id(return_id).await?;
            let is_new = existing.is_none();

            // Map lines (items in return)
            let lines: Vec<YmReturnLine> = return_item
                .items
                .iter()
                .map(|item| {
                    // Map decisions
                    let decisions: Vec<YmReturnDecision> = item
                        .decisions
                        .iter()
                        .map(|d| YmReturnDecision {
                            decision_type: d.decision_type.clone().unwrap_or_default(),
                            amount: d.amount.as_ref().and_then(|a| a.value),
                            currency: d.amount.as_ref().and_then(|a| a.currency_id.clone()),
                            partner_compensation_amount: d
                                .partner_compensation_amount
                                .as_ref()
                                .and_then(|a| a.value),
                            comment: d.comment.clone(),
                        })
                        .collect();

                    // Map photos URLs
                    let photos: Vec<String> =
                        item.photos.iter().filter_map(|p| p.url.clone()).collect();

                    YmReturnLine {
                        item_id: item.market_sku.unwrap_or(0),
                        shop_sku: item.shop_sku.clone().unwrap_or_default(),
                        offer_id: item.offer_id.clone().unwrap_or_default(),
                        name: item.offer_name.clone().unwrap_or_default(),
                        count: item.count,
                        price: item.price,
                        return_reason: item.return_reason.clone(),
                        decisions,
                        photos,
                    }
                })
                .collect();

            // Skip returns with no items
            if lines.is_empty() {
                tracing::warn!("Return {} has no items, skipping", return_id_str);
                continue;
            }

            // Parse dates
            let created_at_source = return_item
                .created_at
                .as_ref()
                .and_then(|s| parse_ym_date(s));
            let updated_at_source = return_item
                .updated_at
                .as_ref()
                .and_then(|s| parse_ym_date(s));

            // Create aggregate
            let header = YmReturnHeader {
                return_id,
                order_id: return_item.order_id,
                connection_id: connection.base.id.as_string(),
                organization_id: organization_id.clone(),
                marketplace_id: connection.marketplace_id.clone(),
                campaign_id: connection
                    .supplier_id
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                return_type: return_item.return_type.clone().unwrap_or_default(),
                amount: return_item.amount.as_ref().and_then(|a| a.value),
                currency: return_item
                    .amount
                    .as_ref()
                    .and_then(|a| a.currency_id.clone()),
            };

            let state = YmReturnState {
                refund_status: return_item.refund_status.clone().unwrap_or_default(),
                created_at_source,
                updated_at_source,
                refund_date: None, // API doesn't provide this directly
            };

            let source_meta = YmReturnSourceMeta {
                raw_payload_ref: String::new(), // Will be filled by service
                fetched_at: chrono::Utc::now(),
                document_version: 1,
            };

            let return_type_display = return_item.return_type.clone().unwrap_or_default();
            let document = YmReturn::new_for_insert(
                return_id_str.clone(),
                format!(
                    "YM {} {} (Order {})",
                    if return_type_display == "UNREDEEMED" {
                        "Невыкуп"
                    } else {
                        "Возврат"
                    },
                    return_id_str,
                    return_item.order_id
                ),
                header,
                lines,
                state,
                source_meta,
                true, // is_posted = true при загрузке через API
            );

            // Save with raw JSON
            let raw_json = serde_json::to_string(&return_item)?;
            match a016_ym_returns::service::store_document_with_raw(document, &raw_json).await {
                Ok(_) => {
                    total_processed += 1;
                    if is_new {
                        total_inserted += 1;
                    } else {
                        total_updated += 1;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to process YM return {}: {}", return_id_str, e);
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        format!("Failed to process return {}", return_id_str),
                        Some(e.to_string()),
                    );
                }
            }

            self.progress_tracker.update_aggregate(
                session_id,
                aggregate_index,
                total_processed,
                None,
                total_inserted,
                total_updated,
            );
        }

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);
        tracing::info!(
            "YM returns import completed: processed={}, inserted={}, updated={}",
            total_processed,
            total_inserted,
            total_updated
        );

        Ok(())
    }
}

/// Parse Yandex Market date (supports multiple formats)
fn parse_ym_date(date_str: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    // Try RFC3339 first (e.g., "2024-01-15T10:30:00Z")
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
        return Some(dt.with_timezone(&chrono::Utc));
    }

    // Try format "DD-MM-YYYY HH:MM:SS" (Yandex Market format with time)
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(date_str, "%d-%m-%Y %H:%M:%S") {
        return Some(chrono::DateTime::from_naive_utc_and_offset(
            naive,
            chrono::Utc,
        ));
    }

    // Try format "DD-MM-YYYY" (Yandex Market format without time)
    if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(date_str, "%d-%m-%Y") {
        let naive_datetime = naive_date.and_hms_opt(0, 0, 0)?;
        return Some(chrono::DateTime::from_naive_utc_and_offset(
            naive_datetime,
            chrono::Utc,
        ));
    }

    // Try format "YYYY-MM-DD HH:MM:SS"
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
        return Some(chrono::DateTime::from_naive_utc_and_offset(
            naive,
            chrono::Utc,
        ));
    }

    // Try format "YYYY-MM-DD"
    if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        let naive_datetime = naive_date.and_hms_opt(0, 0, 0)?;
        return Some(chrono::DateTime::from_naive_utc_and_offset(
            naive_datetime,
            chrono::Utc,
        ));
    }

    tracing::warn!("Failed to parse YM date: {}", date_str);
    None
}

/// Normalize Yandex Market order status
fn normalize_ym_status(status: &str) -> String {
    match status.to_uppercase().as_str() {
        "DELIVERED" | "PICKUP" => "DELIVERED".to_string(),
        "CANCELLED" | "CANCELLED_IN_DELIVERY" | "CANCELLED_BEFORE_PROCESSING" => {
            "CANCELLED".to_string()
        }
        "PROCESSING" | "PENDING" | "RESERVATION" => "PROCESSING".to_string(),
        "DELIVERY" => "IN_DELIVERY".to_string(),
        "" => "UNKNOWN".to_string(),
        other => other.to_uppercase(),
    }
}

impl Clone for ImportExecutor {
    fn clone(&self) -> Self {
        Self {
            api_client: Arc::clone(&self.api_client),
            progress_tracker: Arc::clone(&self.progress_tracker),
        }
    }
}
