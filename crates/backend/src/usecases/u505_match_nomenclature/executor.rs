use super::progress_tracker::ProgressTracker;
use crate::domain::{a004_nomenclature, a007_marketplace_product};
use anyhow::Result;
use contracts::domain::common::AggregateId;
use contracts::usecases::u505_match_nomenclature::{
    progress::MatchStatus,
    request::MatchRequest,
    response::{MatchResponse, MatchStartStatus},
};
use std::sync::Arc;
use uuid::Uuid;

/// Executor для UseCase сопоставления номенклатуры
pub struct MatchExecutor {
    progress_tracker: Arc<ProgressTracker>,
}

impl MatchExecutor {
    pub fn new(progress_tracker: Arc<ProgressTracker>) -> Self {
        Self { progress_tracker }
    }

    /// Запустить сопоставление (создает async task и возвращает session_id)
    pub async fn start_matching(&self, request: MatchRequest) -> Result<MatchResponse> {
        tracing::info!("Starting nomenclature matching with request: {:?}", request);

        // Создать сессию сопоставления
        let session_id = Uuid::new_v4().to_string();

        // Получить количество товаров для обработки
        let products = if let Some(marketplace_id) = &request.marketplace_id {
            a007_marketplace_product::repository::list_by_marketplace_id(marketplace_id).await?
        } else {
            a007_marketplace_product::service::list_all().await?
        };

        let total = products.len() as i32;
        self.progress_tracker
            .create_session(session_id.clone(), Some(total));

        // Запустить сопоставление в фоне
        let self_clone = Arc::new(self.clone());
        let session_id_clone = session_id.clone();
        let request_clone = request.clone();

        tokio::spawn(async move {
            if let Err(e) = self_clone
                .run_matching(&session_id_clone, &request_clone)
                .await
            {
                tracing::error!("Matching failed: {}", e);
                self_clone.progress_tracker.add_error(
                    &session_id_clone,
                    format!("Matching failed: {}", e),
                    None,
                    None,
                );
                self_clone
                    .progress_tracker
                    .complete_session(&session_id_clone, MatchStatus::Failed);
            }
        });

        Ok(MatchResponse {
            session_id,
            status: MatchStartStatus::Started,
            message: format!("Сопоставление запущено для {} товаров", total),
        })
    }

    /// Получить текущий прогресс сопоставления
    pub fn get_progress(
        &self,
        session_id: &str,
    ) -> Option<contracts::usecases::u505_match_nomenclature::progress::MatchProgress> {
        self.progress_tracker.get_progress(session_id)
    }

