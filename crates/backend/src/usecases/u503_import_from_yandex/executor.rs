use super::{progress_tracker::ProgressTracker, yandex_api_client::YandexApiClient};
use crate::domain::a007_marketplace_product;
use anyhow::Result;
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

            // Группируем offer_id для batch запроса к offer-cards
            let offer_ids: Vec<String> = entries.iter().map(|e| e.offer.offer_id.clone()).collect();

            // Получаем детальную информацию
            let info_response = self
                .api_client
                .fetch_product_info(connection, offer_ids)
                .await?;

            // Обрабатываем каждый товар
            for offer_card in info_response.result.offer_cards {
                let product_name = offer_card
                    .mapping
                    .as_ref()
                    .and_then(|m| m.market_sku_name.clone())
                    .unwrap_or_else(|| "Без названия".to_string());

                let display_name = format!("{} - {}", offer_card.offer_id, product_name);

                self.progress_tracker.set_current_item(
                    session_id,
                    aggregate_index,
                    Some(display_name),
                );

                match self.process_product(connection, &offer_card).await {
                    Ok(is_new) => {
                        total_processed += 1;
                        if is_new {
                            total_inserted += 1;
                        } else {
                            total_updated += 1;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to process product {}: {}", offer_card.offer_id, e);
                        self.progress_tracker.add_error(
                            session_id,
                            Some(aggregate_index.to_string()),
                            format!("Failed to process product {}", offer_card.offer_id),
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

    /// Обработать один товар (upsert)
    async fn process_product(
        &self,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        product: &super::yandex_api_client::YandexOfferCard,
    ) -> Result<bool> {
        use contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct;

        // Используем offer_id как marketplace_sku
        let marketplace_sku = product.offer_id.clone();
        let existing = a007_marketplace_product::repository::get_by_marketplace_sku(
            &connection.marketplace_id,
            &marketplace_sku,
        )
        .await?;

        // Парсим цену
        let price = product.price.as_ref().map(|p| p.value);

        // Берем первый barcode из списка
        let barcode = product.barcodes.first().cloned();

        // Получаем category_id и category_name из mapping
        let (category_id, category_name) = product
            .mapping
            .as_ref()
            .and_then(|m| {
                m.market_category_id
                    .map(|id| (Some(id.to_string()), m.market_category_name.clone()))
            })
            .unwrap_or((None, None));

        // Получаем название товара из mapping.marketSkuName
        let product_name = product
            .mapping
            .as_ref()
            .and_then(|m| m.market_sku_name.clone())
            .unwrap_or_else(|| "Без названия".to_string());

        if let Some(mut existing_product) = existing {
            // Обновляем существующий товар
            tracing::debug!("Updating existing product: {}", marketplace_sku);

            existing_product.base.code = product.offer_id.clone();
            existing_product.base.description = product_name.clone();
            existing_product.marketplace_sku = marketplace_sku;
            existing_product.barcode = barcode.clone();
            existing_product.art = product.offer_id.clone();
            existing_product.product_name = product_name.clone();
            existing_product.brand = product.vendor.clone();
            existing_product.category_id = category_id;
            existing_product.category_name = category_name;
            existing_product.price = price;
            existing_product.last_update = Some(chrono::Utc::now());
            existing_product.before_write();

            a007_marketplace_product::repository::update(&existing_product).await?;
            Ok(false)
        } else {
            // Создаем новый товар
            tracing::debug!("Inserting new product: {}", marketplace_sku);

            let new_product = MarketplaceProduct::new_for_insert(
                product.offer_id.clone(),
                product_name.clone(),
                connection.marketplace_id.clone(),
                marketplace_sku,
                barcode,
                product.offer_id.clone(),
                product_name,
                product.vendor.clone(),
                category_id,
                category_name,
                price,
                None, // stock - не доступен в базовом API
                Some(chrono::Utc::now()),
                None, // marketplace_url
                None, // nomenclature_id
                None, // comment
            );

            a007_marketplace_product::repository::insert(&new_product).await?;
            Ok(true)
        }
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
