use super::{
    progress_tracker::ProgressTracker,
    processors::{order, payment_report, product, returns},
    yandex_api_client::YandexApiClient,
};
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
    pub progress_tracker: Arc<ProgressTracker>,
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
                "p907_ym_payment_report" => "Отчёт по платежам YM",
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
    ) -> Option<contracts::usecases::u503_import_from_yandex::progress::ImportProgress> {
        self.progress_tracker.get_progress(session_id)
    }

    /// Выполнить импорт
    pub async fn execute_import(
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
                    self.import_ym_orders(
                        session_id,
                        connection,
                        request.date_from,
                        request.date_to,
                    )
                    .await?;
                }
                "a016_ym_returns" => {
                    self.import_ym_returns(
                        session_id,
                        connection,
                        request.date_from,
                        request.date_to,
                    )
                    .await?;
                }
                "p907_ym_payment_report" => {
                    self.import_ym_payment_report(
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

        // Получаем товары страницами через /campaigns/{campaignId}/offer-mappings
        loop {
            let list_response = self
                .api_client
                .fetch_product_list(connection, page_size, page_token.clone())
                .await?;

            // Если API вернул total, сохраняем его (только при первом запросе)
            if expected_total.is_none() {
                expected_total = Some(list_response.result.paging.total.unwrap_or(0) as i32);
            }

            let offers = list_response.result.offer_mapping_entries;
            let next_page_token = list_response.result.paging.next_page_token;

            if offers.is_empty() {
                break;
            }

            let batch_size = offers.len();
            tracing::info!(
                "Processing batch: {} items, total so far: {}",
                batch_size,
                total_processed
            );

            // Обрабатываем каждый товар
            for offer_mapping in offers {
                let offer = &offer_mapping.offer;
                let mapping = &offer_mapping.mapping;

                let product_name = offer
                    .name
                    .clone()
                    .unwrap_or_else(|| "Без названия".to_string());
                let display_name = format!("{} - {}", offer.offer_id, product_name);

                self.progress_tracker.set_current_item(
                    session_id,
                    aggregate_index,
                    Some(display_name),
                );

                match product::process_product_from_offer(connection, offer, mapping).await {
                    Ok((is_new, barcodes_count)) => {
                        total_processed += 1;
                        if is_new {
                            total_inserted += 1;
                        } else {
                            total_updated += 1;
                        }
                        total_barcodes_imported += barcodes_count;
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

                // Обновить прогресс
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
                    total_barcodes_imported as i32,
                );
            }

            // Очистить текущий элемент после страницы
            self.progress_tracker
                .set_current_item(session_id, aggregate_index, None);

            // Обновляем page_token для следующей страницы
            let old_token = page_token.clone();
            page_token = next_page_token;

            // Если нет next_page_token, значит это последняя страница
            if page_token.is_none() {
                break;
            }

            // Защита от зацикливания: если токен не изменился, прекращаем
            if old_token.is_some() && old_token == page_token {
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

    /// Импорт заказов Yandex Market
    async fn import_ym_orders(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<()> {
        use crate::domain::a002_organization;

        tracing::info!("Importing Yandex Market orders for session: {}", session_id);

        let aggregate_index = "a013_ym_order";
        let mut total_processed = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;

        // 1. Resolve organization by UUID reference from connection
        let organization_id = match Uuid::parse_str(&connection.organization_ref) {
            Ok(org_uuid) => match a002_organization::service::get_by_id(org_uuid).await? {
                Some(org) => org.base.id.as_string(),
                None => {
                    let msg = format!(
                        "Organization UUID '{}' not found",
                        connection.organization_ref
                    );
                    tracing::error!("{}", msg);
                    anyhow::bail!("{}", msg);
                }
            },
            Err(_) => {
                let msg = format!(
                    "Invalid organization_ref UUID in connection: '{}'",
                    connection.organization_ref
                );
                tracing::error!("{}", msg);
                anyhow::bail!("{}", msg);
            }
        };

        // 2. Fetch orders from API with date period
        let orders = self
            .api_client
            .fetch_orders(connection, date_from, date_to)
            .await?;

        tracing::info!("Received {} orders from YM API", orders.len());

        // 3. Process each order
        for order_item in orders {
            let order_id_str = order_item.id.to_string();
            self.progress_tracker.set_current_item(
                session_id,
                aggregate_index,
                Some(format!("YM Order {}", order_id_str)),
            );

            // Fetch detailed order info to get realDeliveryDate
            let order_details = match self
                .api_client
                .fetch_order_details(connection, order_item.id)
                .await
            {
                Ok(details) => details,
                Err(e) => {
                    tracing::warn!(
                        "Failed to fetch details for order {}: {}, using basic data",
                        order_id_str,
                        e
                    );
                    order_item.clone() // Use original order if details fetch fails
                }
            };

            match order::process_order(connection, &organization_id, &order_details).await {
                Ok(is_new) => {
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

        tracing::info!(
            "Importing Yandex Market returns for session: {}",
            session_id
        );

        let aggregate_index = "a016_ym_returns";
        let mut total_processed = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;

        // 1. Resolve organization by UUID reference from connection
        let organization_id = match Uuid::parse_str(&connection.organization_ref) {
            Ok(org_uuid) => match a002_organization::service::get_by_id(org_uuid).await? {
                Some(org) => org.base.id.as_string(),
                None => {
                    let msg = format!(
                        "Organization UUID '{}' not found",
                        connection.organization_ref
                    );
                    tracing::error!("{}", msg);
                    anyhow::bail!("{}", msg);
                }
            },
            Err(_) => {
                let msg = format!(
                    "Invalid organization_ref UUID in connection: '{}'",
                    connection.organization_ref
                );
                tracing::error!("{}", msg);
                anyhow::bail!("{}", msg);
            }
        };

        // 2. Fetch returns from API with date period
        let returns = self
            .api_client
            .fetch_returns(connection, date_from, date_to)
            .await?;

        tracing::info!("Received {} returns from YM API", returns.len());

        // 3. Process each return
        for return_item in returns {
            let return_id_str = return_item.id.to_string();
            self.progress_tracker.set_current_item(
                session_id,
                aggregate_index,
                Some(format!("YM Return {}", return_id_str)),
            );

            match returns::process_return(connection, &organization_id, &return_item).await {
                Ok(is_new) => {
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

    /// Импорт отчёта по платежам Yandex Market (двухфазный процесс)
    ///
    /// Фаза 1: POST /v2/reports/united-netting/generate → получить reportId
    /// Фаза 2: GET /v2/reports/info/{reportId} → polling до DONE (макс. 60 попыток по 5с)
    /// Фаза 3: Скачать CSV и разобрать каждую строку в p907_ym_payment_report
    async fn import_ym_payment_report(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<()> {
        use crate::domain::a002_organization;

        tracing::info!(
            "Importing YM payment report for session: {}",
            session_id
        );

        let aggregate_index = "p907_ym_payment_report";

        // Resolve organization
        let organization_id = match Uuid::parse_str(&connection.organization_ref) {
            Ok(org_uuid) => match a002_organization::service::get_by_id(org_uuid).await? {
                Some(org) => org.base.id.as_string(),
                None => {
                    let msg = format!(
                        "Organization UUID '{}' not found",
                        connection.organization_ref
                    );
                    tracing::error!("{}", msg);
                    anyhow::bail!("{}", msg);
                }
            },
            Err(_) => {
                let msg = format!(
                    "Invalid organization_ref UUID in connection: '{}'",
                    connection.organization_ref
                );
                tracing::error!("{}", msg);
                anyhow::bail!("{}", msg);
            }
        };

        // Phase 1: request report generation
        self.progress_tracker.set_current_item(
            session_id,
            aggregate_index,
            Some("Запрос генерации отчёта...".to_string()),
        );

        let report_id = self
            .api_client
            .generate_payment_report(connection, date_from, date_to)
            .await
            .map_err(|e| {
                self.progress_tracker.add_error(
                    session_id,
                    Some(aggregate_index.to_string()),
                    format!("Ошибка запроса генерации отчёта: {}", e),
                    None,
                );
                e
            })?;

        tracing::info!("Payment report requested, reportId={}", report_id);

        // Phase 2: poll until DONE (up to 60 attempts, 5s each = max 5 minutes)
        const MAX_POLL_ATTEMPTS: u32 = 60;
        const POLL_INTERVAL_SECS: u64 = 5;

        let mut download_url: Option<String> = None;

        for attempt in 1..=MAX_POLL_ATTEMPTS {
            self.progress_tracker.set_current_item(
                session_id,
                aggregate_index,
                Some(format!(
                    "Ожидание готовности отчёта... ({}/{})",
                    attempt, MAX_POLL_ATTEMPTS
                )),
            );

            tokio::time::sleep(tokio::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;

            let (status, file_url) = self
                .api_client
                .poll_report_status(connection, &report_id)
                .await
                .map_err(|e| {
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        format!("Ошибка получения статуса отчёта: {}", e),
                        None,
                    );
                    e
                })?;

            tracing::info!(
                "Payment report status (attempt {}): {}",
                attempt,
                status
            );

            match status.as_str() {
                "DONE" => {
                    download_url = file_url;
                    break;
                }
                "FAILED" => {
                    let msg = "Генерация отчёта завершилась ошибкой на стороне YM".to_string();
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        msg.clone(),
                        None,
                    );
                    anyhow::bail!("{}", msg);
                }
                _ => {
                    // PENDING / PROCESSING — continue polling
                }
            }

            if attempt == MAX_POLL_ATTEMPTS {
                let msg = format!(
                    "Превышено время ожидания готовности отчёта ({} попыток по {}с)",
                    MAX_POLL_ATTEMPTS, POLL_INTERVAL_SECS
                );
                self.progress_tracker.add_error(
                    session_id,
                    Some(aggregate_index.to_string()),
                    msg.clone(),
                    None,
                );
                anyhow::bail!("{}", msg);
            }
        }

        let url = download_url.ok_or_else(|| {
            let msg = "Отчёт DONE, но URL файла не получен";
            self.progress_tracker.add_error(
                session_id,
                Some(aggregate_index.to_string()),
                msg.to_string(),
                None,
            );
            anyhow::anyhow!("{}", msg)
        })?;

        // Phase 3: download ZIP and extract CSV
        self.progress_tracker.set_current_item(
            session_id,
            aggregate_index,
            Some("Загрузка ZIP-архива...".to_string()),
        );

        let (csv_text, zip_path, csv_path) = self
            .api_client
            .download_report_zip(&url)
            .await
            .map_err(|e| {
                self.progress_tracker.add_error(
                    session_id,
                    Some(aggregate_index.to_string()),
                    format!("Ошибка загрузки ZIP: {}", e),
                    None,
                );
                e
            })?;

        tracing::info!(
            "Payment report ZIP saved to: {}, CSV saved to: {}",
            zip_path,
            csv_path
        );

        // Phase 4: parse and import CSV rows
        self.progress_tracker.set_current_item(
            session_id,
            aggregate_index,
            Some(format!("Разбор и загрузка CSV ({})...", csv_path)),
        );

        let (inserted, updated) = payment_report::process_payment_report_csv(
            connection,
            &organization_id,
            &csv_text,
        )
        .await
        .map_err(|e| {
            self.progress_tracker.add_error(
                session_id,
                Some(aggregate_index.to_string()),
                format!("Ошибка обработки CSV: {}", e),
                None,
            );
            e
        })?;

        let total = inserted + updated;
        self.progress_tracker.update_aggregate(
            session_id,
            aggregate_index,
            total,
            Some(total),
            inserted,
            updated,
        );
        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);

        tracing::info!(
            "YM payment report import completed: inserted={}, updated={}",
            inserted,
            updated
        );

        Ok(())
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
