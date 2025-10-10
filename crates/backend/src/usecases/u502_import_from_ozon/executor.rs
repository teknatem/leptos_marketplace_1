use super::{ozon_api_client::OzonApiClient, progress_tracker::ProgressTracker};
use crate::domain::a007_marketplace_product;
use anyhow::Result;
use contracts::usecases::u502_import_from_ozon::{
    progress::ImportStatus,
    request::ImportRequest,
    response::{ImportResponse, ImportStartStatus},
};
use std::sync::Arc;
use uuid::Uuid;

/// Executor для UseCase импорта из OZON
pub struct ImportExecutor {
    api_client: Arc<OzonApiClient>,
    progress_tracker: Arc<ProgressTracker>,
}

impl ImportExecutor {
    pub fn new(progress_tracker: Arc<ProgressTracker>) -> Self {
        Self {
            api_client: Arc::new(OzonApiClient::new()),
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
    ) -> Option<contracts::usecases::u502_import_from_ozon::progress::ImportProgress> {
        self.progress_tracker.get_progress(session_id)
    }

    /// Выполнить импорт
    async fn run_import(
        &self,
        session_id: &str,
        request: &ImportRequest,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
    ) -> Result<()> {
        tracing::info!("Starting OZON import for session: {}", session_id);

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

    /// Импорт товаров из OZON
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
        let mut last_id: Option<String> = None;

        // Получаем товары страницами через /v3/product/list
        loop {
            let list_response = self
                .api_client
                .fetch_product_list(connection, page_size, last_id.clone())
                .await?;

            let items = list_response.result.items;
            if items.is_empty() {
                break;
            }

            let batch_size = items.len();
            tracing::info!(
                "Processing batch: {} items, total so far: {}",
                batch_size,
                total_processed
            );

            // Группируем product_id для batch запроса к /v3/product/info
            let product_ids: Vec<i64> = items.iter().map(|item| item.product_id).collect();

            // Получаем детальную информацию
            let info_response = self
                .api_client
                .fetch_product_info(connection, product_ids)
                .await?;

            // Обрабатываем каждый товар
            for product_info in info_response.items {
                let display_name = format!(
                    "{} - {}",
                    product_info.offer_id,
                    product_info.name
                );

                self.progress_tracker.set_current_item(
                    session_id,
                    aggregate_index,
                    Some(display_name),
                );

                match self.process_product(connection, &product_info).await {
                    Ok(is_new) => {
                        total_processed += 1;
                        if is_new {
                            total_inserted += 1;
                        } else {
                            total_updated += 1;
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to process product {}: {}",
                            product_info.offer_id,
                            e
                        );
                        self.progress_tracker.add_error(
                            session_id,
                            Some(aggregate_index.to_string()),
                            format!("Failed to process product {}", product_info.offer_id),
                            Some(e.to_string()),
                        );
                    }
                }

                // Обновить прогресс
                self.progress_tracker.update_aggregate(
                    session_id,
                    aggregate_index,
                    total_processed,
                    Some(list_response.result.total),
                    total_inserted,
                    total_updated,
                );
            }

            // Очистить текущий элемент после страницы
            self.progress_tracker
                .set_current_item(session_id, aggregate_index, None);

            // Обновляем last_id для следующей страницы
            let old_last_id = last_id.clone();
            last_id = Some(list_response.result.last_id.clone());

            // Защита от зацикливания: если last_id не изменился, прекращаем
            if old_last_id.is_some() && old_last_id == last_id {
                tracing::warn!("last_id did not change, stopping to prevent infinite loop. last_id: {:?}", last_id);
                self.progress_tracker.add_error(
                    session_id,
                    Some(aggregate_index.to_string()),
                    "Pagination stopped".to_string(),
                    Some("API returned the same last_id, possible API issue".to_string()),
                );
                break;
            }

            // Если получили меньше page_size, значит это последняя страница
            if batch_size < page_size as usize {
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
        product: &super::ozon_api_client::OzonProductInfo,
    ) -> Result<bool> {
        use contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct;

        // Проверяем, существует ли товар по marketplace_sku (product_id)
        let marketplace_sku = product.id.to_string();
        let existing = a007_marketplace_product::repository::get_by_marketplace_sku(
            &connection.marketplace_id,
            &marketplace_sku,
        )
        .await?;

        // Парсим цену
        let price = product.price.parse::<f64>().ok();

        // Берем первый barcode из списка
        let barcode = product.barcodes.first().cloned();

        // Получаем остатки
        let stock = product
            .stocks
            .as_ref()
            .and_then(|s| s.stocks.first())
            .map(|item| item.present)
            .unwrap_or(0);

        // Получаем category_id
        let category_id = product.description_category_id.map(|id| id.to_string());

        if let Some(mut existing_product) = existing {
            // Обновляем существующий товар
            tracing::debug!("Updating existing product: {}", marketplace_sku);

            existing_product.base.code = product.offer_id.clone();
            existing_product.base.description = product.name.clone();
            existing_product.marketplace_sku = marketplace_sku;
            existing_product.barcode = barcode.clone();
            existing_product.art = product.offer_id.clone();
            existing_product.product_name = product.name.clone();
            existing_product.category_id = category_id.clone();
            existing_product.price = price;
            existing_product.stock = Some(stock);
            existing_product.last_update = Some(chrono::Utc::now());
            existing_product.before_write();

            a007_marketplace_product::repository::update(&existing_product).await?;
            Ok(false)
        } else {
            // Создаем новый товар
            tracing::debug!("Inserting new product: {}", marketplace_sku);

            let new_product = MarketplaceProduct::new_for_insert(
                product.offer_id.clone(),
                product.name.clone(),
                connection.marketplace_id.clone(),
                marketplace_sku,
                barcode,
                product.offer_id.clone(),
                product.name.clone(),
                None, // brand
                category_id,
                None, // category_name
                price,
                Some(stock),
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