    /// Выполнить сопоставление
    async fn run_matching(&self, session_id: &str, request: &MatchRequest) -> Result<()> {
        tracing::info!("Running matching for session: {}", session_id);

        // Получить все товары маркетплейса
        let products = if let Some(marketplace_id) = &request.marketplace_id {
            a007_marketplace_product::repository::list_by_marketplace_id(marketplace_id).await?
        } else {
            a007_marketplace_product::service::list_all().await?
        };

        tracing::info!("Found {} products to process", products.len());

        let mut processed = 0;
        let mut matched = 0;
        let mut cleared = 0;
        let mut skipped = 0;
        let mut ambiguous = 0;

        // Обработать каждый товар
        for product in products {
            // Установить текущий товар
            self.progress_tracker.set_current_item(
                session_id,
                Some(format!("{} - {}", product.art, product.product_name)),
            );

            match self.process_product(&product, request).await {
                Ok(result) => {
                    processed += 1;
                    match result {
                        MatchResult::Matched => matched += 1,
                        MatchResult::Cleared => cleared += 1,
                        MatchResult::ClearedAmbiguous => {
                            cleared += 1;
                            ambiguous += 1;
                        }
                        MatchResult::Skipped => skipped += 1,
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to process product {}: {}", product.art, e);
                    self.progress_tracker.add_error(
                        session_id,
                        format!("Failed to process product {}", product.art),
                        Some(e.to_string()),
                        Some(product.art.clone()),
                    );
                    processed += 1;
                }
            }

            // Обновить прогресс
            self.progress_tracker.update_progress(
                session_id,
                processed,
                matched,
                cleared,
                skipped,
                ambiguous,
            );
        }

        // Очистить текущий элемент
        self.progress_tracker.set_current_item(session_id, None);

        // Завершить сессию
        let final_status = if self
            .progress_tracker
            .get_progress(session_id)
            .map(|p| p.errors > 0)
            .unwrap_or(false)
        {
            MatchStatus::CompletedWithErrors
        } else {
            MatchStatus::Completed
        };

        self.progress_tracker
            .complete_session(session_id, final_status);

        tracing::info!(
            "Matching completed for session: {}. Processed: {}, Matched: {}, Cleared: {}, Skipped: {}, Ambiguous: {}",
            session_id,
            processed,
            matched,
            cleared,
            skipped,
            ambiguous
        );

        Ok(())
    }

    /// Обработать один товар
    async fn process_product(
        &self,
        product: &contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct,
        request: &MatchRequest,
    ) -> Result<MatchResult> {
        // Проверить, нужно ли обрабатывать товар
        if !request.overwrite_existing && product.nomenclature_id.is_some() {
            tracing::debug!(
                "Skipping product {} - already has nomenclature_id and overwrite_existing=false",
                product.art
            );
            return Ok(MatchResult::Skipped);
        }

        // Найти номенклатуру по артикулу
        let article = product.art.trim();
        if article.is_empty() {
            tracing::warn!("Product {} has empty article", product.base.id.as_string());
            return self.clear_nomenclature_link(product).await;
        }

        tracing::debug!(
            "Searching for article: '{}' (ignore_case: {})",
            article,
            request.ignore_case
        );

        let found_items = if request.ignore_case {
            a004_nomenclature::repository::find_by_article_ignore_case(article).await?
        } else {
            a004_nomenclature::repository::find_by_article(article).await?
        };

        tracing::debug!(
            "Found {} nomenclature items for article '{}'",
            found_items.len(),
            article
        );

        match found_items.len() {
            0 => {
                // Не найдено совпадений - очистить связь
                tracing::debug!("No nomenclature found for article: {}", article);
                self.clear_nomenclature_link(product).await
            }
            1 => {
                // Найдено ровно 1 совпадение - установить связь
                let nomenclature = &found_items[0];
                tracing::info!(
                    "Matched product {} with nomenclature {} ({})",
                    article,
                    nomenclature.base.code,
                    nomenclature.base.description
                );
                self.set_nomenclature_link(product, nomenclature.base.id.as_string())
                    .await
            }
            _ => {
                // Найдено >1 совпадений - очистить связь
                tracing::warn!(
                    "Ambiguous match for article {}: found {} nomenclature items",
                    article,
                    found_items.len()
                );
                self.clear_nomenclature_link_ambiguous(product).await
            }
        }
    }

    /// Установить связь с номенклатурой
    async fn set_nomenclature_link(
        &self,
        product: &contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct,
        nomenclature_id: String,
    ) -> Result<MatchResult> {
        let mut product_clone = product.clone();
        product_clone.nomenclature_id = Some(nomenclature_id);
        product_clone.before_write();

        a007_marketplace_product::repository::update(&product_clone).await?;
        Ok(MatchResult::Matched)
    }

    /// Очистить связь с номенклатурой (не найдено совпадений)
    async fn clear_nomenclature_link(
        &self,
        product: &contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct,
    ) -> Result<MatchResult> {
        if product.nomenclature_id.is_none() {
            // Уже пусто - не нужно обновлять
            return Ok(MatchResult::Cleared);
        }

        let mut product_clone = product.clone();
        product_clone.nomenclature_id = None;
        product_clone.before_write();

        a007_marketplace_product::repository::update(&product_clone).await?;
        Ok(MatchResult::Cleared)
    }

    /// Очистить связь с номенклатурой (неоднозначное сопоставление)
    async fn clear_nomenclature_link_ambiguous(
        &self,
        product: &contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct,
    ) -> Result<MatchResult> {
        if product.nomenclature_id.is_none() {
            // Уже пусто - не нужно обновлять
            return Ok(MatchResult::ClearedAmbiguous);
        }

        let mut product_clone = product.clone();
        product_clone.nomenclature_id = None;
        product_clone.before_write();

        a007_marketplace_product::repository::update(&product_clone).await?;
        Ok(MatchResult::ClearedAmbiguous)
    }
}

impl Clone for MatchExecutor {
    fn clone(&self) -> Self {
        Self {
            progress_tracker: Arc::clone(&self.progress_tracker),
        }
    }
}

/// Результат обработки одного товара
enum MatchResult {
    /// Успешно сопоставлен
    Matched,
    /// Связь очищена (не найдено совпадений)
    Cleared,
    /// Связь очищена (неоднозначное сопоставление)
    ClearedAmbiguous,
    /// Пропущен
    Skipped,
}
