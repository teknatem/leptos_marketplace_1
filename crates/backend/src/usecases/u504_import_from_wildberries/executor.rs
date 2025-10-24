use super::{progress_tracker::ProgressTracker, wildberries_api_client::WildberriesApiClient};
use crate::domain::a007_marketplace_product;
use anyhow::Result;
use contracts::domain::common::AggregateId;
use contracts::usecases::u504_import_from_wildberries::{
    progress::ImportStatus,
    request::ImportRequest,
    response::{ImportResponse, ImportStartStatus},
};
use std::sync::Arc;
use uuid::Uuid;

/// Executor для UseCase импорта из Wildberries
pub struct ImportExecutor {
    api_client: Arc<WildberriesApiClient>,
    progress_tracker: Arc<ProgressTracker>,
}

impl ImportExecutor {
    pub fn new(progress_tracker: Arc<ProgressTracker>) -> Self {
        Self {
            api_client: Arc::new(WildberriesApiClient::new()),
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
    ) -> Option<contracts::usecases::u504_import_from_wildberries::progress::ImportProgress> {
        self.progress_tracker.get_progress(session_id)
    }

    /// Выполнить импорт
    async fn run_import(
        &self,
        session_id: &str,
        request: &ImportRequest,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
    ) -> Result<()> {
        tracing::info!("Starting Wildberries import for session: {}", session_id);

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

    /// Импорт товаров из Wildberries
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
        let mut cursor: Option<super::wildberries_api_client::WildberriesCursor> = None;
        let mut expected_total: Option<i32> = None;

        tracing::info!(
            "╔═══════════════════════════════════════════════════════════"
        );
        tracing::info!(
            "║ WILDBERRIES IMPORT DIAGNOSTICS"
        );
        tracing::info!(
            "║ Connection: {} ({})",
            connection.base.description,
            connection.marketplace_id
        );
        tracing::info!(
            "╚═══════════════════════════════════════════════════════════"
        );

        // ═══════════════════════════════════════════════════════════════
        // ДИАГНОСТИЧЕСКИЙ РЕЖИМ: Тестируем различные варианты запросов
        // ═══════════════════════════════════════════════════════════════
        tracing::info!("┌─────────────────────────────────────────────────────────");
        tracing::info!("│ 🔬 RUNNING API DIAGNOSTICS");
        tracing::info!("│ Testing different API request variations...");
        tracing::info!("└─────────────────────────────────────────────────────────");

        match self.api_client.diagnostic_fetch_all_variations(connection).await {
            Ok(results) => {
                tracing::info!("┌─────────────────────────────────────────────────────────");
                tracing::info!("│ 📊 DIAGNOSTIC RESULTS:");
                for (idx, result) in results.iter().enumerate() {
                    tracing::info!("│");
                    tracing::info!("│ Test #{}: {}", idx + 1, result.test_name);
                    if result.success {
                        tracing::info!("│   ✓ SUCCESS");
                        tracing::info!("│   Items returned: {}", result.total_returned);
                        tracing::info!("│   Cursor total: {}", result.cursor_total);
                        if result.cursor_total != result.total_returned as i32 {
                            tracing::warn!(
                                "│   ⚠️  MISMATCH: cursor.total ({}) != items.length ({})",
                                result.cursor_total,
                                result.total_returned
                            );
                        }
                    } else {
                        tracing::error!("│   ✗ FAILED");
                        if let Some(ref error) = result.error {
                            tracing::error!("│   Error: {}", error);
                        }
                    }
                }
                tracing::info!("└─────────────────────────────────────────────────────────");

                // Анализ результатов
                let best_result = results.iter()
                    .filter(|r| r.success)
                    .max_by_key(|r| r.cursor_total);

                if let Some(best) = best_result {
                    if best.cursor_total > 100 {
                        tracing::warn!("┌─────────────────────────────────────────────────────────");
                        tracing::warn!("│ 🔍 IMPORTANT FINDING:");
                        tracing::warn!("│ Test '{}' returned cursor.total={}", best.test_name, best.cursor_total);
                        tracing::warn!("│ This suggests there ARE more products available!");
                        tracing::warn!("│ Current implementation might be using wrong parameters.");
                        tracing::warn!("└─────────────────────────────────────────────────────────");
                    } else if best.cursor_total <= 20 {
                        tracing::info!("┌─────────────────────────────────────────────────────────");
                        tracing::info!("│ 📌 CONCLUSION:");
                        tracing::info!("│ All tests return similar low counts ({})", best.cursor_total);
                        tracing::info!("│ This suggests:");
                        tracing::info!("│   1. These might be ALL products in this account, OR");
                        tracing::info!("│   2. Products have different status (archived, etc.), OR");
                        tracing::info!("│   3. API key has limited scope/permissions");
                        tracing::info!("│");
                        tracing::info!("│ ⚠️  RECOMMENDATION: Check Wildberries personal account");
                        tracing::info!("│ to verify actual product count and their statuses.");
                        tracing::info!("└─────────────────────────────────────────────────────────");
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to run diagnostics: {}", e);
            }
        }

        tracing::info!("┌─────────────────────────────────────────────────────────");
        tracing::info!("│ 📦 PROCEEDING WITH NORMAL IMPORT");
        tracing::info!("└─────────────────────────────────────────────────────────");

        // Получаем товары страницами через Wildberries API
        loop {
            let cursor_info = cursor.as_ref().map(|c| {
                format!(
                    "nmID={:?}, updatedAt={}, total={}",
                    c.nm_id,
                    c.updated_at.as_ref().map(|s| s.as_str()).unwrap_or("none"),
                    c.total
                )
            }).unwrap_or_else(|| "INITIAL REQUEST (no cursor)".to_string());

            tracing::info!(
                "┌─────────────────────────────────────────────────────────"
            );
            tracing::info!(
                "│ Page Request #{} | Cursor: {}",
                (total_processed / page_size) + 1,
                cursor_info
            );

            let list_response = self
                .api_client
                .fetch_product_list(connection, page_size, cursor.clone())
                .await?;

            // Логируем информацию о полученном курсоре
            tracing::info!(
                "│ Cursor in response: nmID={:?}, updatedAt={}, total={}",
                list_response.cursor.nm_id,
                list_response.cursor.updated_at.as_ref().map(|s| s.as_str()).unwrap_or("none"),
                list_response.cursor.total
            );

            // Если API вернул total, сохраняем его (только при первом запросе)
            if expected_total.is_none() && list_response.cursor.total > 0 {
                expected_total = Some(list_response.cursor.total as i32);
                tracing::info!(
                    "│ ✓ API returned TOTAL count: {} products",
                    list_response.cursor.total
                );
            }

            let cards = list_response.cards;
            let batch_size = cards.len();
            
            tracing::info!(
                "│ Response: {} items received | Total so far: {}/{}",
                batch_size,
                total_processed,
                expected_total.map(|t| t.to_string()).unwrap_or_else(|| "?".to_string())
            );

            if cards.is_empty() {
                tracing::info!(
                    "│ ⚠ Empty batch received - stopping pagination"
                );
                tracing::info!(
                    "└─────────────────────────────────────────────────────────"
                );
                break;
            }

            // Обрабатываем каждый товар
            for card in cards {
                let product_name = card.title.clone().unwrap_or_else(|| "Без названия".to_string());
                let display_name = format!("{} - {}", card.nm_id, product_name);

                self.progress_tracker.set_current_item(
                    session_id,
                    aggregate_index,
                    Some(display_name),
                );

                match self.process_product(connection, &card).await {
                    Ok(is_new) => {
                        total_processed += 1;
                        if is_new {
                            total_inserted += 1;
                        } else {
                            total_updated += 1;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to process product {}: {}", card.nm_id, e);
                        self.progress_tracker.add_error(
                            session_id,
                            Some(aggregate_index.to_string()),
                            format!("Failed to process product {}", card.nm_id),
                            Some(e.to_string()),
                        );
                    }
                }

                // Обновить прогресс
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

            tracing::info!(
                "│ ✓ Batch processed: {} inserted, {} updated",
                total_inserted,
                total_updated
            );

            // Определяем, есть ли еще страницы
            // API Wildberries возвращает cursor с информацией о следующей странице
            // Если мы получили все товары, то нужно остановиться
            let next_cursor = if total_processed >= expected_total.unwrap_or(i32::MAX) {
                tracing::info!(
                    "│ → All products received ({}/{}), no next page needed",
                    total_processed,
                    expected_total.unwrap_or(0)
                );
                None
            } else if batch_size < page_size as usize {
                // Если получили меньше чем запрашивали, значит это последняя страница
                tracing::info!(
                    "│ → Received {} items (less than page_size {}), last page",
                    batch_size,
                    page_size
                );
                None
            } else {
                // Есть еще страницы, используем курсор из ответа
                tracing::info!(
                    "│ → More pages available, using cursor from response"
                );
                Some(list_response.cursor.clone())
            };

            tracing::info!(
                "└─────────────────────────────────────────────────────────"
            );

            // Обновляем курсор для следующей страницы
            cursor = next_cursor.clone();

            // Если нет next_cursor, значит это последняя страница
            if cursor.is_none() {
                tracing::info!(
                    "┌─────────────────────────────────────────────────────────"
                );
                tracing::info!(
                    "│ ✓ PAGINATION COMPLETE: No more cursor"
                );
                tracing::info!(
                    "│   Total products: {}/{}",
                    total_processed,
                    expected_total.map(|t| t.to_string()).unwrap_or_else(|| "?".to_string())
                );
                tracing::info!(
                    "└─────────────────────────────────────────────────────────"
                );
                break;
            }

            // Защита от зацикливания
            if total_processed >= expected_total.unwrap_or(i32::MAX) {
                tracing::info!(
                    "┌─────────────────────────────────────────────────────────"
                );
                tracing::info!(
                    "│ ✓ PAGINATION COMPLETE: Reached expected total"
                );
                tracing::info!(
                    "│   Processed: {} | Expected: {}",
                    total_processed,
                    expected_total.unwrap_or(0)
                );
                tracing::info!(
                    "└─────────────────────────────────────────────────────────"
                );
                break;
            }

            // Логируем информацию о следующей странице
            if let Some(ref c) = cursor {
                tracing::info!(
                    "│ → Next cursor: nmID={:?}, updatedAt={}",
                    c.nm_id,
                    c.updated_at.as_ref().map(|s| s.as_str()).unwrap_or("none")
                );
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
        card: &super::wildberries_api_client::WildberriesCard,
    ) -> Result<bool> {
        use contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct;

        // Используем nm_id как marketplace_sku
        let marketplace_sku = card.nm_id.to_string();
        let existing = a007_marketplace_product::repository::get_by_marketplace_sku(
            &connection.marketplace_id,
            &marketplace_sku,
        )
        .await?;

        // Парсим цену из первого size (если есть)
        let price = card
            .sizes
            .first()
            .and_then(|s| s.price.map(|p| p as f64));

        // Берем первый barcode из списка sizes
        let barcode = card
            .sizes
            .first()
            .and_then(|s| s.barcode.clone());

        // Получаем название товара
        let product_name = card.title.clone().unwrap_or_else(|| "Без названия".to_string());

        if let Some(mut existing_product) = existing {
            // Обновляем существующий товар
            tracing::debug!("Updating existing product: {}", marketplace_sku);

            existing_product.base.code = card.vendor_code.clone();
            existing_product.base.description = product_name.clone();
            existing_product.marketplace_sku = marketplace_sku;
            existing_product.barcode = barcode.clone();
            existing_product.art = card.vendor_code.clone();
            existing_product.product_name = product_name.clone();
            existing_product.brand = card.brand.clone();
            existing_product.category_id = Some(card.subject_id.to_string());
            existing_product.category_name = None; // WB API не возвращает название категории
            existing_product.price = price;
            existing_product.last_update = Some(chrono::Utc::now());
            existing_product.before_write();

            a007_marketplace_product::repository::update(&existing_product).await?;
            Ok(false)
        } else {
            // Создаем новый товар
            tracing::debug!("Inserting new product: {}", marketplace_sku);

            let new_product = MarketplaceProduct::new_for_insert(
                card.vendor_code.clone(),
                product_name.clone(),
                connection.marketplace_id.clone(),
                connection.base.id.as_string(),
                marketplace_sku,
                barcode,
                card.vendor_code.clone(),
                product_name,
                card.brand.clone(),
                Some(card.subject_id.to_string()),
                None, // category_name
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
