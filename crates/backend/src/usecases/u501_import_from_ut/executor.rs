use super::{
    odata_models_counterparty::*, odata_models_nomenclature::*, odata_models_organization::*,
    progress_tracker::ProgressTracker, ut_odata_client::UtODataClient,
};
use crate::domain::{a001_connection_1c, a002_organization, a003_counterparty, a004_nomenclature};
use anyhow::Result;
use contracts::domain::common::AggregateId;
use contracts::usecases::u501_import_from_ut::{
    progress::ImportStatus,
    request::ImportRequest,
    response::{ImportResponse, ImportStartStatus},
};
use std::sync::Arc;
use uuid::Uuid;

/// Executor для UseCase импорта из УТ 11
pub struct ImportExecutor {
    odata_client: Arc<UtODataClient>,
    pub progress_tracker: Arc<ProgressTracker>,
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
                "a003_counterparty" => "Контрагенты",
                "a004_nomenclature" => "Номенклатура",
                "p901_barcodes" => "Штрихкоды номенклатуры",
                "p906_prices" => "Цены номенклатуры",
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
                .execute_import(&session_id_clone, &request_clone, &connection_clone)
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
    ) -> Option<contracts::usecases::u501_import_from_ut::progress::ImportProgress> {
        self.progress_tracker.get_progress(session_id)
    }

    /// Выполнить импорт
    pub async fn execute_import(
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
                "a003_counterparty" => {
                    self.import_counterparties(session_id, connection).await?;
                }
                "a004_nomenclature" => {
                    self.import_nomenclature(session_id, connection, request.delete_obsolete)
                        .await?;
                }
                "p901_barcodes" => {
                    self.import_barcodes(session_id, connection).await?;
                }
                "p906_prices" => {
                    // Получить период из запроса
                    let period_from = request
                        .period_from
                        .clone()
                        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
                    let period_to = request
                        .period_to
                        .clone()
                        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
                    self.import_nomenclature_prices(
                        session_id,
                        connection,
                        &period_from,
                        &period_to,
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

    /// Импорт организаций из УТ
    async fn import_organizations(
        &self,
        session_id: &str,
        connection: &contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase,
    ) -> Result<()> {
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
                .fetch_collection(
                    connection,
                    "Catalog_Организации",
                    Some(page_size),
                    Some(skip),
                )
                .await?;

            if response.value.is_empty() {
                break;
            }

            let value_len = response.value.len();

            // Обработать каждую организацию
            for odata_org in response.value {
                // Обновить текущий элемент
                self.progress_tracker.set_current_item(
                    session_id,
                    aggregate_index,
                    Some(format!("{} - {}", odata_org.code, odata_org.description)),
                );
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

            // Очистить текущий элемент после страницы
            self.progress_tracker
                .set_current_item(session_id, aggregate_index, None);

            skip += page_size;

            // Если получили меньше page_size, значит это последняя страница
            if value_len < page_size as usize {
                break;
            }

            // Небольшая пауза между батчами для снижения нагрузки на сервер 1С
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);
        tracing::info!(
            "Organizations import completed: processed={}, inserted={}, updated={}",
            processed,
            inserted,
            updated
        );

        Ok(())
    }

    /// Импорт контрагентов из УТ - упрощенная версия
    async fn import_counterparties(
        &self,
        session_id: &str,
        connection: &contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase,
    ) -> Result<()> {
        tracing::info!("Importing counterparties for session: {}", session_id);

        let aggregate_index = "a003_counterparty";

        // Получаем общее количество элементов
        let total = self
            .odata_client
            .get_collection_count(connection, "Catalog_Контрагенты")
            .await
            .ok()
            .flatten();

        tracing::info!("Total counterparty items to import: {:?}", total);

        let page_size = 100;
        let mut total_processed = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;
        let mut skip = 0;

        // Загружаем ВСЕ элементы последовательно блоками по 100
        loop {
            let response: UtCounterpartyListResponse = self
                .odata_client
                .fetch_collection(
                    connection,
                    "Catalog_Контрагенты",
                    Some(page_size),
                    Some(skip),
                )
                .await?;

            if response.value.is_empty() {
                break;
            }

            let batch_size = response.value.len();
            tracing::info!("Processing batch: skip={}, size={}", skip, batch_size);

            // Обрабатываем ВСЕ элементы из пакета (и папки, и элементы)
            for odata_item in response.value {
                let item_type = if odata_item.is_folder {
                    "Папка"
                } else {
                    "Элемент"
                };

                self.progress_tracker.set_current_item(
                    session_id,
                    aggregate_index,
                    Some(format!(
                        "[{}] {} - {}",
                        item_type, odata_item.code, odata_item.description
                    )),
                );

                match self.process_counterparty(&odata_item).await {
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
                            "Failed to process {} {}: {}",
                            item_type,
                            odata_item.code,
                            e
                        );
                        self.progress_tracker.add_error(
                            session_id,
                            Some(aggregate_index.to_string()),
                            format!("Failed to process {} {}", item_type, odata_item.code),
                            Some(e.to_string()),
                        );
                    }
                }

                // Обновляем прогресс после каждого элемента
                self.progress_tracker.update_aggregate(
                    session_id,
                    aggregate_index,
                    total_processed,
                    total,
                    total_inserted,
                    total_updated,
                );
            }

            skip += page_size;

            // Если получили меньше элементов чем запрашивали - это последний батч
            if batch_size < page_size as usize {
                break;
            }

            // Небольшая пауза между батчами для снижения нагрузки на сервер 1С
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        // Очистить текущий элемент после завершения
        self.progress_tracker
            .set_current_item(session_id, aggregate_index, None);

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);

        tracing::info!(
            "Counterparties import completed: total_processed={}, inserted={}, updated={}",
            total_processed,
            total_inserted,
            total_updated
        );

        Ok(())
    }

    /// Импорт номенклатуры из УТ - упрощенная версия
    async fn import_nomenclature(
        &self,
        session_id: &str,
        connection: &contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase,
        delete_obsolete: bool,
    ) -> Result<()> {
        tracing::info!("Importing nomenclature for session: {}", session_id);

        let aggregate_index = "a004_nomenclature";

        // Получаем общее количество элементов
        let total = self
            .odata_client
            .get_collection_count(connection, "Catalog_Номенклатура")
            .await
            .ok()
            .flatten();

        tracing::info!("Total nomenclature items to import: {:?}", total);

        let page_size = 100;
        let mut total_processed = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;
        let mut skip = 0;
        let mut unique_ids = std::collections::HashSet::new();
        let mut empty_ref_keys = 0;
        let mut invalid_ref_keys = 0;

        // Создаем файл для записи всех элементов OData
        let odata_log_path = std::path::Path::new("target")
            .join("logs")
            .join("nomenclature_odata.csv");
        let mut odata_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&odata_log_path)?;
        use std::io::Write;
        writeln!(odata_file, "Ref_Key,Code,Description")?;

        // Загружаем ВСЕ элементы последовательно блоками по 100
        loop {
            let response: UtNomenclatureListResponse = self
                .odata_client
                .fetch_collection(
                    connection,
                    "Catalog_Номенклатура",
                    Some(page_size),
                    Some(skip),
                )
                .await?;

            if response.value.is_empty() {
                break;
            }

            let batch_size = response.value.len();
            tracing::info!(
                "📦 Nomenclature batch: skip={}, size={}, total_so_far={}",
                skip,
                batch_size,
                total_processed
            );

            let batch_start = std::time::Instant::now();

            // Обрабатываем ВСЕ элементы из пакета (и папки, и элементы)
            for odata_item in response.value {
                let item_type = if odata_item.is_folder {
                    "Папка"
                } else {
                    "Элемент"
                };

                // Записываем в CSV файл
                let ref_key = if odata_item.ref_key.is_empty() {
                    "EMPTY".to_string()
                } else {
                    odata_item.ref_key.clone()
                };
                let code = odata_item.code.replace(',', ";").replace('\n', " ");
                let description = odata_item.description.replace(',', ";").replace('\n', " ");
                writeln!(odata_file, "{},{},{}", ref_key, code, description)?;

                // Статистика по ref_key
                if odata_item.ref_key.is_empty() {
                    empty_ref_keys += 1;
                } else if let Err(_) = uuid::Uuid::parse_str(&odata_item.ref_key) {
                    invalid_ref_keys += 1;
                } else {
                    unique_ids.insert(odata_item.ref_key.clone());
                }

                self.progress_tracker.set_current_item(
                    session_id,
                    aggregate_index,
                    Some(format!(
                        "[{}] {} - {}",
                        item_type, odata_item.code, odata_item.description
                    )),
                );

                match self.process_nomenclature(&odata_item).await {
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
                            "Failed to process {} {}: {}",
                            item_type,
                            odata_item.code,
                            e
                        );
                        self.progress_tracker.add_error(
                            session_id,
                            Some(aggregate_index.to_string()),
                            format!("Failed to process {} {}", item_type, odata_item.code),
                            Some(e.to_string()),
                        );
                    }
                }

                // Обновляем прогресс после каждого элемента
                self.progress_tracker.update_aggregate(
                    session_id,
                    aggregate_index,
                    total_processed,
                    total,
                    total_inserted,
                    total_updated,
                );
            }

            let batch_elapsed = batch_start.elapsed();
            tracing::info!(
                "✅ Nomenclature batch processed in {:.2}s",
                batch_elapsed.as_secs_f64()
            );

            skip += page_size;

            // Если получили меньше элементов чем запрашивали - это последний батч
            if batch_size < page_size as usize {
                break;
            }

            // Небольшая пауза между батчами для снижения нагрузки на сервер 1С
            tracing::info!("⏸️  Pausing 1s before next batch...");
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        // Очистить текущий элемент после завершения
        self.progress_tracker
            .set_current_item(session_id, aggregate_index, None);

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);

        tracing::info!(
            "✅ Nomenclature import completed: total_processed={}, inserted={}, updated={}",
            total_processed,
            total_inserted,
            total_updated
        );
        tracing::info!(
            "🔑 UUID statistics: unique={}, empty={}, invalid={}",
            unique_ids.len(),
            empty_ref_keys,
            invalid_ref_keys
        );
        tracing::info!("📄 OData dump saved to: {}", odata_log_path.display());

        // Проверяем сколько записей реально в базе
        let db_items = match a004_nomenclature::repository::list_all().await {
            Ok(items) => {
                tracing::info!(
                    "📊 Database verification: {} items in a004_nomenclature table",
                    items.len()
                );
                items
            }
            Err(e) => {
                tracing::error!("Failed to verify database: {}", e);
                vec![]
            }
        };

        // Удаление устаревших записей (которых нет в источнике)
        if delete_obsolete && !unique_ids.is_empty() {
            tracing::info!("🗑️  Checking for obsolete records to delete...");

            // Получаем все ID из БД
            let db_ids: std::collections::HashSet<uuid::Uuid> =
                db_items.iter().map(|item| item.base.id.value()).collect();

            // Преобразуем unique_ids (String) в UUID
            let source_ids: std::collections::HashSet<uuid::Uuid> = unique_ids
                .iter()
                .filter_map(|s| uuid::Uuid::parse_str(s).ok())
                .collect();

            // Находим ID, которые есть в БД, но нет в источнике
            let obsolete_ids: Vec<uuid::Uuid> = db_ids.difference(&source_ids).copied().collect();

            if !obsolete_ids.is_empty() {
                tracing::info!(
                    "🗑️  Found {} obsolete records to delete",
                    obsolete_ids.len()
                );

                match a004_nomenclature::repository::delete_by_ids(obsolete_ids.clone()).await {
                    Ok(deleted_count) => {
                        tracing::info!("✅ Deleted {} obsolete records", deleted_count);
                    }
                    Err(e) => {
                        tracing::error!("Failed to delete obsolete records: {}", e);
                        self.progress_tracker.add_error(
                            session_id,
                            Some(aggregate_index.to_string()),
                            "Failed to delete obsolete records".to_string(),
                            Some(e.to_string()),
                        );
                    }
                }
            } else {
                tracing::info!("✅ No obsolete records found");
            }
        }

        Ok(())
    }

    /// Копировать измерения из базовой номенклатуры в производную
    async fn copy_dimensions_from_base(
        derivative: &mut contracts::domain::a004_nomenclature::aggregate::Nomenclature,
    ) -> Result<()> {
        // Проверяем что это производная номенклатура
        if !derivative.is_derivative {
            return Ok(());
        }

        // Получаем ссылку на базовую номенклатуру
        let Some(ref base_ref) = derivative.base_nomenclature_ref else {
            return Ok(());
        };

        // Парсим UUID базовой номенклатуры
        let base_uuid = match uuid::Uuid::parse_str(base_ref) {
            Ok(uuid) => uuid,
            Err(e) => {
                tracing::warn!(
                    "Invalid base_nomenclature_ref UUID: {}, error: {}",
                    base_ref,
                    e
                );
                return Ok(());
            }
        };

        // Загружаем базовую номенклатуру
        let base_nomenclature = match a004_nomenclature::repository::get_by_id(base_uuid).await? {
            Some(base) => base,
            None => {
                tracing::warn!("Base nomenclature not found: {}", base_ref);
                return Ok(());
            }
        };

        // Копируем измерения
        derivative.dim1_category = base_nomenclature.dim1_category.clone();
        derivative.dim2_line = base_nomenclature.dim2_line.clone();
        derivative.dim3_model = base_nomenclature.dim3_model.clone();
        derivative.dim4_format = base_nomenclature.dim4_format.clone();
        derivative.dim5_sink = base_nomenclature.dim5_sink.clone();
        derivative.dim6_size = base_nomenclature.dim6_size.clone();

        tracing::debug!(
            "Copied dimensions from base {} to derivative {}",
            base_ref,
            derivative.base.id.as_string()
        );

        Ok(())
    }

    async fn process_nomenclature(&self, odata: &UtNomenclatureOData) -> Result<bool> {
        use uuid::Uuid;

        let existing = if !odata.ref_key.is_empty() {
            if let Ok(uuid) = Uuid::parse_str(&odata.ref_key) {
                a004_nomenclature::repository::get_by_id(uuid).await?
            } else {
                tracing::warn!("Invalid ref_key UUID: {}", odata.ref_key);
                None
            }
        } else {
            tracing::warn!(
                "Empty ref_key for item: {} - {}",
                odata.code,
                odata.description
            );
            None
        };

        if let Some(mut existing_item) = existing {
            tracing::debug!(
                "Updating existing nomenclature: {} - {}",
                odata.ref_key,
                odata.description
            );
            // ВСЕГДА обновляем для диагностики - убрана проверка should_update
            existing_item.base.code = odata.code.clone();
            existing_item.base.description = odata.description.clone();
            existing_item.full_description = odata.full_description.clone().unwrap_or_default();
            existing_item.is_folder = odata.is_folder;
            existing_item.parent_id = odata
                .parent_key
                .as_ref()
                .and_then(|s| Uuid::parse_str(s).ok())
                .map(|u| u.to_string());
            existing_item.article = odata.article.clone().unwrap_or_default();
            existing_item.is_assembly = odata.is_assembly.unwrap_or(false);
            existing_item.base_nomenclature_ref = odata
                .base_nomenclature_key
                .as_ref()
                .and_then(|s| Uuid::parse_str(s).ok())
                .map(|u| u.to_string());
            // Используем метод compute_is_derivative
            existing_item.is_derivative = existing_item.compute_is_derivative();
            existing_item.base.metadata.is_deleted = odata.deletion_mark;

            // Копируем измерения из базовой номенклатуры, если это производная
            Self::copy_dimensions_from_base(&mut existing_item).await?;

            existing_item.before_write();

            a004_nomenclature::repository::update(&existing_item).await?;
            Ok(false)
        } else {
            tracing::debug!(
                "Inserting new nomenclature: {} - {}",
                odata.ref_key,
                odata.description
            );
            let mut new_item = odata.to_aggregate().map_err(|e| anyhow::anyhow!(e))?;

            // Копируем измерения из базовой номенклатуры, если это производная
            Self::copy_dimensions_from_base(&mut new_item).await?;

            new_item.before_write();

            match a004_nomenclature::repository::insert(&new_item).await {
                Ok(_) => {
                    tracing::debug!(
                        "Successfully inserted: {} - {}",
                        odata.ref_key,
                        odata.description
                    );
                    Ok(true)
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to insert {} - {}: {}",
                        odata.ref_key,
                        odata.description,
                        e
                    );
                    Err(e)
                }
            }
        }
    }

    async fn process_counterparty(&self, odata: &UtCounterpartyOData) -> Result<bool> {
        use uuid::Uuid;

        let existing = if !odata.ref_key.is_empty() {
            if let Ok(uuid) = Uuid::parse_str(&odata.ref_key) {
                a003_counterparty::repository::get_by_id(uuid).await?
            } else {
                None
            }
        } else {
            None
        };

        if let Some(mut existing_item) = existing {
            // ВСЕГДА обновляем для диагностики - убрана проверка should_update
            existing_item.base.code = odata.code.clone();
            existing_item.base.description = odata.description.clone();
            existing_item.is_folder = odata.is_folder;
            existing_item.parent_id = odata
                .parent_key
                .as_ref()
                .and_then(|s| Uuid::parse_str(s).ok())
                .map(|u| u.to_string());
            existing_item.inn = odata.inn.clone().unwrap_or_default();
            existing_item.kpp = odata.kpp.clone().unwrap_or_default();
            existing_item.base.metadata.is_deleted = odata.deletion_mark;
            existing_item.before_write();

            a003_counterparty::repository::update(&existing_item).await?;
            Ok(false)
        } else {
            let mut new_item = odata.to_aggregate().map_err(|e| anyhow::anyhow!(e))?;
            new_item.before_write();

            match a003_counterparty::repository::insert(&new_item).await {
                Ok(_) => Ok(true),
                Err(e) => Err(e),
            }
        }
    }

    /// Обработать одну организацию (upsert)
    async fn process_organization(&self, odata_org: &UtOrganizationOData) -> Result<bool> {
        use uuid::Uuid;

        tracing::debug!(
            "Processing organization: ref_key={}, code={}, description={}",
            odata_org.ref_key,
            odata_org.code,
            odata_org.description
        );

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

            // ВСЕГДА обновляем для диагностики - убрана проверка should_update
            tracing::info!(
                "Updating organization: ref_key={}, code={}",
                odata_org.ref_key,
                odata_org.code
            );

            existing_org.base.code = odata_org.code.clone();
            existing_org.base.description = odata_org.description.clone();
            existing_org.full_name = odata_org
                .full_name
                .clone()
                .unwrap_or_else(|| odata_org.description.clone());
            existing_org.inn = odata_org.inn.clone().unwrap_or_default();
            existing_org.kpp = odata_org.kpp.clone().unwrap_or_default();
            existing_org.base.metadata.is_deleted = odata_org.deletion_mark;
            existing_org.before_write();

            a002_organization::repository::update(&existing_org).await?;
            tracing::info!(
                "Successfully updated organization: ref_key={}, code={}",
                odata_org.ref_key,
                odata_org.code
            );
            Ok(false) // Обновление
        } else {
            tracing::info!(
                "Creating new organization: code={}, description={}",
                odata_org.code,
                odata_org.description
            );

            // Создать новую
            let mut new_org = odata_org.to_aggregate().map_err(|e| anyhow::anyhow!(e))?;

            tracing::debug!(
                "Organization aggregate created: id={}, code={}, inn={}, kpp={}",
                new_org.to_string_id(),
                new_org.base.code,
                new_org.inn,
                new_org.kpp
            );

            // Предупреждения о потенциальных проблемах (не блокируем импорт)
            if new_org.base.code.trim().is_empty() {
                tracing::warn!(
                    "Organization has empty code: ref_key={}, description={}",
                    odata_org.ref_key,
                    new_org.base.description
                );
            }
            if new_org.base.description.trim().is_empty() {
                tracing::warn!(
                    "Organization has empty description: ref_key={}",
                    odata_org.ref_key
                );
            }
            if new_org.inn.trim().is_empty() {
                tracing::warn!(
                    "Organization has empty INN: ref_key={}, description={}",
                    odata_org.ref_key,
                    new_org.base.description
                );
            }

            // Вызов before_write для обновления метаданных
            new_org.before_write();

            let result = a002_organization::repository::insert(&new_org).await;

            match result {
                Ok(uuid) => {
                    tracing::info!(
                        "Successfully inserted organization: code={}, uuid={}",
                        odata_org.code,
                        uuid
                    );
                    Ok(true) // Вставка
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to insert organization: code={}, error={}",
                        odata_org.code,
                        e
                    );
                    Err(e)
                }
            }
        }
    }

    /// Импорт штрихкодов номенклатуры из УТ
    async fn import_barcodes(
        &self,
        session_id: &str,
        connection: &contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase,
    ) -> Result<()> {
        use crate::projections::p901_nomenclature_barcodes::u501_import::UtNomenclatureBarcodeListResponse;

        tracing::info!("Importing barcodes for session: {}", session_id);

        let aggregate_index = "p901_barcodes";

        // Получаем общее количество элементов
        let total = self
            .odata_client
            .get_collection_count(connection, "InformationRegister_ШтрихкодыНоменклатуры")
            .await
            .ok()
            .flatten();

        tracing::info!("Total barcodes to import: {:?}", total);

        let page_size = 100;
        let mut total_processed = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;
        let mut skip = 0;

        // Загружаем элементы последовательно блоками по 100
        // Используем $expand для получения артикула из справочника номенклатуры
        loop {
            let response: UtNomenclatureBarcodeListResponse = self
                .odata_client
                .fetch_collection_with_options(
                    connection,
                    "InformationRegister_ШтрихкодыНоменклатуры",
                    Some(page_size),
                    Some(skip),
                    None,                 // filter
                    Some("Номенклатура"), // expand - 1С OData не поддерживает вложенный $select
                    None,                 // select
                )
                .await?;

            if response.value.is_empty() {
                break;
            }

            let batch_size = response.value.len();
            tracing::info!(
                "Processing barcodes batch: skip={}, size={}",
                skip,
                batch_size
            );

            // Обрабатываем все элементы из пакета
            for odata_item in response.value {
                self.progress_tracker.set_current_item(
                    session_id,
                    aggregate_index,
                    Some(format!(
                        "Barcode: {} -> {}",
                        odata_item.barcode, odata_item.owner_key
                    )),
                );

                match self.process_barcode(&odata_item).await {
                    Ok(is_new) => {
                        total_processed += 1;
                        if is_new {
                            total_inserted += 1;
                        } else {
                            total_updated += 1;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to process barcode {}: {}", odata_item.barcode, e);
                        self.progress_tracker.add_error(
                            session_id,
                            Some(aggregate_index.to_string()),
                            format!("Failed to process barcode {}", odata_item.barcode),
                            Some(e.to_string()),
                        );
                    }
                }

                // Обновляем прогресс после каждого элемента
                self.progress_tracker.update_aggregate(
                    session_id,
                    aggregate_index,
                    total_processed,
                    total,
                    total_inserted,
                    total_updated,
                );
            }

            skip += page_size;

            // Если получили меньше элементов чем запрашивали - это последний батч
            if batch_size < page_size as usize {
                break;
            }

            // Небольшая пауза между батчами
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        // Очистить текущий элемент после завершения
        self.progress_tracker
            .set_current_item(session_id, aggregate_index, None);

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);

        tracing::info!(
            "Barcodes import completed: total_processed={}, inserted={}, updated={}",
            total_processed,
            total_inserted,
            total_updated
        );

        Ok(())
    }

    /// Обработка одного штрихкода из OData
    async fn process_barcode(
        &self,
        odata: &crate::projections::p901_nomenclature_barcodes::u501_import::UtNomenclatureBarcodeOData,
    ) -> Result<bool> {
        use crate::projections::p901_nomenclature_barcodes::repository;

        // Проверяем существование записи по композитному ключу (barcode + source='1C')
        let existing = repository::get_by_barcode_and_source(&odata.barcode, "1C").await?;

        if let Some(existing_model) = existing {
            // Проверяем, нужно ли обновление
            if odata.should_update(&existing_model) {
                let entry = odata.to_entry().map_err(|e| anyhow::anyhow!(e))?;
                repository::upsert_entry(&entry).await?;
                tracing::debug!("Updated barcode: {} (source: 1C)", odata.barcode);
                Ok(false) // Обновление
            } else {
                // Данные не изменились, пропускаем
                Ok(false)
            }
        } else {
            // Новая запись
            let entry = odata.to_entry().map_err(|e| anyhow::anyhow!(e))?;
            repository::upsert_entry(&entry).await?;
            tracing::debug!("Inserted new barcode: {} (source: 1C)", odata.barcode);
            Ok(true) // Вставка
        }
    }

    /// Импорт цен номенклатуры из HTTP API /hs/mpi_api/prices_dealer
    async fn import_nomenclature_prices(
        &self,
        session_id: &str,
        connection: &contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase,
        _period_from: &str,
        _period_to: &str,
    ) -> Result<()> {
        use crate::projections::p906_nomenclature_prices::repository;
        use crate::projections::p906_nomenclature_prices::u501_import::PricesDealerResponse;

        tracing::info!("Importing nomenclature prices for session: {}", session_id);

        let aggregate_index = "p906_prices";

        // Формируем URL для HTTP API
        // Из URL типа "http://host:port/trade/odata/standard.odata" получаем "http://host:port/trade"
        let base_url = connection.url.trim_end_matches('/');
        let trade_base = if let Some(pos) = base_url.find("/odata") {
            &base_url[..pos]
        } else {
            base_url
        };
        let api_url = format!("{}/hs/mpi_api/prices_dealer", trade_base);

        tracing::info!("Fetching prices from: {}", api_url);

        // Очищаем таблицу перед загрузкой
        tracing::info!("Clearing all price records...");
        let deleted_count = repository::delete_all().await?;
        tracing::info!("Deleted {} old price records", deleted_count);

        // Делаем HTTP запрос
        let client = reqwest::Client::builder()
            .no_proxy() // Отключаем системный прокси для прямого подключения к 1С
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;
        let response = client
            .get(&api_url)
            .basic_auth(&connection.login, Some(&connection.password))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("HTTP error {}: {}", status, body));
        }

        let prices_data: PricesDealerResponse = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;

        tracing::info!(
            "Received {} total records (initial: {}, history: {})",
            prices_data.count,
            prices_data.initial.len(),
            prices_data.history.len()
        );

        let total_count = prices_data.initial.len() + prices_data.history.len();
        let mut total_processed = 0;
        let mut total_inserted = 0;
        let mut total_errors = 0;

        // Обрабатываем initial записи
        tracing::info!(
            "Processing {} dealer price records...",
            prices_data.initial.len()
        );
        for (i, item) in prices_data.initial.iter().enumerate() {
            // Логируем первые 3 записи
            if i < 3 {
                tracing::info!("Sample initial record {}: {}", i + 1, item.debug_info());
            }

            self.progress_tracker.set_current_item(
                session_id,
                aggregate_index,
                Some(format!("[initial] {}", item.debug_info())),
            );

            match item.to_entry() {
                Ok(entry) => match repository::insert_entry(&entry).await {
                    Ok(_) => {
                        total_processed += 1;
                        total_inserted += 1;
                    }
                    Err(e) => {
                        tracing::error!("Failed to insert price: {}", e);
                        total_errors += 1;
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to parse price item: {}", e);
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        format!("Parse error: {}", e),
                        Some(item.debug_info()),
                    );
                    total_errors += 1;
                }
            }

            // Обновляем прогресс
            self.progress_tracker.update_aggregate(
                session_id,
                aggregate_index,
                total_processed,
                Some(total_count as i32),
                total_inserted,
                0,
            );
        }

        // Обрабатываем history записи
        tracing::info!(
            "Processing {} history records...",
            prices_data.history.len()
        );
        for (i, item) in prices_data.history.iter().enumerate() {
            // Логируем первые 3 записи
            if i < 3 {
                tracing::info!("Sample history record {}: {}", i + 1, item.debug_info());
            }

            self.progress_tracker.set_current_item(
                session_id,
                aggregate_index,
                Some(format!("[history] {}", item.debug_info())),
            );

            match item.to_entry() {
                Ok(entry) => match repository::insert_entry(&entry).await {
                    Ok(_) => {
                        total_processed += 1;
                        total_inserted += 1;
                    }
                    Err(e) => {
                        tracing::error!("Failed to insert price: {}", e);
                        total_errors += 1;
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to parse price item: {}", e);
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        format!("Parse error: {}", e),
                        Some(item.debug_info()),
                    );
                    total_errors += 1;
                }
            }

            // Обновляем прогресс
            self.progress_tracker.update_aggregate(
                session_id,
                aggregate_index,
                total_processed,
                Some(total_count as i32),
                total_inserted,
                0,
            );
        }

        // Очистить текущий элемент после завершения
        self.progress_tracker
            .set_current_item(session_id, aggregate_index, None);

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);

        tracing::info!(
            "Nomenclature prices import completed: processed={}, inserted={}, errors={}",
            total_processed,
            total_inserted,
            total_errors
        );

        Ok(())
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
