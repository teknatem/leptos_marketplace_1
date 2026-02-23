use super::{
    progress_tracker::ProgressTracker,
    wildberries_api_client::WildberriesApiClient,
    processors::{product, sales, order, finance_report, commission, goods_prices},
};
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
    pub progress_tracker: Arc<ProgressTracker>,
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
                "a015_wb_orders" => "Заказы Wildberries (Backfill)",
                "a012_wb_sales" => "Продажи Wildberries",
                "p903_wb_finance_report" => "Финансовый отчет WB",
                "p905_wb_commission_history" => "История комиссий WB",
                "p908_wb_goods_prices" => "Цены товаров WB",
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
    ) -> Option<contracts::usecases::u504_import_from_wildberries::progress::ImportProgress> {
        self.progress_tracker.get_progress(session_id)
    }

    /// Выполнить импорт
    pub async fn execute_import(
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
                "a012_wb_sales" => {
                    self.import_wb_sales(
                        session_id,
                        connection,
                        request.date_from,
                        request.date_to,
                    )
                    .await?;
                }
                "a015_wb_orders" => {
                    self.import_wb_orders(
                        session_id,
                        connection,
                        request.date_from,
                        request.date_to,
                    )
                    .await?;
                }
                "p903_wb_finance_report" => {
                    self.import_wb_finance_report(
                        session_id,
                        connection,
                        request.date_from,
                        request.date_to,
                    )
                    .await?;
                }
                "p905_wb_commission_history" => {
                    self.import_commission_history(session_id, connection)
                        .await?;
                }
                "p908_wb_goods_prices" => {
                    self.import_wb_goods_prices(session_id, connection).await?;
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

        // Получаем товары страницами через Wildberries API
        loop {
            let list_response = self
                .api_client
                .fetch_product_list(connection, page_size, cursor.clone())
                .await?;

            // Если API вернул total, сохраняем его (только при первом запросе)
            if expected_total.is_none() && list_response.cursor.total > 0 {
                expected_total = Some(list_response.cursor.total as i32);
            }

            let cards = list_response.cards;
            let batch_size = cards.len();

            if cards.is_empty() {
                break;
            }

            // Обрабатываем каждый товар
            for card in cards {
                let product_name = card
                    .title
                    .clone()
                    .unwrap_or_else(|| "Без названия".to_string());
                let display_name = format!("{} - {}", card.nm_id, product_name);

                self.progress_tracker.set_current_item(
                    session_id,
                    aggregate_index,
                    Some(display_name),
                );

                match product::process_product(connection, &card).await {
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

            // Определяем, есть ли еще страницы
            let next_cursor = if total_processed >= expected_total.unwrap_or(i32::MAX) {
                None
            } else if batch_size < page_size as usize {
                None
            } else {
                Some(list_response.cursor.clone())
            };

            // Обновляем курсор для следующей страницы
            cursor = next_cursor.clone();

            if cursor.is_none() {
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

    /// Импорт продаж из Wildberries API в a012_wb_sales
    async fn import_wb_sales(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<()> {
        use crate::domain::a002_organization;

        let aggregate_index = "a012_wb_sales";
        let mut total_processed = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;

        tracing::info!(
            "Importing WB sales for session: {} from date: {} to date: {}",
            session_id,
            date_from,
            date_to
        );

        // Получаем ID организации по UUID-ссылке из подключения
        let organization_id = match Uuid::parse_str(&connection.organization_ref) {
            Ok(org_uuid) => match a002_organization::service::get_by_id(org_uuid).await? {
                Some(org) => org.base.id.as_string(),
                None => {
                    let error_msg = format!(
                        "Организация с UUID '{}' не найдена в справочнике",
                        connection.organization_ref
                    );
                    tracing::error!("{}", error_msg);
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        error_msg.clone(),
                        None,
                    );
                    anyhow::bail!("{}", error_msg);
                }
            },
            Err(_) => {
                let error_msg = format!(
                    "Некорректный organization_ref UUID в подключении: '{}'",
                    connection.organization_ref
                );
                tracing::error!("{}", error_msg);
                self.progress_tracker.add_error(
                    session_id,
                    Some(aggregate_index.to_string()),
                    error_msg.clone(),
                    None,
                );
                anyhow::bail!("{}", error_msg);
            }
        };

        // Получаем продажи из API WB
        let sales_rows = self
            .api_client
            .fetch_sales(connection, date_from, date_to)
            .await?;

        tracing::info!("Received {} sale rows from WB API", sales_rows.len());

        // Обрабатываем каждую продажу
        for sale_row in sales_rows {
            match sales::process_sale_row(connection, &organization_id, &sale_row).await {
                Ok(is_new) => {
                    total_processed += 1;
                    if is_new {
                        total_inserted += 1;
                    } else {
                        total_updated += 1;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to process WB sale: {}", e);
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        "Failed to process WB sale".to_string(),
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
            "WB sales import completed: processed={}, inserted={}, updated={}",
            total_processed,
            total_inserted,
            total_updated
        );

        Ok(())
    }

    /// Импорт заказов из Wildberries API в a015_wb_orders
    async fn import_wb_orders(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<()> {
        use crate::domain::a002_organization;

        let aggregate_index = "a015_wb_orders";
        let mut total_processed = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;

        tracing::info!(
            "Importing WB orders for session: {} from date: {} to date: {}",
            session_id,
            date_from,
            date_to
        );

        // Получаем ID организации по UUID-ссылке из подключения
        let organization_id = match Uuid::parse_str(&connection.organization_ref) {
            Ok(org_uuid) => match a002_organization::service::get_by_id(org_uuid).await? {
                Some(org) => org.base.id.as_string(),
                None => {
                    let error_msg = format!(
                        "Организация с UUID '{}' не найдена в справочнике",
                        connection.organization_ref
                    );
                    tracing::error!("{}", error_msg);
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        error_msg.clone(),
                        None,
                    );
                    anyhow::bail!("{}", error_msg);
                }
            },
            Err(_) => {
                let error_msg = format!(
                    "Некорректный organization_ref UUID в подключении: '{}'",
                    connection.organization_ref
                );
                tracing::error!("{}", error_msg);
                self.progress_tracker.add_error(
                    session_id,
                    Some(aggregate_index.to_string()),
                    error_msg.clone(),
                    None,
                );
                anyhow::bail!("{}", error_msg);
            }
        };

        // Получаем заказы из API WB
        let order_rows = self
            .api_client
            .fetch_orders(connection, date_from, date_to)
            .await?;

        tracing::info!("Received {} order rows from WB API", order_rows.len());

        // Обрабатываем каждый заказ
        for order_row in order_rows {
            match order::process_order_row(connection, &organization_id, &order_row).await {
                Ok(is_new) => {
                    total_processed += 1;
                    if is_new {
                        total_inserted += 1;
                    } else {
                        total_updated += 1;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to process WB order: {}", e);
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        "Failed to process WB order".to_string(),
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
            "WB orders import completed: processed={}, inserted={}, updated={}",
            total_processed,
            total_inserted,
            total_updated
        );

        Ok(())
    }

    /// Импорт финансовых отчетов Wildberries из API в p903_wb_finance_report
    /// 
    /// ВАЖНО: API reportDetailByPeriod имеет лимит 1 запрос в минуту!
    /// Данные загружаются за весь период с пагинацией, а не по дням.
    async fn import_wb_finance_report(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<()> {
        use crate::domain::a002_organization;

        let aggregate_index = "p903_wb_finance_report";
        let mut total_processed = 0;
        let mut total_inserted = 0;

        tracing::info!(
            "Importing WB finance report for session: {} from date: {} to date: {}",
            session_id,
            date_from,
            date_to
        );

        // Получаем ID организации по UUID-ссылке из подключения
        let organization_id = match Uuid::parse_str(&connection.organization_ref) {
            Ok(org_uuid) => match a002_organization::service::get_by_id(org_uuid).await? {
                Some(org) => org.base.id.as_string(),
                None => {
                    let error_msg = format!(
                        "Организация с UUID '{}' не найдена в справочнике",
                        connection.organization_ref
                    );
                    tracing::error!("{}", error_msg);
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        error_msg.clone(),
                        None,
                    );
                    anyhow::bail!("{}", error_msg);
                }
            },
            Err(_) => {
                let error_msg = format!(
                    "Некорректный organization_ref UUID в подключении: '{}'",
                    connection.organization_ref
                );
                tracing::error!("{}", error_msg);
                self.progress_tracker.add_error(
                    session_id,
                    Some(aggregate_index.to_string()),
                    error_msg.clone(),
                    None,
                );
                anyhow::bail!("{}", error_msg);
            }
        };

        self.progress_tracker.set_current_item(
            session_id,
            aggregate_index,
            Some(format!(
                "Загрузка за период {} - {} (API: 1 запрос/мин)",
                date_from.format("%Y-%m-%d"),
                date_to.format("%Y-%m-%d")
            )),
        );

        // Загружаем финансовые отчеты за весь период с пагинацией
        // API сам использует пагинацию через rrdid и ждет между запросами
        let report_rows = self
            .api_client
            .fetch_finance_report_by_period(connection, date_from, date_to)
            .await?;

        let total_rows = report_rows.len() as i32;
        tracing::info!(
            "Received {} finance report rows for period {} - {}",
            total_rows,
            date_from,
            date_to
        );

        // Вставляем новые записи
        for row in report_rows {
            match finance_report::process_finance_report_row(connection, &organization_id, &row).await {
                Ok(_) => {
                    total_inserted += 1;
                }
                Err(e) => {
                    tracing::error!("Failed to process finance report row: {}", e);
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        "Failed to process finance report row".to_string(),
                        Some(e.to_string()),
                    );
                }
            }

            total_processed += 1;

            // Обновляем прогресс каждые 100 записей
            if total_processed % 100 == 0 {
                self.progress_tracker.update_aggregate(
                    session_id,
                    aggregate_index,
                    total_processed,
                    Some(total_rows),
                    total_inserted,
                    0,
                );
            }
        }

        // Финальное обновление с точным числом записей
        self.progress_tracker.update_aggregate(
            session_id,
            aggregate_index,
            total_processed,
            Some(total_rows),
            total_inserted,
            0,
        );

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);
        tracing::info!(
            "WB finance report import completed: processed={}, inserted={}",
            total_processed,
            total_inserted
        );

        Ok(())
    }

    /// Импорт истории комиссий Wildberries в p905
    async fn import_commission_history(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
    ) -> Result<()> {
        let aggregate_index = "p905_wb_commission_history";
        let mut new_records = 0;
        let mut updated_records = 0;
        let mut skipped_records = 0;

        tracing::info!("Importing WB commission history for session: {}", session_id);

        // Получаем тарифы из API
        let tariffs = self.api_client.fetch_commission_tariffs(connection).await?;

        // Фильтруем пустые или некорректные записи
        let filtered_tariffs: Vec<_> = tariffs
            .into_iter()
            .filter(|t| t.subject_id > 0 && !t.subject_name.is_empty())
            .collect();

        tracing::info!("Processing {} commission tariffs", filtered_tariffs.len());

        let today = chrono::Utc::now().date_naive();

        for tariff in filtered_tariffs {
            match commission::process_commission_tariff(connection, &tariff, today).await {
                Ok((created, is_new)) => {
                    if created {
                        if is_new {
                            new_records += 1;
                        } else {
                            updated_records += 1;
                        }
                    } else {
                        skipped_records += 1;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to process commission tariff: {}", e);
                }
            }
        }

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);
        tracing::info!(
            "WB commission history import completed: new={}, updated={}, skipped={}",
            new_records,
            updated_records,
            skipped_records
        );

        Ok(())
    }

    /// Импорт цен товаров Wildberries в p908
    async fn import_wb_goods_prices(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
    ) -> Result<()> {
        let aggregate_index = "p908_wb_goods_prices";
        let page_size = 1000;
        let mut offset = 0i32;
        let mut total_processed = 0i32;
        let mut total_upserted = 0i32;

        tracing::info!("Importing WB goods prices for session: {}", session_id);

        loop {
            let page = self
                .api_client
                .fetch_goods_prices(connection, page_size, offset)
                .await?;

            if page.is_empty() {
                break;
            }

            let page_len = page.len() as i32;

            for row in &page {
                match goods_prices::process_goods_price(connection, row).await {
                    Ok(_) => {
                        total_upserted += 1;
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to process goods price row nm_id={}: {}",
                            row.nm_id,
                            e
                        );
                        self.progress_tracker.add_error(
                            session_id,
                            Some(aggregate_index.to_string()),
                            format!("Failed to process nm_id={}", row.nm_id),
                            Some(e.to_string()),
                        );
                    }
                }
                total_processed += 1;
            }

            self.progress_tracker.update_aggregate(
                session_id,
                aggregate_index,
                total_processed,
                None,
                total_upserted,
                0,
            );

            if page_len < page_size {
                break;
            }

            offset += page_size;
        }

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);
        tracing::info!(
            "WB goods prices import completed: processed={}, upserted={}",
            total_processed,
            total_upserted
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
