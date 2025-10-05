use super::{progress_tracker::ProgressTracker, ut_odata_client::UtODataClient};
use crate::domain::{a001_connection_1c, a002_organization};
use anyhow::Result;
use contracts::usecases::u501_import_from_ut::{
    progress::ImportStatus, request::ImportRequest, response::{ImportResponse, ImportStartStatus},
};
use std::sync::Arc;
use uuid::Uuid;

/// Executor для UseCase импорта из УТ 11
pub struct ImportExecutor {
    odata_client: Arc<UtODataClient>,
    progress_tracker: Arc<ProgressTracker>,
}

impl ImportExecutor {
    pub fn new(progress_tracker: Arc<ProgressTracker>) -> Self {
        Self {
            odata_client: Arc::new(UtODataClient::new()),
            progress_tracker,
        }
    }

    /// Запустить импорт (создает async task и возвращает session_id)
    pub async fn start_import(&self, request: ImportRequest) -> Result<ImportResponse> {
        // Валидация запроса
        let connection_id = Uuid::parse_str(&request.connection_id)
            .map_err(|_| anyhow::anyhow!("Invalid connection_id"))?;

        // Получить подключение
        let connection = a001_connection_1c::service::get_by_id(connection_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;

        // Создать сессию импорта
        let session_id = Uuid::new_v4().to_string();
        self.progress_tracker.create_session(session_id.clone());

        // Добавить агрегаты для отслеживания
        for aggregate_index in &request.target_aggregates {
            let aggregate_name = match aggregate_index.as_str() {
                "a002_organization" => "Организации",
                _ => "Unknown",
            };
            self.progress_tracker
                .add_aggregate(&session_id, aggregate_index.clone(), aggregate_name.to_string());
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
    pub fn get_progress(&self, session_id: &str) -> Option<contracts::usecases::u501_import_from_ut::progress::ImportProgress> {
        self.progress_tracker.get_progress(session_id)
    }

    /// Выполнить импорт
    async fn run_import(
        &self,
        session_id: &str,
        request: &ImportRequest,
        connection: &contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase,
    ) -> Result<()> {
        tracing::info!("Starting import for session: {}", session_id);

        for aggregate_index in &request.target_aggregates {
            match aggregate_index.as_str() {
                "a002_organization" => {
                    self.import_organizations(session_id, connection).await?;
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
        let final_status = if self.progress_tracker.get_progress(session_id)
            .map(|p| p.total_errors > 0)
            .unwrap_or(false)
        {
            ImportStatus::CompletedWithErrors
        } else {
            ImportStatus::Completed
        };

        self.progress_tracker.complete_session(session_id, final_status);
        tracing::info!("Import completed for session: {}", session_id);

        Ok(())
    }

    /// Импорт организаций из УТ
    async fn import_organizations(
        &self,
        session_id: &str,
        connection: &contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase,
    ) -> Result<()> {
        use a002_organization::from_ut_odata::UtOrganizationListResponse;

        tracing::info!("Importing organizations for session: {}", session_id);

        let aggregate_index = "a002_organization";

        // Получить количество (опционально)
        let total = self
            .odata_client
            .get_collection_count(connection, "Catalog_Организации")
            .await
            .ok()
            .flatten();

        // Параметры пагинации
        let page_size = 100;
        let mut skip = 0;
        let mut processed = 0;
        let mut inserted = 0;
        let mut updated = 0;

        loop {
            // Получить страницу данных
            let response: UtOrganizationListResponse = self
                .odata_client
                .fetch_collection(connection, "Catalog_Организации", Some(page_size), Some(skip))
                .await?;

            if response.value.is_empty() {
                break;
            }

            let value_len = response.value.len();

            // Обработать каждую организацию
            for odata_org in response.value {
                match self.process_organization(&odata_org).await {
                    Ok(is_new) => {
                        processed += 1;
                        if is_new {
                            inserted += 1;
                        } else {
                            updated += 1;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to process organization {}: {}", odata_org.code, e);
                        self.progress_tracker.add_error(
                            session_id,
                            Some(aggregate_index.to_string()),
                            format!("Failed to process organization {}", odata_org.code),
                            Some(e.to_string()),
                        );
                    }
                }

                // Обновить прогресс
                self.progress_tracker.update_aggregate(
                    session_id,
                    aggregate_index,
                    processed,
                    total,
                    inserted,
                    updated,
                );
            }

            skip += page_size;

            // Если получили меньше page_size, значит это последняя страница
            if value_len < page_size as usize {
                break;
            }
        }

        self.progress_tracker.complete_aggregate(session_id, aggregate_index);
        tracing::info!("Organizations import completed: processed={}, inserted={}, updated={}", processed, inserted, updated);

        Ok(())
    }

    /// Обработать одну организацию (upsert)
    async fn process_organization(&self, odata_org: &a002_organization::from_ut_odata::UtOrganizationOData) -> Result<bool> {
        use uuid::Uuid;

        tracing::debug!("Processing organization: ref_key={}, code={}, description={}",
            odata_org.ref_key, odata_org.code, odata_org.description);

        // Попытаться найти существующую организацию по ID (Ref_Key из 1С)
        let existing = if !odata_org.ref_key.is_empty() {
            if let Ok(uuid) = Uuid::parse_str(&odata_org.ref_key) {
                a002_organization::repository::get_by_id(uuid).await?
            } else {
                None
            }
        } else {
            None
        };

        if let Some(mut existing_org) = existing {
            tracing::debug!("Found existing organization with code={}", odata_org.code);

            // Проверить, нужно ли обновление
            if odata_org.should_update(&existing_org) {
                tracing::info!("Updating organization: ref_key={}, code={}", odata_org.ref_key, odata_org.code);

                // Обновить
                existing_org.base.code = odata_org.code.clone();
                existing_org.base.description = odata_org.description.clone();
                existing_org.full_name = odata_org.full_name.clone().unwrap_or_else(|| odata_org.description.clone());
                existing_org.inn = odata_org.inn.clone().unwrap_or_default();
                existing_org.kpp = odata_org.kpp.clone().unwrap_or_default();
                existing_org.before_write();

                a002_organization::repository::update(&existing_org).await?;
                tracing::info!("Successfully updated organization: ref_key={}, code={}", odata_org.ref_key, odata_org.code);
                Ok(false) // Обновление
            } else {
                tracing::debug!("No changes needed for organization: code={}", odata_org.code);
                Ok(false) // Без изменений
            }
        } else {
            tracing::info!("Creating new organization: code={}, description={}",
                odata_org.code, odata_org.description);

            // Создать новую
            let mut new_org = odata_org.to_aggregate().map_err(|e| anyhow::anyhow!(e))?;

            tracing::debug!("Organization aggregate created: id={}, code={}, inn={}, kpp={}",
                new_org.to_string_id(), new_org.base.code, new_org.inn, new_org.kpp);

            // Предупреждения о потенциальных проблемах (не блокируем импорт)
            if new_org.base.code.trim().is_empty() {
                tracing::warn!("Organization has empty code: ref_key={}, description={}",
                    odata_org.ref_key, new_org.base.description);
            }
            if new_org.base.description.trim().is_empty() {
                tracing::warn!("Organization has empty description: ref_key={}", odata_org.ref_key);
            }
            if new_org.inn.trim().is_empty() {
                tracing::warn!("Organization has empty INN: ref_key={}, description={}",
                    odata_org.ref_key, new_org.base.description);
            }

            // Вызов before_write для обновления метаданных
            new_org.before_write();

            let result = a002_organization::repository::insert(&new_org).await;

            match result {
                Ok(uuid) => {
                    tracing::info!("Successfully inserted organization: code={}, uuid={}",
                        odata_org.code, uuid);
                    Ok(true) // Вставка
                }
                Err(e) => {
                    tracing::error!("Failed to insert organization: code={}, error={}",
                        odata_org.code, e);
                    Err(e)
                }
            }
        }
    }
}

impl Clone for ImportExecutor {
    fn clone(&self) -> Self {
        Self {
            odata_client: Arc::clone(&self.odata_client),
            progress_tracker: Arc::clone(&self.progress_tracker),
        }
    }
}
