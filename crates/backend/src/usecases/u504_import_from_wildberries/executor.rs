#[allow(unused_imports)]
use super::wildberries_api_client::WbMarketplaceOrderRow;
use super::{
    processors::{
        commission, document, finance_report, goods_prices, marketplace_order, order, product,
        promotion, sales, supply,
    },
    progress_tracker::ProgressTracker,
    wildberries_api_client::{
        WbAdvertFullStat, WbAdvertFullStatApp, WbAdvertFullStatDay, WbAdvertFullStatNm,
        WildberriesApiClient,
    },
};
use anyhow::{Context, Result};
use contracts::domain::a026_wb_advert_daily::aggregate::{
    WbAdvertDaily, WbAdvertDailyHeader, WbAdvertDailyLine, WbAdvertDailyMetrics,
    WbAdvertDailySourceMeta,
};
use contracts::domain::a030_wb_advert_campaign::aggregate::{
    WbAdvertCampaign, WbAdvertCampaignHeader, WbAdvertCampaignSourceMeta,
};
use contracts::domain::common::AggregateId;
use contracts::system::tasks::progress::TaskProgress;
use contracts::usecases::u504_import_from_wildberries::{
    progress::ImportStatus,
    request::ImportRequest,
    response::{ImportResponse, ImportStartStatus},
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

/// Итог фонового импорта WB для планировщика: `wb_advert_partial_success` — не двигать watermark.
#[derive(Debug, Default, Clone, Copy)]
pub struct ImportRunFlags {
    pub wb_advert_partial_success: bool,
}

#[derive(Default)]
struct AdvertLineAccumulator {
    nm_name: String,
    metrics: WbAdvertDailyMetrics,
    advert_ids: BTreeSet<i64>,
    app_types: BTreeSet<i32>,
    placements: BTreeSet<String>,
}

#[derive(Default)]
struct AdvertDayAccumulator {
    totals: WbAdvertDailyMetrics,
    lines: BTreeMap<i64, AdvertLineAccumulator>,
}

const WB_ADVERT_MIN_REQUEST_INTERVAL_MS: u64 = 250;
const WB_ADVERT_FULLSTATS_CHUNK_DELAY_SECS: u64 = 21;
const WB_ADVERT_FULLSTATS_CHUNK_SIZE: usize = 50;

fn normalize_day_date(value: &str) -> String {
    if value.len() >= 10 {
        value[..10].to_string()
    } else {
        value.to_string()
    }
}

fn append_metrics(target: &mut WbAdvertDailyMetrics, source: &WbAdvertDailyMetrics) {
    target.views += source.views;
    target.clicks += source.clicks;
    target.atbs += source.atbs;
    target.orders += source.orders;
    target.shks += source.shks;
    target.sum += source.sum;
    target.sum_price += source.sum_price;
    target.canceled += source.canceled;
}

fn metrics_from_day(day: &WbAdvertFullStatDay) -> WbAdvertDailyMetrics {
    WbAdvertDailyMetrics {
        views: day.views,
        clicks: day.clicks,
        atbs: day.atbs,
        orders: day.orders,
        shks: day.shks,
        sum: day.sum,
        sum_price: day.sum_price,
        canceled: day.canceled,
        ..Default::default()
    }
}

fn metrics_from_nm(nm: &WbAdvertFullStatNm) -> WbAdvertDailyMetrics {
    WbAdvertDailyMetrics {
        views: nm.views,
        clicks: nm.clicks,
        atbs: nm.atbs,
        orders: nm.orders,
        shks: nm.shks,
        sum: nm.sum,
        sum_price: nm.sum_price,
        canceled: nm.canceled,
        ..Default::default()
    }
}

fn finalize_metrics(metrics: &mut WbAdvertDailyMetrics) {
    metrics.ctr = if metrics.views > 0 {
        (metrics.clicks as f64 / metrics.views as f64) * 100.0
    } else {
        0.0
    };
    metrics.cpc = if metrics.clicks > 0 {
        metrics.sum / metrics.clicks as f64
    } else {
        0.0
    };
    metrics.cr = if metrics.clicks > 0 {
        (metrics.orders as f64 / metrics.clicks as f64) * 100.0
    } else {
        0.0
    };
}

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

    /// Только память: активные (`Running`) сессии для лёгкого мониторинга, без БД и без диска.
    pub fn list_live_task_progress(&self) -> Vec<TaskProgress> {
        self.progress_tracker
            .snapshot_sessions()
            .into_iter()
            .filter(|p| matches!(p.status, ImportStatus::Running))
            .map(Into::into)
            .collect()
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
                "a020_wb_promotion" => "Акции WB (Календарь)",
                "a030_wb_advert_campaign" => "Рекламные кампании WB",
                "wb_advert_stats" | "wb_advert_stats_csv" => "Статистика рекламных кампаний WB",
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

    /// Подписи агрегатов (как в `start_import` + все ветки `execute_import`).
    fn wb_aggregate_display_name(aggregate_index: &str) -> &'static str {
        match aggregate_index {
            "a007_marketplace_product" => "Товары маркетплейса",
            "a015_wb_orders" => "Заказы Wildberries (Backfill)",
            "a012_wb_sales" => "Продажи Wildberries",
            "p903_wb_finance_report" => "Финансовый отчет WB",
            "p905_wb_commission_history" => "История комиссий WB",
            "p908_wb_goods_prices" => "Цены товаров WB",
            "a020_wb_promotion" => "Акции WB (Календарь)",
            "a030_wb_advert_campaign" => "Рекламные кампании WB",
            "wb_advert_stats" | "wb_advert_stats_csv" => "Статистика рекламных кампаний WB",
            "a027_wb_documents" => "Документы WB",
            "a029_wb_supply" => "Поставки WB",
            "a015_wb_orders_new" => "Новые заказы WB (оперативно)",
            "a015_wb_orders_supply_link" => "Связь заказов с поставками",
            _ => "Unknown",
        }
    }

    /// Регламентные задачи вызывают `execute_import` напрямую — сессия в трекере должна существовать,
    /// иначе `get_progress` после завершения вернёт `None` и в `sys_task_runs` не попадут метрики.
    fn ensure_progress_session(&self, session_id: &str, request: &ImportRequest) {
        if self.progress_tracker.get_progress(session_id).is_some() {
            return;
        }
        self.progress_tracker.create_session(session_id.to_string());
        for aggregate_index in &request.target_aggregates {
            self.progress_tracker.add_aggregate(
                session_id,
                aggregate_index.clone(),
                Self::wb_aggregate_display_name(aggregate_index).to_string(),
            );
        }
    }

    /// Выполнить импорт.
    /// Гарантирует вызов `complete_session` в трекере при любом исходе —
    /// даже если внутренний шаг вернул ошибку.
    pub async fn execute_import(
        &self,
        session_id: &str,
        request: &ImportRequest,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
    ) -> Result<ImportRunFlags> {
        tracing::info!("Starting Wildberries import for session: {}", session_id);

        self.ensure_progress_session(session_id, request);

        let _http_tracking = self
            .api_client
            .bind_http_tracking(Arc::clone(&self.progress_tracker), session_id.to_string());
        let work_result = self.run_aggregates(session_id, request, connection).await;

        // Трекер ВСЕГДА получает финальный статус — не только в happy path.
        let final_status = match &work_result {
            Err(e) => {
                self.progress_tracker.add_error(
                    session_id,
                    None,
                    format!("Import failed: {}", e),
                    None,
                );
                ImportStatus::Failed
            }
            Ok(flags) => {
                let tracker_errors = self
                    .progress_tracker
                    .get_progress(session_id)
                    .map(|p| p.total_errors > 0)
                    .unwrap_or(false);
                if flags.wb_advert_partial_success || tracker_errors {
                    ImportStatus::CompletedWithErrors
                } else {
                    ImportStatus::Completed
                }
            }
        };

        self.progress_tracker
            .complete_session(session_id, final_status);
        tracing::info!("Import completed for session: {}", session_id);

        work_result
    }

    /// Внутренний цикл по агрегатам; возвращает первую ошибку без изменения трекера.
    async fn run_aggregates(
        &self,
        session_id: &str,
        request: &ImportRequest,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
    ) -> Result<ImportRunFlags> {
        let mut flags = ImportRunFlags::default();
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
                "a020_wb_promotion" => {
                    self.import_wb_promotions(
                        session_id,
                        connection,
                        request.date_from,
                        request.date_to,
                    )
                    .await?;
                }
                "a027_wb_documents" => {
                    self.import_wb_documents(
                        session_id,
                        connection,
                        request.date_from,
                        request.date_to,
                    )
                    .await?;
                }
                "a029_wb_supply" => {
                    self.import_wb_supplies(
                        session_id,
                        connection,
                        request.date_from,
                        request.date_to,
                    )
                    .await?;
                }
                "a030_wb_advert_campaign" => {
                    self.import_wb_advert_campaigns(session_id, connection)
                        .await?;
                }
                "wb_advert_stats" | "wb_advert_stats_csv" => {
                    let partial = self
                        .import_wb_advert_stats(
                            session_id,
                            connection,
                            request.date_from,
                            request.date_to,
                        )
                        .await?;
                    if partial {
                        flags.wb_advert_partial_success = true;
                    }
                }
                "a015_wb_orders_new" => {
                    self.import_wb_new_marketplace_orders(
                        session_id,
                        connection,
                        request.date_from,
                        request.date_to,
                    )
                    .await?;
                }
                "a015_wb_orders_supply_link" => {
                    tracing::info!(
                        "Aggregate a015_wb_orders_supply_link is deprecated; delegating to a029_wb_supply import"
                    );
                    self.import_wb_supplies(
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
        Ok(flags)
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
        for (sale_row, raw_json) in sales_rows {
            match sales::process_sale_row(connection, &organization_id, &sale_row, &raw_json).await
            {
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

    async fn import_wb_supplies(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<()> {
        use crate::domain::a002_organization;

        let aggregate_index = "a029_wb_supply";
        let mut total_processed = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;

        tracing::info!(
            "Importing WB supplies for session: {} from {} to {}",
            session_id,
            date_from,
            date_to
        );

        let organization_id = match Uuid::parse_str(&connection.organization_ref) {
            Ok(org_uuid) => match a002_organization::service::get_by_id(org_uuid).await? {
                Some(org) => org.base.id.as_string(),
                None => {
                    let msg = format!(
                        "Организация с UUID '{}' не найдена",
                        connection.organization_ref
                    );
                    tracing::error!("{}", msg);
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        msg.clone(),
                        None,
                    );
                    anyhow::bail!("{}", msg);
                }
            },
            Err(_) => {
                let msg = format!(
                    "Некорректный organization_ref UUID: '{}'",
                    connection.organization_ref
                );
                tracing::error!("{}", msg);
                self.progress_tracker.add_error(
                    session_id,
                    Some(aggregate_index.to_string()),
                    msg.clone(),
                    None,
                );
                anyhow::bail!("{}", msg);
            }
        };

        let supply_rows = self
            .api_client
            .fetch_supplies(connection, date_from, date_to)
            .await?;

        tracing::info!("Received {} supply rows from WB API", supply_rows.len());

        for supply_row in supply_rows {
            let income_id_opt = supply_row
                .id
                .rsplit('-')
                .next()
                .and_then(|s| s.parse::<i64>().ok());

            let (supply_order_ids_loaded, supply_order_ids) = match self
                .api_client
                .fetch_supply_order_ids(connection, &supply_row.id)
                .await
            {
                Ok(order_ids) => {
                    tracing::info!(
                        "Supply {}: fetched {} order ids from WB API",
                        supply_row.id,
                        order_ids.len()
                    );
                    (true, order_ids)
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to fetch order ids for supply {} (done={}): {}",
                        supply_row.id,
                        supply_row.done.unwrap_or(false),
                        e
                    );
                    (false, vec![])
                }
            };

            tracing::info!(
                "Preparing enrichment for supply {} (done={})",
                supply_row.id,
                supply_row.done.unwrap_or(false)
            );

            if supply_order_ids_loaded {
                if let Some(income_id) = income_id_opt {
                    if let Err(e) = self
                        .sync_a015_supply_links_for_supply(
                            &supply_row.id,
                            income_id,
                            &supply_order_ids,
                        )
                        .await
                    {
                        tracing::warn!(
                            "Supply {}: failed to sync a015 supply links: {}",
                            supply_row.id,
                            e
                        );
                    }
                }
            }

            let stat_orders_fallback = if let Some(income_id) = income_id_opt {
                match crate::domain::a015_wb_orders::service::list_by_income_id(income_id).await {
                    Ok(orders) => {
                        tracing::info!(
                            "Supply {}: found {} orders via a015 income_id={}",
                            supply_row.id,
                            orders.len(),
                            income_id
                        );
                        orders
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Supply {}: a015 enrichment failed for income_id={}: {}",
                            supply_row.id,
                            income_id,
                            e
                        );
                        vec![]
                    }
                }
            } else {
                vec![]
            };

            let sticker_order_ids: Vec<i64> = supply_order_ids.clone();

            let sticker_rows = if sticker_order_ids.is_empty() {
                vec![]
            } else {
                match self
                    .api_client
                    .fetch_order_stickers(connection, &sticker_order_ids, "zplv", 58, 40)
                    .await
                {
                    Ok(mut stickers) => {
                        for sticker in &mut stickers {
                            sticker.file = None;
                        }
                        tracing::info!(
                            "Supply {}: fetched {} stickers from WB API",
                            supply_row.id,
                            stickers.len()
                        );
                        stickers
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to fetch stickers for supply {} ({} order ids): {}",
                            supply_row.id,
                            sticker_order_ids.len(),
                            e
                        );
                        vec![]
                    }
                }
            };

            match supply::process_supply_row(
                connection,
                &organization_id,
                &supply_row,
                supply_order_ids,
                sticker_rows,
                stat_orders_fallback,
            )
            .await
            {
                Ok(is_new) => {
                    total_processed += 1;
                    if is_new {
                        total_inserted += 1;
                    } else {
                        total_updated += 1;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to process WB supply {}: {}", supply_row.id, e);
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        format!("Failed to process supply {}", supply_row.id),
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

            // Brief pause between supply order fetches
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);

        tracing::info!(
            "WB supplies import completed: processed={}, inserted={}, updated={}",
            total_processed,
            total_inserted,
            total_updated
        );

        Ok(())
    }

    async fn sync_a015_supply_links_for_supply(
        &self,
        supply_id: &str,
        income_id: i64,
        current_order_ids: &[i64],
    ) -> Result<()> {
        use crate::domain::a015_wb_orders::service as orders_service;

        let current_order_ids: HashSet<i64> = current_order_ids
            .iter()
            .copied()
            .filter(|&order_id| order_id > 0)
            .collect();

        let currently_linked_orders = orders_service::list_by_income_id(income_id).await?;
        for order in &currently_linked_orders {
            let numeric_order_id = order.line.line_id.parse::<i64>().unwrap_or(0);
            if numeric_order_id > 0 && !current_order_ids.contains(&numeric_order_id) {
                orders_service::set_income_id_by_document_no(&order.header.document_no, None)
                    .await?;
            }
        }

        if current_order_ids.is_empty() {
            tracing::info!(
                "Supply {}: cleared links for income_id={} because WB returned no current orders",
                supply_id,
                income_id
            );
            return Ok(());
        }

        let known_orders = orders_service::list_by_numeric_order_ids(
            &current_order_ids.iter().copied().collect::<Vec<_>>(),
        )
        .await?;

        for order in known_orders {
            let current_income_id = order.source_meta.income_id.filter(|&value| value > 0);
            if current_income_id != Some(income_id) {
                orders_service::set_income_id_by_document_no(
                    &order.header.document_no,
                    Some(income_id),
                )
                .await?;
            }
        }

        Ok(())
    }

    /// Imports new FBS orders from Marketplace API for real-time order visibility.
    ///
    /// Flow:
    /// 1. GET /api/v3/orders/new — brand-new orders (status "waiting", not yet in supply)
    /// 2. GET /api/v3/orders?dateFrom=... — recent orders including those already in supplies
    ///
    /// For each order:
    /// - If not in a015 yet → INSERT with partial data (no financial fields)
    /// - If already in a015 → update income_id if supplyId is now known
    ///
    /// Statistics API (Backfill) should run separately to fill financial/analytics fields.
    async fn import_wb_new_marketplace_orders(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<()> {
        use crate::domain::a002_organization;

        let aggregate_index = "a015_wb_orders_new";
        let mut total_processed = 0i32;
        let mut total_inserted = 0i32;
        let mut total_updated = 0i32;

        self.progress_tracker.add_aggregate(
            session_id,
            aggregate_index.to_string(),
            "Новые заказы WB (Оперативно)".to_string(),
        );

        // Step 1: fetch brand-new orders
        let new_orders = match self
            .api_client
            .fetch_new_marketplace_orders(connection)
            .await
        {
            Ok(orders) => orders,
            Err(e) => {
                tracing::warn!("Failed to fetch /api/v3/orders/new: {}", e);
                vec![]
            }
        };
        tracing::info!("New marketplace orders (/new): {}", new_orders.len());

        // Step 2: fetch recent orders in date range (includes supplyId for assigned orders)
        let organization_id = match Uuid::parse_str(&connection.organization_ref) {
            Ok(org_uuid) => match a002_organization::service::get_by_id(org_uuid).await? {
                Some(org) => org.base.id.as_string(),
                None => {
                    let msg = format!(
                        "Организация с UUID '{}' не найдена",
                        connection.organization_ref
                    );
                    tracing::error!("{}", msg);
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        msg.clone(),
                        None,
                    );
                    anyhow::bail!("{}", msg);
                }
            },
            Err(_) => {
                let msg = format!(
                    "Некорректный organization_ref UUID: '{}'",
                    connection.organization_ref
                );
                tracing::error!("{}", msg);
                self.progress_tracker.add_error(
                    session_id,
                    Some(aggregate_index.to_string()),
                    msg.clone(),
                    None,
                );
                anyhow::bail!("{}", msg);
            }
        };

        let date_from_ts = date_from
            .and_hms_opt(0, 0, 0)
            .map(|dt| {
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc)
                    .timestamp()
            })
            .unwrap_or(0);
        let date_to_ts = date_to
            .and_hms_opt(23, 59, 59)
            .map(|dt| {
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc)
                    .timestamp()
            })
            .unwrap_or(0);

        let recent_orders = match self
            .api_client
            .fetch_marketplace_orders(connection, date_from_ts, date_to_ts)
            .await
        {
            Ok(orders) => orders,
            Err(e) => {
                tracing::warn!("Failed to fetch /api/v3/orders: {}", e);
                vec![]
            }
        };
        tracing::info!(
            "Recent marketplace orders (/orders): {}",
            recent_orders.len()
        );

        // Merge: /new orders first, then recent (dedup by id handled naturally via document_no)
        let all_orders: Vec<_> = new_orders.into_iter().chain(recent_orders).collect();
        tracing::info!("Total marketplace orders to process: {}", all_orders.len());

        for order in &all_orders {
            match marketplace_order::process_marketplace_order(connection, &organization_id, order)
                .await
            {
                Ok(is_new) => {
                    total_processed += 1;
                    if is_new {
                        total_inserted += 1;
                    } else {
                        total_updated += 1;
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to process marketplace order {}: {}", order.id, e);
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        format!("Order {}", order.id),
                        Some(e.to_string()),
                    );
                }
            }

            if total_processed % 50 == 0 {
                self.progress_tracker.update_aggregate(
                    session_id,
                    aggregate_index,
                    total_processed,
                    Some(all_orders.len() as i32),
                    total_inserted,
                    total_updated,
                );
            }
        }

        self.progress_tracker.update_aggregate(
            session_id,
            aggregate_index,
            total_processed,
            Some(all_orders.len() as i32),
            total_inserted,
            total_updated,
        );
        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);

        tracing::info!(
            "Marketplace orders import done: processed={}, inserted={}, updated={}",
            total_processed,
            total_inserted,
            total_updated
        );

        Ok(())
    }

    /// Fetches FBS orders from /api/v3/orders (WB Marketplace API v3) and updates
    /// income_id in a015_wb_orders for orders that have a supplyId assigned.
    /// This provides real-time supply linkage without the statistics API delay.
    #[allow(dead_code)]
    async fn import_wb_orders_supply_link(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<()> {
        let aggregate_index = "a015_wb_orders_supply_link";
        let mut total_synced = 0;
        let mut total_supply_rows_refreshed = 0;
        let mut touched_income_ids = BTreeSet::new();

        tracing::info!(
            "Fetching marketplace orders to update supply links: {} to {}",
            date_from,
            date_to
        );

        self.progress_tracker.add_aggregate(
            session_id,
            aggregate_index.to_string(),
            "Связь заказов с поставками".to_string(),
        );

        let date_from_ts = date_from
            .and_hms_opt(0, 0, 0)
            .map(|dt| {
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc)
                    .timestamp()
            })
            .unwrap_or(0);
        let date_to_ts = date_to
            .and_hms_opt(23, 59, 59)
            .map(|dt| {
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc)
                    .timestamp()
            })
            .unwrap_or(0);

        let marketplace_orders = match self
            .api_client
            .fetch_marketplace_orders(connection, date_from_ts, date_to_ts)
            .await
        {
            Ok(orders) => orders,
            Err(e) => {
                let msg = format!("Failed to fetch marketplace orders: {}", e);
                tracing::error!("{}", msg);
                self.progress_tracker.add_error(
                    session_id,
                    Some(aggregate_index.to_string()),
                    msg,
                    None,
                );
                return Ok(());
            }
        };

        let total_fetched = marketplace_orders.len();
        tracing::info!("Marketplace orders fetched: {}", total_fetched);

        for order in &marketplace_orders {
            let document_no = match &order.rid {
                Some(rid) if !rid.is_empty() => rid.clone(),
                _ => continue,
            };

            let Some(existing_order) =
                crate::domain::a015_wb_orders::service::get_by_document_no(&document_no).await?
            else {
                continue;
            };

            if order.id > 0 {
                let _ = crate::domain::a015_wb_orders::service::update_line_id_by_document_no(
                    &document_no,
                    order.id,
                )
                .await;
            }

            let old_income_id = existing_order
                .source_meta
                .income_id
                .filter(|&value| value > 0);
            let new_income_id = match order.supply_id.as_deref().map(str::trim) {
                Some("") | None => None,
                Some(supply_id) => match supply_id
                    .rsplit('-')
                    .next()
                    .and_then(|s| s.parse::<i64>().ok())
                    .filter(|&value| value > 0)
                {
                    Some(value) => Some(value),
                    None => {
                        tracing::warn!("Cannot parse income_id from supplyId: {}", supply_id);
                        old_income_id
                    }
                },
            };

            if let Some(value) = old_income_id {
                touched_income_ids.insert(value);
            }
            if let Some(value) = new_income_id {
                touched_income_ids.insert(value);
            }

            if old_income_id == new_income_id {
                continue;
            }

            match crate::domain::a015_wb_orders::service::set_income_id_by_document_no(
                &document_no,
                new_income_id,
            )
            .await
            {
                Ok(_) => {
                    total_synced += 1;
                    tracing::debug!(
                        "Synced order {} supply link: {:?} -> {:?}",
                        document_no,
                        old_income_id,
                        new_income_id
                    );
                }
                Err(e) => {
                    tracing::warn!("Failed to sync income_id for order {}: {}", document_no, e);
                }
            }

            self.progress_tracker.update_aggregate(
                session_id,
                aggregate_index,
                total_synced as i32,
                Some(total_fetched as i32),
                total_synced as i32,
                0,
            );
        }

        for income_id in touched_income_ids {
            let supply_id = format!("WB-GI-{}", income_id);
            let Some(mut supply_doc) =
                crate::domain::a029_wb_supply::service::get_by_supply_id(&supply_id).await?
            else {
                continue;
            };

            let stat_orders =
                match crate::domain::a015_wb_orders::service::list_by_income_id(income_id).await {
                    Ok(orders) if !orders.is_empty() => orders,
                    Ok(_) => continue,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to load a015 orders for missing supply income_id={}: {}",
                            income_id,
                            e
                        );
                        continue;
                    }
                };

            supply_doc.supply_orders = supply::build_supply_rows_from_stat_orders(&stat_orders);
            supply_doc.base.description = format!(
                "WB Supply {} - {} orders",
                supply_id,
                supply_doc.supply_orders.len()
            );

            match crate::domain::a029_wb_supply::service::store_document(supply_doc).await {
                Ok(_) => {
                    total_supply_rows_refreshed += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to refresh linked orders for supply {}: {}",
                        supply_id,
                        e
                    );
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        supply_id,
                        Some(e.to_string()),
                    );
                }
            }
        }

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);

        tracing::info!(
            "Supply link import completed: fetched={}, links_synced={}, existing_supplies_refreshed={}",
            total_fetched,
            total_synced,
            total_supply_rows_refreshed
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
        let mut processed_days = 0;
        let mut changed_days = 0;
        let mut total_source_rows = 0;
        let mut total_gl_rows = 0;

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
                "Р”РЅРµРІРЅРѕР№ reconciliation {} - {} (API: 1 Р·Р°РїСЂРѕСЃ/РјРёРЅ)",
                date_from.format("%Y-%m-%d"),
                date_to.format("%Y-%m-%d")
            )),
        );

        let total_days = (date_to - date_from).num_days() as i32 + 1;
        let mut current_date = date_from;
        while current_date <= date_to {
            self.progress_tracker.set_current_item(
                session_id,
                aggregate_index,
                Some(format!(
                    "Р”Р°С‚Р° {}: Р·Р°РіСЂСѓР·РєР° Рё reconciliation",
                    current_date.format("%Y-%m-%d")
                )),
            );

            let report_rows = self
                .api_client
                .fetch_finance_report_by_period(connection, current_date, current_date)
                .await?;

            let mut entries = Vec::with_capacity(report_rows.len());
            for row in report_rows {
                match finance_report::map_finance_report_row(connection, &organization_id, &row)
                    .await
                {
                    Ok(entry) => entries.push(entry),
                    Err(e) => {
                        tracing::error!(
                            "Failed to map finance report row for {}: {}",
                            current_date,
                            e
                        );
                        self.progress_tracker.add_error(
                            session_id,
                            Some(aggregate_index.to_string()),
                            format!(
                                "Failed to map finance report row for {}",
                                current_date.format("%Y-%m-%d")
                            ),
                            Some(e.to_string()),
                        );
                    }
                }
            }

            match crate::projections::p903_wb_finance_report::service::reconcile_day(
                &connection.to_string_id(),
                current_date,
                &entries,
            )
            .await
            {
                Ok(result) => {
                    if result.changed {
                        changed_days += 1;
                    }
                    total_source_rows += result.source_rows as i32;
                    total_gl_rows += result.general_ledger_rows as i32;
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to reconcile finance report day {}: {}",
                        current_date,
                        e
                    );
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        format!(
                            "Failed to reconcile finance report day {}",
                            current_date.format("%Y-%m-%d")
                        ),
                        Some(e.to_string()),
                    );
                }
            }

            processed_days += 1;
            self.progress_tracker.update_aggregate(
                session_id,
                aggregate_index,
                processed_days,
                Some(total_days),
                total_source_rows,
                total_gl_rows,
            );

            current_date += chrono::Duration::days(1);
        }

        self.progress_tracker.update_aggregate(
            session_id,
            aggregate_index,
            processed_days,
            Some(total_days),
            total_source_rows,
            total_gl_rows,
        );

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);
        tracing::info!(
            "WB finance report import completed: days={}, changed_days={}, source_rows={}, gl_rows={}",
            processed_days,
            changed_days,
            total_source_rows,
            total_gl_rows
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

        tracing::info!(
            "Importing WB commission history for session: {}",
            session_id
        );

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

    /// Импорт акций WB Calendar в a020
    async fn import_wb_promotions(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<()> {
        let aggregate_index = "a020_wb_promotion";
        let mut total_processed = 0i32;
        let mut total_new = 0i32;
        let mut total_updated = 0i32;

        tracing::info!(
            "Importing WB calendar promotions for session: {}, period: {} - {}",
            session_id,
            date_from,
            date_to
        );

        // Получить organization_id из connection
        let organization_id = {
            use contracts::domain::common::AggregateId;
            let org_id = connection.organization_ref.clone();
            if org_id.is_empty() {
                tracing::warn!(
                    "organization_ref is empty for connection {}",
                    connection.base.id.as_string()
                );
            }
            org_id
        };

        // Форматируем даты в RFC3339 (WB ожидает ISO 8601 с временной зоной)
        let start_dt = format!("{}T00:00:00Z", date_from.format("%Y-%m-%d"));
        let end_dt = format!("{}T23:59:59Z", date_to.format("%Y-%m-%d"));

        // Загружаем список акций
        let promotions = match self
            .api_client
            .fetch_calendar_promotions(connection, &start_dt, &end_dt, false)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Failed to fetch WB calendar promotions: {}", e);
                self.progress_tracker.add_error(
                    session_id,
                    Some(aggregate_index.to_string()),
                    "Failed to fetch promotions list".to_string(),
                    Some(e.to_string()),
                );
                self.progress_tracker
                    .complete_aggregate(session_id, aggregate_index);
                return Ok(());
            }
        };

        tracing::info!("Found {} WB promotions in period", promotions.len());

        // Batch-fetch details для всех акций (по 100 за раз)
        let mut details_map: std::collections::HashMap<i64, crate::usecases::u504_import_from_wildberries::wildberries_api_client::WbCalendarPromotionDetail> =
            std::collections::HashMap::new();
        {
            let all_ids: Vec<i64> = promotions.iter().map(|p| p.id).collect();
            for chunk in all_ids.chunks(100) {
                match self
                    .api_client
                    .fetch_promotion_details(connection, chunk)
                    .await
                {
                    Ok(details_list) => {
                        for d in details_list {
                            details_map.insert(d.id, d);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch promotion details batch: {}", e);
                    }
                }
            }
            tracing::info!(
                "Loaded details for {}/{} promotions",
                details_map.len(),
                promotions.len()
            );
        }

        for promo in &promotions {
            let promo_name = promo
                .name
                .clone()
                .unwrap_or_else(|| format!("{}", promo.id));
            self.progress_tracker.set_current_item(
                session_id,
                aggregate_index,
                Some(format!("{} - {}", promo.id, promo_name)),
            );

            // Загружаем список nmId товаров для этой акции (не работает для type="auto")
            let promo_type = promo.promotion_type.as_deref();
            let nm_ids = match self
                .api_client
                .fetch_promotion_nomenclatures(connection, promo.id, promo_type)
                .await
            {
                Ok(ids) => ids,
                Err(e) => {
                    tracing::warn!(
                        "Failed to fetch nomenclatures for promotion {}: {}",
                        promo.id,
                        e
                    );
                    vec![]
                }
            };

            let details = details_map.get(&promo.id);

            match promotion::process_promotion(connection, &organization_id, promo, nm_ids, details)
                .await
            {
                Ok(is_new) => {
                    total_processed += 1;
                    if is_new {
                        total_new += 1;
                    } else {
                        total_updated += 1;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to process promotion {}: {}", promo.id, e);
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        format!("Failed to process promotion {}", promo.id),
                        Some(e.to_string()),
                    );
                }
            }

            self.progress_tracker.update_aggregate(
                session_id,
                aggregate_index,
                total_processed,
                Some(promotions.len() as i32),
                total_new,
                total_updated,
            );
        }

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);
        tracing::info!(
            "WB promotions import completed: new={}, updated={}",
            total_new,
            total_updated
        );

        Ok(())
    }

    /// Получить статистику рекламных кампаний WB за период и сохранить в CSV
    async fn import_wb_documents(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<()> {
        use crate::domain::a002_organization;

        let aggregate_index = "a027_wb_documents";
        let mut total_processed = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;

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

        let rows = self
            .api_client
            .fetch_documents_list(connection, date_from, date_to)
            .await?;

        for row in rows {
            self.progress_tracker.set_current_item(
                session_id,
                aggregate_index,
                Some(row.service_name.clone()),
            );

            match document::process_document_header(connection, &organization_id, &row).await {
                Ok(is_new) => {
                    total_processed += 1;
                    if is_new {
                        total_inserted += 1;
                    } else {
                        total_updated += 1;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to process WB document {}: {}", row.service_name, e);
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        format!("Failed to process WB document {}", row.service_name),
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
            .set_current_item(session_id, aggregate_index, None);
        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);

        Ok(())
    }

    async fn import_wb_advert_campaigns(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
    ) -> Result<usize> {
        let aggregate_index = "a030_wb_advert_campaign";
        self.progress_tracker.set_current_item(
            session_id,
            aggregate_index,
            Some("Получение списка кампаний".into()),
        );

        let summaries = match self
            .api_client
            .fetch_advert_campaign_summaries(connection)
            .await
        {
            Ok(summaries) => summaries,
            Err(err) => {
                let existing = crate::domain::a030_wb_advert_campaign::service::list_by_connection(
                    &connection.to_string_id(),
                )
                .await
                .context("Failed to read existing a030_wb_advert_campaign fallback")?;
                let message = format!("Failed to fetch WB advert campaign summaries: {}", err);
                self.progress_tracker.add_error(
                    session_id,
                    Some(aggregate_index.to_string()),
                    message.clone(),
                    Some(err.to_string()),
                );

                if !existing.is_empty() {
                    tracing::warn!(
                        "{}; keeping existing a030 campaigns for connection={} count={}",
                        message,
                        connection.to_string_id(),
                        existing.len()
                    );
                    self.progress_tracker.update_aggregate(
                        session_id,
                        aggregate_index,
                        existing.len() as i32,
                        Some(existing.len() as i32),
                        0,
                        0,
                    );
                    self.progress_tracker
                        .complete_aggregate(session_id, aggregate_index);
                    return Ok(existing.len());
                }

                anyhow::bail!(
                    "{}; no existing a030 campaigns are available for fallback",
                    message
                );
            }
        };

        if summaries.is_empty() {
            self.progress_tracker
                .complete_aggregate(session_id, aggregate_index);
            return Ok(0);
        }

        self.progress_tracker.update_aggregate(
            session_id,
            aggregate_index,
            0,
            Some(summaries.len() as i32),
            0,
            0,
        );

        // Load lightweight snapshot (advert_id → change_time + has_info_json) to decide
        // which campaigns need a fresh API call.  Full aggregates are NOT loaded here —
        // the upsert will preserve existing info_json for campaigns we pass Null for.
        let snapshot = crate::domain::a030_wb_advert_campaign::service::list_info_snapshot(
            &connection.to_string_id(),
        )
        .await
        .unwrap_or_default();

        // Classify every campaign from the summaries response.
        // Priority 1 — new (not in DB): must fetch info.
        // Priority 2 — existing but change_time changed: fetch info (data may differ).
        // Priority 3 — existing, unchanged change_time, has info_json: skip API call.
        // Priority 4 — existing, unchanged change_time, no info_json: fetch info.
        let mut priority1: Vec<i64> = Vec::new(); // new
        let mut priority2: Vec<i64> = Vec::new(); // changed
        let mut priority4: Vec<i64> = Vec::new(); // no info yet

        for summary in &summaries {
            let advert_id = summary.advert_id;
            match snapshot.get(&advert_id) {
                None => priority1.push(advert_id),
                Some(snap) => {
                    let same_change_time = snap.change_time == summary.change_time;
                    if same_change_time && snap.has_info_json {
                        // nothing to do — cached info is still valid
                    } else if !snap.has_info_json {
                        priority4.push(advert_id);
                    } else {
                        priority2.push(advert_id);
                    }
                }
            }
        }

        // Build the ordered list of IDs to fetch, then take at most 50 (one API call).
        // Remaining IDs are deferred to the next run; the upsert preserves their info_json.
        let mut need_info_ids: Vec<i64> = priority1
            .into_iter()
            .chain(priority4)
            .chain(priority2)
            .collect();

        const MAX_INFO_IDS_PER_RUN: usize = 50;
        let deferred_count = need_info_ids.len().saturating_sub(MAX_INFO_IDS_PER_RUN);
        need_info_ids.truncate(MAX_INFO_IDS_PER_RUN);

        let cached_count = summaries.len() - need_info_ids.len() - deferred_count;
        tracing::info!(
            "WB advert campaign info: total={}, cached={}, fetch_now={}, deferred_to_next_run={}",
            summaries.len(),
            cached_count,
            need_info_ids.len(),
            deferred_count,
        );
        if deferred_count > 0 {
            tracing::warn!(
                "WB advert API rate limit: only {} of {} campaigns will get fresh info_json \
                 this run (limit=1 req/hour). Deferred {} campaigns will be updated on \
                 subsequent runs.",
                need_info_ids.len(),
                summaries.len(),
                deferred_count,
            );
        }

        // Fetch info_json for the selected campaigns (at most one API call).
        let mut info_by_id: HashMap<i64, serde_json::Value> = HashMap::new();
        if !need_info_ids.is_empty() {
            tokio::time::sleep(tokio::time::Duration::from_millis(
                WB_ADVERT_MIN_REQUEST_INTERVAL_MS,
            ))
            .await;

            self.progress_tracker.set_current_item(
                session_id,
                aggregate_index,
                Some(format!(
                    "Свойства {} кампаний из WB API",
                    need_info_ids.len()
                )),
            );

            match self
                .api_client
                .fetch_advert_campaign_info_values(connection, &need_info_ids)
                .await
            {
                Ok(values) => {
                    for value in values {
                        if let Some(id) = value
                            .get("advertId")
                            .or_else(|| value.get("id"))
                            .and_then(|v| v.as_i64())
                        {
                            info_by_id.insert(id, value);
                        }
                    }
                    tracing::info!(
                        "WB advert campaign info fetched: requested={}, received={}",
                        need_info_ids.len(),
                        info_by_id.len(),
                    );
                }
                Err(err) => {
                    let message = format!(
                        "WB advert campaign info fetch failed; saving all campaigns without \
                         fresh info_json (existing info_json is preserved by upsert): {}",
                        err
                    );
                    tracing::warn!("{}", message);
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        message,
                        Some(err.to_string()),
                    );
                }
            }
        }

        // Capture before the loop consumes info_by_id via .remove().
        let api_fetched_count = info_by_id.len();

        let fetched_at = chrono::Utc::now().to_rfc3339();
        let mut campaigns = Vec::with_capacity(summaries.len());
        for summary in &summaries {
            // Campaigns not in info_by_id get Null — the upsert will preserve existing info_json.
            let info_json = info_by_id
                .remove(&summary.advert_id)
                .unwrap_or(serde_json::Value::Null);
            let header = WbAdvertCampaignHeader {
                advert_id: summary.advert_id,
                connection_id: connection.to_string_id(),
                organization_id: connection.organization_ref.clone(),
                marketplace_id: connection.marketplace_id.clone(),
                campaign_type: summary.campaign_type,
                status: summary.status,
                change_time: summary.change_time.clone(),
                nm_count: 0, // recalculated by before_write() from info_json
            };
            let source_meta = WbAdvertCampaignSourceMeta {
                source: "wb_advert_campaigns".to_string(),
                fetched_at: fetched_at.clone(),
                info_json,
            };
            let mut campaign = WbAdvertCampaign::new_for_insert(header, source_meta);
            campaign.before_write();
            campaign.validate().map_err(|e| anyhow::anyhow!(e))?;
            campaigns.push(campaign);
        }

        let (new_count, total_count) =
            crate::domain::a030_wb_advert_campaign::service::upsert_many(&campaigns)
                .await
                .context("Failed to save a030_wb_advert_campaign")?;
        self.progress_tracker.update_aggregate(
            session_id,
            aggregate_index,
            total_count as i32,
            Some(summaries.len() as i32),
            new_count as i32,         // "Новые" = физически добавленные записи
            api_fetched_count as i32, // "Изменено" = получили свежий info_json из API
        );
        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);
        tracing::info!(
            "WB Advert campaigns synced: connection={}, total={}, new={}, api_fetched={}, deferred={}",
            connection.to_string_id(),
            total_count,
            new_count,
            api_fetched_count,
            deferred_count,
        );
        Ok(total_count)
    }

    /// Загрузка статистики рекламы WB без промежуточного CSV.
    /// Возвращает `true`, если были частичные ошибки API (данные за период всё равно пересобираются из успешных ответов).
    async fn import_wb_advert_stats(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<bool> {
        let aggregate_index = "wb_advert_stats";
        let begin_date = date_from.format("%Y-%m-%d").to_string();
        let end_date = date_to.format("%Y-%m-%d").to_string();

        tracing::info!(
            "WB Advert stats: session={}, period={} to {}",
            session_id,
            begin_date,
            end_date
        );

        let all_advert_ids =
            crate::domain::a030_wb_advert_campaign::service::list_advert_ids_by_connection(
                &connection.to_string_id(),
            )
            .await
            .context("Failed to read advert ids from a030_wb_advert_campaign")?;

        // Filter out completed campaigns (status=7) that ended before the period start —
        // they cannot have any activity in [date_from, date_to].
        let advert_ids =
            crate::domain::a030_wb_advert_campaign::service::list_advert_ids_for_period(
                &connection.to_string_id(),
                &begin_date,
            )
            .await
            .context("Failed to read filtered advert ids from a030_wb_advert_campaign")?;

        let skipped_count = all_advert_ids.len().saturating_sub(advert_ids.len());
        tracing::info!(
            "WB Advert: total={}, period_relevant={}, skipped_completed={}",
            all_advert_ids.len(),
            advert_ids.len(),
            skipped_count,
        );

        if advert_ids.is_empty() {
            tracing::info!(
                "WB Advert: no relevant campaigns for period, clearing existing documents"
            );
            crate::domain::a026_wb_advert_daily::service::replace_for_period(
                &connection.to_string_id(),
                &begin_date,
                &end_date,
                &[],
            )
            .await?;
            self.progress_tracker
                .complete_aggregate(session_id, aggregate_index);
            return Ok(false);
        }

        tracing::info!(
            "WB Advert: {} campaigns → {} chunks of up to {} (delay {}s each)",
            advert_ids.len(),
            advert_ids.chunks(WB_ADVERT_FULLSTATS_CHUNK_SIZE).count(),
            WB_ADVERT_FULLSTATS_CHUNK_SIZE,
            WB_ADVERT_FULLSTATS_CHUNK_DELAY_SECS,
        );
        self.progress_tracker.update_aggregate(
            session_id,
            aggregate_index,
            0,
            Some(advert_ids.len() as i32),
            0,
            0,
        );

        let chunks: Vec<&[i64]> = advert_ids.chunks(WB_ADVERT_FULLSTATS_CHUNK_SIZE).collect();
        let total_chunks = chunks.len();
        let mut processed_campaigns = 0i32;
        let mut had_fetch_errors = false;
        let mut all_stats: Vec<WbAdvertFullStat> = Vec::new();
        let mut successful_advert_ids: Vec<i64> = Vec::new();

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            self.progress_tracker.set_current_item(
                session_id,
                aggregate_index,
                Some(format!(
                    "Чанк {}/{} (advertIds: {}..)",
                    chunk_idx + 1,
                    total_chunks,
                    chunk[0]
                )),
            );

            match self
                .api_client
                .fetch_advert_fullstats(connection, chunk, &begin_date, &end_date)
                .await
            {
                Ok(stats) => {
                    processed_campaigns += chunk.len() as i32;
                    successful_advert_ids.extend(chunk.iter().copied());
                    all_stats.extend(stats.iter().cloned());
                }
                Err(e) => {
                    had_fetch_errors = true;
                    tracing::warn!(
                        "Failed to fetch fullstats for chunk {}: {}",
                        chunk_idx + 1,
                        e
                    );
                    self.progress_tracker.add_error(
                        session_id,
                        Some(aggregate_index.to_string()),
                        format!("Chunk {} failed: {}", chunk_idx + 1, e),
                        Some(e.to_string()),
                    );
                }
            }

            self.progress_tracker.update_aggregate(
                session_id,
                aggregate_index,
                processed_campaigns,
                Some(advert_ids.len() as i32),
                all_stats.len() as i32,
                0,
            );

            if chunk_idx + 1 < total_chunks {
                tokio::time::sleep(tokio::time::Duration::from_secs(
                    WB_ADVERT_FULLSTATS_CHUNK_DELAY_SECS,
                ))
                .await;
            }
        }

        if had_fetch_errors {
            self.progress_tracker.add_error(
                session_id,
                Some(aggregate_index.to_string()),
                "Часть рекламной статистики не загрузилась; сохранены только успешно полученные данные"
                    .to_string(),
                None,
            );
        }

        if had_fetch_errors && successful_advert_ids.is_empty() {
            anyhow::bail!(
                "WB Advert fullstats failed for all {} chunks; existing a026 data was left unchanged",
                total_chunks
            );
        }

        let build_started_at = std::time::Instant::now();
        tracing::info!(
            "WB Advert document build started: connection={}, stats={}",
            connection.to_string_id(),
            all_stats.len()
        );
        let documents = self
            .build_wb_advert_documents(connection, &all_stats)
            .await
            .with_context(|| {
                format!(
                    "Failed during WB advert document build for connection={} period={}..{}",
                    connection.to_string_id(),
                    begin_date,
                    end_date
                )
            })?;
        let document_ids: Vec<Uuid> = documents
            .iter()
            .map(|document| document.base.id.value())
            .collect();
        tracing::info!(
            "WB Advert document build completed: connection={}, documents={}, elapsed_ms={}",
            connection.to_string_id(),
            documents.len(),
            build_started_at.elapsed().as_millis()
        );

        let replace_started_at = std::time::Instant::now();
        tracing::info!(
            "WB Advert document replace started: connection={}, period={}..{}, documents={}",
            connection.to_string_id(),
            begin_date,
            end_date,
            documents.len()
        );
        let documents_count = if had_fetch_errors {
            successful_advert_ids.sort_unstable();
            successful_advert_ids.dedup();
            crate::domain::a026_wb_advert_daily::service::replace_for_period_advert_ids(
                &connection.to_string_id(),
                &begin_date,
                &end_date,
                &successful_advert_ids,
                &documents,
            )
            .await
            .with_context(|| {
                format!(
                    "Failed during scoped WB advert replace_for_period for connection={} period={}..{} successful_adverts={} documents={}",
                    connection.to_string_id(),
                    begin_date,
                    end_date,
                    successful_advert_ids.len(),
                    documents.len()
                )
            })?
        } else {
            crate::domain::a026_wb_advert_daily::service::replace_for_period(
                &connection.to_string_id(),
                &begin_date,
                &end_date,
                &documents,
            )
            .await
            .with_context(|| {
                format!(
                    "Failed during WB advert replace_for_period for connection={} period={}..{} documents={}",
                    connection.to_string_id(),
                    begin_date,
                    end_date,
                    documents.len()
                )
            })?
        };

        let post_started_at = std::time::Instant::now();
        tracing::info!(
            "WB Advert auto-post started: connection={}, period={}..{}, documents={}",
            connection.to_string_id(),
            begin_date,
            end_date,
            document_ids.len()
        );
        for document_id in &document_ids {
            crate::domain::a026_wb_advert_daily::posting::post_document(*document_id)
                .await
                .with_context(|| {
                    format!(
                        "Failed during WB advert auto-post for connection={} document_id={}",
                        connection.to_string_id(),
                        document_id
                    )
                })?;
        }
        tracing::info!(
            "WB Advert auto-post completed: connection={}, period={}..{}, documents={}, elapsed_ms={}",
            connection.to_string_id(),
            begin_date,
            end_date,
            document_ids.len(),
            post_started_at.elapsed().as_millis()
        );

        self.progress_tracker.update_aggregate(
            session_id,
            aggregate_index,
            processed_campaigns,
            Some(advert_ids.len() as i32),
            documents_count as i32,
            0,
        );

        tracing::info!(
            "WB Advert documents synced: connection={}, period={}..{}, documents={}, elapsed_ms={}",
            connection.to_string_id(),
            begin_date,
            end_date,
            documents_count,
            replace_started_at.elapsed().as_millis()
        );

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);
        tracing::info!(
            "WB Advert stats completed: {} campaigns processed, {} stat records, partial={}",
            processed_campaigns,
            all_stats.len(),
            had_fetch_errors
        );

        Ok(had_fetch_errors)
    }

    async fn build_wb_advert_documents(
        &self,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        stats: &[WbAdvertFullStat],
    ) -> Result<Vec<WbAdvertDaily>> {
        let mut by_doc: BTreeMap<(String, i64), AdvertDayAccumulator> = BTreeMap::new();

        for stat in stats {
            for day in &stat.days {
                let date_key = normalize_day_date(&day.date);
                let doc_acc = by_doc.entry((date_key, stat.advert_id)).or_default();
                append_metrics(&mut doc_acc.totals, &metrics_from_day(day));

                for app in &day.apps {
                    self.accumulate_day_app(doc_acc, stat.advert_id, app);
                }
            }
        }

        let total_line_groups: usize = by_doc.values().map(|day| day.lines.len()).sum();
        tracing::info!(
            "WB Advert document build prepared: connection={}, documents={}, nm_groups={}",
            connection.to_string_id(),
            by_doc.len(),
            total_line_groups
        );

        let mut nomenclature_cache: HashMap<i64, Option<String>> = HashMap::new();
        let mut documents = Vec::with_capacity(by_doc.len());

        for ((document_date, advert_id), mut day_acc) in by_doc {
            let mut lines = Vec::with_capacity(day_acc.lines.len());
            let mut attributed_totals = WbAdvertDailyMetrics::default();

            for (nm_id, line_acc) in &mut day_acc.lines {
                let nomenclature_ref = self
                    .resolve_wb_nomenclature_ref(connection, *nm_id, &mut nomenclature_cache)
                    .await?;

                let mut metrics = line_acc.metrics.clone();
                finalize_metrics(&mut metrics);
                append_metrics(&mut attributed_totals, &metrics);

                lines.push(WbAdvertDailyLine {
                    nm_id: *nm_id,
                    nm_name: line_acc.nm_name.clone(),
                    nomenclature_ref,
                    advert_ids: line_acc.advert_ids.iter().copied().collect(),
                    app_types: line_acc.app_types.iter().copied().collect(),
                    placements: line_acc.placements.iter().cloned().collect(),
                    metrics,
                });
            }

            lines.sort_by(|a, b| {
                a.nm_name
                    .to_lowercase()
                    .cmp(&b.nm_name.to_lowercase())
                    .then_with(|| a.nm_id.cmp(&b.nm_id))
            });

            let mut totals = day_acc.totals.clone();
            finalize_metrics(&mut totals);

            let mut unattributed_totals =
                crate::domain::a026_wb_advert_daily::repository::subtract_metrics(
                    &day_acc.totals,
                    &attributed_totals,
                );
            finalize_metrics(&mut unattributed_totals);

            let header = WbAdvertDailyHeader {
                document_no: format!("WB-ADV-{}-{}", advert_id, document_date),
                document_date: document_date.clone(),
                advert_id,
                connection_id: connection.to_string_id(),
                organization_id: connection.organization_ref.clone(),
                marketplace_id: connection.marketplace_id.clone(),
            };

            let source_meta = WbAdvertDailySourceMeta {
                source: "wb_advert_stats".to_string(),
                fetched_at: chrono::Utc::now().to_rfc3339(),
            };

            let mut document = WbAdvertDaily::new_for_insert(
                header,
                totals,
                unattributed_totals,
                lines,
                source_meta,
            );
            document.before_write();
            document.validate().map_err(|e| anyhow::anyhow!(e))?;
            documents.push(document);
        }

        Ok(documents)
    }

    fn accumulate_day_app(
        &self,
        day_acc: &mut AdvertDayAccumulator,
        advert_id: i64,
        app: &WbAdvertFullStatApp,
    ) {
        for nm in &app.nms {
            let line = day_acc.lines.entry(nm.nm_id).or_default();
            if line.nm_name.is_empty() {
                line.nm_name = nm.name.clone().unwrap_or_default();
            }
            append_metrics(&mut line.metrics, &metrics_from_nm(nm));
            line.advert_ids.insert(advert_id);
            line.app_types.insert(app.app_type);
        }
    }

    async fn resolve_wb_nomenclature_ref(
        &self,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        nm_id: i64,
        cache: &mut HashMap<i64, Option<String>>,
    ) -> Result<Option<String>> {
        if let Some(cached) = cache.get(&nm_id) {
            return Ok(cached.clone());
        }

        let resolved =
            crate::usecases::u504_import_from_wildberries::processors::wb_nomenclature::resolve_wb_nomenclature_ref(
                &connection.to_string_id(),
                nm_id,
                None,
            )
            .await?;
        cache.insert(nm_id, resolved.clone());
        Ok(resolved)
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
