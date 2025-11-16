use super::{ozon_api_client::OzonApiClient, progress_tracker::ProgressTracker};
use crate::domain::a007_marketplace_product;
use anyhow::Result;
use contracts::domain::common::AggregateId;
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
                "a008_marketplace_sales" => "Продажи (фин. транзакции)",
                "a009_ozon_returns" => "Возвраты OZON",
                "a010_ozon_fbs_posting" => "OZON FBS Документы продаж",
                "a011_ozon_fbo_posting" => "OZON FBO Документы продаж",
                "a014_ozon_transactions" => "Транзакции OZON",
                "p902_ozon_finance_realization" => "Финансовые данные реализации OZON",
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
                "a008_marketplace_sales" => {
                    self.import_marketplace_sales(
                        session_id,
                        connection,
                        request.date_from,
                        request.date_to,
                    )
                    .await?;
                }
                "a009_ozon_returns" => {
                    self.import_ozon_returns(
                        session_id,
                        connection,
                        request.date_from,
                        request.date_to,
                    )
                    .await?;
                }
                "a010_ozon_fbs_posting" => {
                    self.import_ozon_fbs_postings(
                        session_id,
                        connection,
                        request.date_from,
                        request.date_to,
                    )
                    .await?;
                }
                "a011_ozon_fbo_posting" => {
                    self.import_ozon_fbo_postings(
                        session_id,
                        connection,
                        request.date_from,
                        request.date_to,
                    )
                    .await?;
                }
                "a014_ozon_transactions" => {
                    self.import_ozon_transactions(
                        session_id,
                        connection,
                        request.date_from,
                        request.date_to,
                    )
                    .await?;
                }
                "p902_ozon_finance_realization" => {
                    self.import_ozon_finance_realization(
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
                let display_name = format!("{} - {}", product_info.offer_id, product_info.name);

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
                tracing::warn!(
                    "last_id did not change, stopping to prevent infinite loop. last_id: {:?}",
                    last_id
                );
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

        // Берем первый barcode из списка
        let barcode = product.barcodes.first().cloned();

        // Получаем category_id
        let category_id = product.description_category_id.map(|id| id.to_string());

        if let Some(mut existing_product) = existing {
            // Обновляем существующий товар
            tracing::debug!("Updating existing product: {}", marketplace_sku);

            existing_product.base.code = product.offer_id.clone();
            existing_product.base.description = product.name.clone();
            existing_product.marketplace_sku = marketplace_sku;
            existing_product.barcode = barcode.clone();
            existing_product.article = product.offer_id.clone();
            existing_product.category_id = category_id.clone();
            existing_product.last_update = Some(chrono::Utc::now());
            existing_product.before_write();

            a007_marketplace_product::repository::update(&existing_product).await?;
            Ok(false)
        } else {
            // Создаем новый товар
            tracing::debug!("Inserting new product: {}", marketplace_sku);

            let mut new_product = MarketplaceProduct::new_for_insert(
                product.offer_id.clone(),
                product.name.clone(),
                connection.marketplace_id.clone(),
                connection.base.id.as_string(),
                marketplace_sku,
                barcode,
                product.offer_id.clone(),
                None, // brand
                category_id,
                None, // category_name
                Some(chrono::Utc::now()),
                None, // nomenclature_ref
                None, // comment
            );

            // Автоматический поиск номенклатуры по артикулу
            let _ = a007_marketplace_product::service::search_and_set_nomenclature(&mut new_product).await;

            a007_marketplace_product::repository::insert(&new_product).await?;
            Ok(true)
        }
    }

    /// Импорт финансовых транзакций (продажи/возвраты) в a008
    async fn import_marketplace_sales(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<()> {
        use crate::domain::a002_organization;
        use crate::domain::a007_marketplace_product;
        use crate::domain::a008_marketplace_sales;

        let aggregate_index = "a008_marketplace_sales";
        let mut total_processed = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;

        // Получаем ID организации по названию
        let organization_id =
            match a002_organization::repository::get_by_description(&connection.organization)
                .await?
            {
                Some(org) => org.base.id.as_string(),
                None => {
                    let error_msg = format!(
                        "Организация '{}' не найдена в справочнике",
                        connection.organization
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

        // Кеш сопоставлений SKU/offer_id -> product_id
        use std::collections::HashMap;
        let mut sku_to_product_id: HashMap<String, String> = HashMap::new();

        // Пагинация страницами
        let page_size = 1000;
        let mut page = 1;

        loop {
            let resp = self
                .api_client
                .fetch_finance_transactions(connection, date_from, date_to, page, page_size)
                .await?;

            if resp.result.operations.is_empty() {
                break;
            }

            for op in resp.result.operations {
                self.progress_tracker.set_current_item(
                    session_id,
                    aggregate_index,
                    Some(op.operation_type.clone()),
                );

                // Берем позиции из items (могут быть несколько на одну операцию)
                let items = if op.items.is_empty() {
                    vec![super::ozon_api_client::OzonFinanceItem {
                        sku: None,
                        offer_id: None,
                        quantity: None,
                    }]
                } else {
                    op.items.clone()
                };

                // Вычисляем общее количество для распределения суммы
                let total_qty: i32 = items.iter().map(|item| item.quantity.unwrap_or(1)).sum();

                // Если все quantity отсутствуют, используем количество позиций
                let items_count = items.len() as i32;
                let divisor = if total_qty > 0 {
                    total_qty
                } else {
                    items_count
                };

                for item in items {
                    let key = item
                        .sku
                        .map(|v| v.to_string())
                        .or(item.offer_id.clone())
                        .unwrap_or_default();
                    if key.is_empty() {
                        self.progress_tracker.add_error(
                            session_id,
                            Some(aggregate_index.to_string()),
                            "Операция без sku/offer_id".to_string(),
                            None,
                        );
                        continue;
                    }

                    // Получаем или создаем product_id в a007
                    let product_id = if let Some(pid) = sku_to_product_id.get(&key) {
                        pid.clone()
                    } else {
                        let existing =
                            a007_marketplace_product::repository::get_by_marketplace_sku(
                                &connection.marketplace_id,
                                &key,
                            )
                            .await?;
                        let pid = if let Some(mp) = existing {
                            mp.to_string_id()
                        } else {
                            let mut new = contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct::new_for_insert(
                                key.clone(),
                                key.clone(),
                                connection.marketplace_id.clone(),
                                connection.base.id.as_string(),
                                key.clone(),
                                None,
                                key.clone(),
                                None,
                                None,
                                None,
                                Some(chrono::Utc::now()),
                                None,
                                Some("auto-created from finance operation".to_string()),
                            );
                            // Автоматический поиск номенклатуры по артикулу
                            let _ = a007_marketplace_product::service::search_and_set_nomenclature(&mut new).await;
                            let id = a007_marketplace_product::repository::insert(&new).await?;
                            id.to_string()
                        };
                        sku_to_product_id.insert(key.clone(), pid.clone());
                        pid
                    };

                    // Дата начисления = дата операции (дата часть); API вернул "YYYY-MM-DD HH:MM:SS"
                    let accrual_date = chrono::NaiveDateTime::parse_from_str(
                        &op.operation_date,
                        "%Y-%m-%d %H:%M:%S",
                    )
                    .map(|dt| dt.date())
                    .unwrap_or_else(|_| {
                        chrono::DateTime::parse_from_rfc3339(&op.operation_date)
                            .map(|dt| dt.naive_utc().date())
                            .unwrap_or(date_from)
                    });

                    // Количество: используем из API или 1 по умолчанию
                    let qty = item.quantity.unwrap_or(1);

                    // Распределяем сумму операции пропорционально количеству
                    let revenue = if divisor > 0 {
                        (op.amount * qty as f64) / divisor as f64
                    } else {
                        op.amount
                    };

                    // Читаем существующую запись по ключу (включая operation_type)
                    let existing = a008_marketplace_sales::repository::get_by_key(
                        &connection.base.id.as_string(),
                        &product_id,
                        accrual_date,
                        &op.operation_type,
                    )
                    .await?;

                    if let Some(mut sale) = existing {
                        sale.quantity += qty;
                        sale.revenue += revenue;
                        sale.before_write();
                        a008_marketplace_sales::repository::update(&sale).await?;
                        total_updated += 1;
                    } else {
                        let dto = contracts::domain::a008_marketplace_sales::aggregate::MarketplaceSalesDto {
                            id: None,
                            code: None,
                            description: format!("{} {}", op.operation_type, key),
                            connection_id: connection.base.id.as_string(),
                            organization_id: organization_id.clone(),
                            marketplace_id: connection.marketplace_id.clone(),
                            accrual_date,
                            product_id: product_id.clone(),
                            quantity: qty,
                            revenue,
                            operation_type: op.operation_type.clone(),
                            comment: None,
                        };
                        let _ = a008_marketplace_sales::service::create(dto).await?;
                        total_inserted += 1;
                    }
                }

                total_processed += 1;
                self.progress_tracker.update_aggregate(
                    session_id,
                    aggregate_index,
                    total_processed,
                    None,
                    total_inserted,
                    total_updated,
                );
            }

            // Пагинация: если API вернул has_next, используем его, иначе вычисляем по page/total при наличии
            if let Some(has_next) = resp.result.has_next {
                if !has_next {
                    break;
                }
                page += 1;
            } else if let (Some(p), Some(ps), Some(t)) =
                (resp.result.page, resp.result.page_size, resp.result.total)
            {
                if (p as usize * ps as usize) >= t as usize {
                    break;
                }
                page += 1;
            } else {
                // если нет явных полей пагинации, выходим когда получили пустой список (выше) или продолжаем одну страницу
                break;
            }
            self.progress_tracker
                .set_current_item(session_id, aggregate_index, None);
        }

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);
        Ok(())
    }

    /// Импорт возвратов из OZON в a009
    async fn import_ozon_returns(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<()> {
        use crate::domain::a002_organization;
        use crate::domain::a009_ozon_returns;

        let aggregate_index = "a009_ozon_returns";
        let mut total_processed = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;

        // Получаем ID организации по названию
        let organization_id =
            match a002_organization::repository::get_by_description(&connection.organization)
                .await?
            {
                Some(org) => org.base.id.as_string(),
                None => {
                    let error_msg = format!(
                        "Организация '{}' не найдена в справочнике",
                        connection.organization
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

        // Курсорная пагинация через last_id
        let mut last_id: i64 = 0;
        let limit = 500; // Максимум для OZON API

        loop {
            let resp = self
                .api_client
                .fetch_returns_list(connection, date_from, date_to, last_id, limit)
                .await?;

            if resp.returns.is_empty() {
                break;
            }

            let returns_count = resp.returns.len();
            tracing::info!(
                "Received {} returns from API (last_id={})",
                returns_count,
                last_id
            );

            // Сохраняем last_id для следующего запроса перед перемещением вектора
            let new_last_id = resp.returns.last().map(|r| r.id).unwrap_or(last_id);

            for return_item in resp.returns {
                let return_id_str = return_item.id.to_string();
                let return_reason = return_item
                    .return_reason_name
                    .as_deref()
                    .unwrap_or("Unknown");
                let return_type = return_item.return_type.as_deref().unwrap_or("Unknown");
                let order_id_str = return_item
                    .order_id
                    .map(|id| id.to_string())
                    .unwrap_or_default();
                let order_number = return_item.order_number.as_deref().unwrap_or("");
                let posting_number = return_item.posting_number.as_deref().unwrap_or("");
                let clearing_id_str = return_item.clearing_id.map(|id| id.to_string());
                let return_clearing_id_str =
                    return_item.return_clearing_id.map(|id| id.to_string());

                // Парсим дату возврата из logistic.return_date (ISO 8601 формат: "2022-01-19T19:55:35.433Z")
                let return_date = return_item
                    .logistic
                    .as_ref()
                    .and_then(|l| l.return_date.as_ref())
                    .and_then(|moment_str| {
                        // Пытаемся распарсить как ISO datetime
                        chrono::DateTime::parse_from_rfc3339(moment_str)
                            .map(|dt| dt.naive_utc().date())
                            .or_else(|_| {
                                // Fallback: пытаемся распарсить как дату YYYY-MM-DD
                                chrono::NaiveDate::parse_from_str(moment_str, "%Y-%m-%d")
                            })
                            .ok()
                    })
                    .unwrap_or(date_from); // Если не удалось распарсить, используем дату из периода запроса

                // Проверяем наличие товара: если product == None или все поля внутри пустые
                let has_product = return_item
                    .product
                    .as_ref()
                    .and_then(|p| {
                        // Если хотя бы одно поле заполнено, считаем что товар есть
                        if p.sku.is_some() || p.name.is_some() || p.offer_id.is_some() {
                            Some(())
                        } else {
                            None
                        }
                    })
                    .is_some();

                tracing::debug!(
                    "Return {} has_product={}, product: {:?}",
                    return_id_str,
                    has_product,
                    return_item.product
                );

                // Если нет товара в возврате, создаем одну запись без товара
                if !has_product {
                    let display_name = format!("Возврат {} без товаров", return_id_str);
                    self.progress_tracker.set_current_item(
                        session_id,
                        aggregate_index,
                        Some(display_name.clone()),
                    );

                    // Проверяем существует ли возврат по ключу (connection_id, return_id, sku="")
                    let existing = a009_ozon_returns::repository::get_by_return_key(
                        &connection.base.id.as_string(),
                        &return_id_str,
                        "",
                    )
                    .await?;

                    if let Some(mut ozon_return) = existing {
                        // Обновляем существующий возврат
                        ozon_return.return_reason_name = return_reason.to_string();
                        ozon_return.return_type = return_type.to_string();
                        ozon_return.return_date = return_date;
                        ozon_return.order_id = order_id_str.clone();
                        ozon_return.order_number = order_number.to_string();
                        ozon_return.posting_number = posting_number.to_string();
                        ozon_return.clearing_id = clearing_id_str.clone();
                        ozon_return.return_clearing_id = return_clearing_id_str.clone();
                        ozon_return.before_write();
                        a009_ozon_returns::repository::update(&ozon_return).await?;
                        total_updated += 1;
                    } else {
                        // Создаем новый возврат
                        let dto = contracts::domain::a009_ozon_returns::aggregate::OzonReturnsDto {
                            id: None,
                            code: None,
                            description: display_name,
                            connection_id: connection.base.id.as_string(),
                            organization_id: organization_id.clone(),
                            marketplace_id: connection.marketplace_id.clone(),
                            return_id: return_id_str.clone(),
                            return_date,
                            return_reason_name: return_reason.to_string(),
                            return_type: return_type.to_string(),
                            order_id: order_id_str.clone(),
                            order_number: order_number.to_string(),
                            sku: String::new(),
                            product_name: String::new(),
                            price: 0.0,
                            quantity: 0,
                            posting_number: posting_number.to_string(),
                            clearing_id: clearing_id_str.clone(),
                            return_clearing_id: return_clearing_id_str.clone(),
                            comment: None,
                        };
                        let _ = a009_ozon_returns::service::create(dto).await?;
                        total_inserted += 1;
                    }

                    total_processed += 1;
                    self.progress_tracker.update_aggregate(
                        session_id,
                        aggregate_index,
                        total_processed,
                        None,
                        total_inserted,
                        total_updated,
                    );
                } else {
                    // Обрабатываем товар в возврате
                    let product = return_item.product.as_ref().unwrap(); // уже проверили что has_product
                    let sku_str = product.sku.map(|s| s.to_string()).unwrap_or_default();
                    let product_name = product.name.as_deref().unwrap_or("Unknown");
                    let price = product.price.as_ref().and_then(|p| p.price).unwrap_or(0.0);
                    let quantity = product.quantity.unwrap_or(0);

                    tracing::debug!(
                        "Return {} product: sku={}, name={}, price={}, qty={}",
                        return_id_str,
                        sku_str,
                        product_name,
                        price,
                        quantity
                    );

                    let display_name =
                        format!("{} - {} - {}", return_id_str, sku_str, product_name);
                    self.progress_tracker.set_current_item(
                        session_id,
                        aggregate_index,
                        Some(display_name.clone()),
                    );

                    // Проверяем существует ли возврат по ключу (connection_id, return_id, sku)
                    let existing = a009_ozon_returns::repository::get_by_return_key(
                        &connection.base.id.as_string(),
                        &return_id_str,
                        &sku_str,
                    )
                    .await?;

                    if let Some(mut ozon_return) = existing {
                        // Обновляем существующий возврат
                        ozon_return.sku = sku_str.clone();
                        ozon_return.product_name = product_name.to_string();
                        ozon_return.price = price;
                        ozon_return.quantity = quantity;
                        ozon_return.return_reason_name = return_reason.to_string();
                        ozon_return.return_type = return_type.to_string();
                        ozon_return.return_date = return_date;
                        ozon_return.order_id = order_id_str.clone();
                        ozon_return.order_number = order_number.to_string();
                        ozon_return.posting_number = posting_number.to_string();
                        ozon_return.clearing_id = clearing_id_str.clone();
                        ozon_return.return_clearing_id = return_clearing_id_str.clone();
                        ozon_return.before_write();
                        a009_ozon_returns::repository::update(&ozon_return).await?;
                        total_updated += 1;
                    } else {
                        // Создаем новый возврат
                        let dto = contracts::domain::a009_ozon_returns::aggregate::OzonReturnsDto {
                            id: None,
                            code: None,
                            description: display_name,
                            connection_id: connection.base.id.as_string(),
                            organization_id: organization_id.clone(),
                            marketplace_id: connection.marketplace_id.clone(),
                            return_id: return_id_str.clone(),
                            return_date,
                            return_reason_name: return_reason.to_string(),
                            return_type: return_type.to_string(),
                            order_id: order_id_str.clone(),
                            order_number: order_number.to_string(),
                            sku: sku_str.clone(),
                            product_name: product_name.to_string(),
                            price,
                            quantity,
                            posting_number: posting_number.to_string(),
                            clearing_id: clearing_id_str.clone(),
                            return_clearing_id: return_clearing_id_str.clone(),
                            comment: None,
                        };
                        let _ = a009_ozon_returns::service::create(dto).await?;
                        total_inserted += 1;
                    }

                    total_processed += 1;
                    self.progress_tracker.update_aggregate(
                        session_id,
                        aggregate_index,
                        total_processed,
                        None,
                        total_inserted,
                        total_updated,
                    );
                }
            }

            // Если получили меньше limit, значит это последняя страница
            if returns_count < limit as usize {
                break;
            }

            // Защита от зацикливания: если last_id не изменился
            if returns_count > 0 && last_id == new_last_id {
                tracing::warn!("last_id did not change, stopping pagination");
                break;
            }

            last_id = new_last_id;
        }

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);
        tracing::info!(
            "OZON returns import completed: processed={}, inserted={}, updated={}",
            total_processed,
            total_inserted,
            total_updated
        );
        Ok(())
    }

    /// Импорт OZON FBS Posting в a010
    async fn import_ozon_fbs_postings(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<()> {
        use crate::domain::a002_organization;
        use crate::domain::a010_ozon_fbs_posting;
        use contracts::domain::a010_ozon_fbs_posting::aggregate::{
            OzonFbsPosting, OzonFbsPostingHeader, OzonFbsPostingLine, OzonFbsPostingSourceMeta,
            OzonFbsPostingState,
        };

        let aggregate_index = "a010_ozon_fbs_posting";
        let mut total_processed = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;

        // Получаем ID организации по названию
        let organization_id =
            match a002_organization::repository::get_by_description(&connection.organization)
                .await?
            {
                Some(org) => org.base.id.as_string(),
                None => {
                    let error_msg = format!(
                        "Организация '{}' не найдена в справочнике",
                        connection.organization
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

        // Пагинация
        let limit = 100;
        let mut offset = 0;

        loop {
            let resp = self
                .api_client
                .fetch_fbs_postings(connection, date_from, date_to, limit, offset)
                .await?;

            if resp.result.postings.is_empty() {
                break;
            }

            let postings_count = resp.result.postings.len();
            tracing::info!(
                "Received {} FBS postings from API (offset={})",
                postings_count,
                offset
            );

            for posting in resp.result.postings {
                let posting_number = posting.posting_number.clone();
                self.progress_tracker.set_current_item(
                    session_id,
                    aggregate_index,
                    Some(format!("FBS Posting {}", posting_number)),
                );

                // Проверяем, существует ли документ
                let existing =
                    a010_ozon_fbs_posting::service::get_by_document_no(&posting_number).await?;
                let is_new = existing.is_none();

                // Конвертируем продукты в строки документа
                let lines: Vec<OzonFbsPostingLine> = posting
                    .products
                    .iter()
                    .enumerate()
                    .map(|(idx, product)| OzonFbsPostingLine {
                        line_id: format!("{}_{}", posting_number, idx + 1),
                        product_id: product
                            .product_id
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| product.offer_id.clone()),
                        offer_id: product.offer_id.clone(),
                        name: product.name.clone(),
                        barcode: None,
                        qty: product.quantity as f64,
                        price_list: product.price,
                        discount_total: None,
                        price_effective: product.price,
                        amount_line: product.price.map(|p| p * product.quantity as f64),
                        currency_code: product.currency_code.clone(),
                    })
                    .collect();

                // Парсим delivered_at (используем delivering_date как основное поле, delivered_at как fallback)
                let date_source = if posting.delivering_date.is_some() {
                    "delivering_date"
                } else if posting.delivered_at.is_some() {
                    "delivered_at"
                } else {
                    "none"
                };

                let delivered_at = posting
                    .delivering_date
                    .as_ref()
                    .or(posting.delivered_at.as_ref())
                    .and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(s)
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .ok()
                    });

                tracing::debug!(
                    "FBS Posting {}: date_source={}, delivering_date={:?}, delivered_at={:?}, parsed_date={:?}",
                    posting_number,
                    date_source,
                    posting.delivering_date,
                    posting.delivered_at,
                    delivered_at
                );

                // Создаем документ
                let header = OzonFbsPostingHeader {
                    document_no: posting_number.clone(),
                    scheme: "FBS".to_string(),
                    connection_id: connection.base.id.as_string(),
                    organization_id: organization_id.clone(),
                    marketplace_id: connection.marketplace_id.clone(),
                };

                // Нормализуем статус
                let status_norm = normalize_ozon_status(&posting.status);
                // Все постинги проводятся, но проекции создаются только для DELIVERED
                let is_posted = true;

                let state = OzonFbsPostingState {
                    status_raw: posting.status.clone(),
                    status_norm,
                    substatus_raw: posting.substatus.clone(),
                    delivered_at,
                    updated_at_source: None,
                };

                let source_meta = OzonFbsPostingSourceMeta {
                    raw_payload_ref: String::new(), // Будет заполнен в service
                    fetched_at: chrono::Utc::now(),
                    document_version: 1,
                };

                let document = OzonFbsPosting::new_for_insert(
                    posting_number.clone(),
                    format!("FBS Posting {}", posting_number),
                    header,
                    lines,
                    state,
                    source_meta,
                    is_posted, // Все постинги проводятся, проекции только для DELIVERED
                );

                // Сохраняем с сырым JSON (автоматически проецируется в P900)
                let raw_json = serde_json::to_string(&posting)?;
                match a010_ozon_fbs_posting::service::store_document_with_raw(document, &raw_json)
                    .await
                {
                    Ok(_) => {
                        total_processed += 1;
                        if is_new {
                            total_inserted += 1;
                        } else {
                            total_updated += 1;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to process FBS posting {}: {}", posting_number, e);
                        self.progress_tracker.add_error(
                            session_id,
                            Some(aggregate_index.to_string()),
                            format!("Failed to process FBS posting {}", posting_number),
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

            // Проверяем, есть ли еще данные
            if !resp.result.has_next {
                break;
            }

            offset += limit;
            self.progress_tracker
                .set_current_item(session_id, aggregate_index, None);
        }

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);
        tracing::info!(
            "OZON FBS postings import completed: processed={}, inserted={}, updated={}",
            total_processed,
            total_inserted,
            total_updated
        );
        Ok(())
    }

    /// Импорт OZON FBO Posting в a011
    async fn import_ozon_fbo_postings(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<()> {
        use crate::domain::a002_organization;
        use crate::domain::a011_ozon_fbo_posting;
        use contracts::domain::a011_ozon_fbo_posting::aggregate::{
            OzonFboPosting, OzonFboPostingHeader, OzonFboPostingLine, OzonFboPostingSourceMeta,
            OzonFboPostingState,
        };

        let aggregate_index = "a011_ozon_fbo_posting";
        let mut total_processed = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;

        // Получаем ID организации по названию
        let organization_id =
            match a002_organization::repository::get_by_description(&connection.organization)
                .await?
            {
                Some(org) => org.base.id.as_string(),
                None => {
                    let error_msg = format!(
                        "Организация '{}' не найдена в справочнике",
                        connection.organization
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

        // Пагинация
        let limit = 100;
        let mut offset = 0;

        loop {
            let resp = self
                .api_client
                .fetch_fbo_postings(connection, date_from, date_to, limit, offset)
                .await?;

            let postings_count = resp.result.len();
            if postings_count == 0 {
                break;
            }

            tracing::info!(
                "Received {} FBO postings from API (offset={})",
                postings_count,
                offset
            );

            for posting in resp.result {
                let posting_number = posting.posting_number.clone();
                self.progress_tracker.set_current_item(
                    session_id,
                    aggregate_index,
                    Some(format!("FBO Posting {}", posting_number)),
                );

                // Проверяем, существует ли документ
                let existing =
                    a011_ozon_fbo_posting::service::get_by_document_no(&posting_number).await?;
                let is_new = existing.is_none();

                // Конвертируем продукты в строки документа
                let lines: Vec<OzonFboPostingLine> = posting
                    .products
                    .iter()
                    .enumerate()
                    .map(|(idx, product)| OzonFboPostingLine {
                        line_id: format!("{}_{}", posting_number, idx + 1),
                        product_id: product
                            .product_id
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| product.offer_id.clone()),
                        offer_id: product.offer_id.clone(),
                        name: product.name.clone(),
                        barcode: None,
                        qty: product.quantity as f64,
                        price_list: product.price,
                        discount_total: None,
                        price_effective: product.price,
                        amount_line: product.price.map(|p| p * product.quantity as f64),
                        currency_code: product.currency_code.clone(),
                    })
                    .collect();

                // Парсим delivered_at (FBO использует delivered_at и delivering_date)
                let date_source = if posting.delivered_at.is_some() {
                    "delivered_at"
                } else if posting.delivering_date.is_some() {
                    "delivering_date"
                } else {
                    "none"
                };

                let delivered_at = posting
                    .delivered_at
                    .as_ref()
                    .or(posting.delivering_date.as_ref())
                    .and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(s)
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .ok()
                    });

                tracing::debug!(
                    "FBO Posting {}: date_source={}, delivered_at={:?}, delivering_date={:?}, parsed_date={:?}",
                    posting_number,
                    date_source,
                    posting.delivered_at,
                    posting.delivering_date,
                    delivered_at
                );

                // Парсим дату создания заказа
                let created_at = posting.created_at.as_ref().and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(s)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .ok()
                });

                // Создаем документ
                let header = OzonFboPostingHeader {
                    document_no: posting_number.clone(),
                    scheme: "FBO".to_string(),
                    connection_id: connection.base.id.as_string(),
                    organization_id: organization_id.clone(),
                    marketplace_id: connection.marketplace_id.clone(),
                };

                // Нормализуем статус
                let status_norm = normalize_ozon_status(&posting.status);
                // Все постинги (включая CANCELLED) проводятся в P900 согласно логике OZON
                let is_posted = true;

                let state = OzonFboPostingState {
                    status_raw: posting.status.clone(),
                    status_norm,
                    substatus_raw: posting.substatus.clone(),
                    created_at,
                    delivered_at,
                    updated_at_source: None,
                };

                let source_meta = OzonFboPostingSourceMeta {
                    raw_payload_ref: String::new(), // Будет заполнен в service
                    fetched_at: chrono::Utc::now(),
                    document_version: 1,
                };

                let document = OzonFboPosting::new_for_insert(
                    posting_number.clone(),
                    format!("FBO Posting {}", posting_number),
                    header,
                    lines,
                    state,
                    source_meta,
                    is_posted, // Все постинги проводятся (DELIVERED и CANCELLED)
                );

                // Сохраняем с сырым JSON (автоматически проецируется в P900)
                let raw_json = serde_json::to_string(&posting)?;
                match a011_ozon_fbo_posting::service::store_document_with_raw(document, &raw_json)
                    .await
                {
                    Ok(_) => {
                        total_processed += 1;
                        if is_new {
                            total_inserted += 1;
                        } else {
                            total_updated += 1;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to process FBO posting {}: {}", posting_number, e);
                        self.progress_tracker.add_error(
                            session_id,
                            Some(aggregate_index.to_string()),
                            format!("Failed to process FBO posting {}", posting_number),
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

            // FBO API не имеет has_next, проверяем по количеству записей
            if postings_count < limit as usize {
                break;
            }

            offset += limit;
            self.progress_tracker
                .set_current_item(session_id, aggregate_index, None);
        }

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);
        tracing::info!(
            "OZON FBO postings import completed: processed={}, inserted={}, updated={}",
            total_processed,
            total_inserted,
            total_updated
        );
        Ok(())
    }

    /// Импорт финансовых данных реализации OZON в p902
    pub async fn import_ozon_finance_realization(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<()> {
        use crate::domain::a002_organization;
        use crate::projections::p902_ozon_finance_realization::{repository, service};

        let aggregate_index = "p902_ozon_finance_realization";
        let mut total_processed = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;

        // Получаем ID организации по названию
        let organization_id =
            match a002_organization::repository::get_by_description(&connection.organization)
                .await?
            {
                Some(org) => org.base.id.as_string(),
                None => {
                    let error_msg = format!(
                        "Организация '{}' не найдена в справочнике",
                        connection.organization
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

        // Finance Realization API НЕ ПОДДЕРЖИВАЕТ ПАГИНАЦИЮ - возвращает все данные за месяц сразу
        // Генерируем уникальный registrator_ref для всей сессии импорта
        let registrator_ref = Uuid::new_v4().to_string();

        let resp = self
            .api_client
            .fetch_finance_realization(connection, date_from, date_to, 10000, 0)
            .await?;

        if !resp.rows.is_empty() {
            use chrono::Datelike;
            let rows_count = resp.rows.len();
            tracing::info!(
                "Received {} finance realization rows from API for month {}-{}",
                rows_count,
                date_from.year(),
                date_from.month()
            );

            let currency_code = resp.header.currency_sys_name.clone();
            let report_number = resp.header.number.clone();
            let doc_date = resp.header.doc_date.clone();

            for row in resp.rows {
                let row_for_json = row.clone(); // Клонируем для сериализации в JSON
                let item = row.item;
                // Используем order.posting_number если есть, иначе offer_id
                let posting_number = if let Some(ref order) = row.order {
                    order.posting_number.clone()
                } else if !item.offer_id.is_empty() {
                    item.offer_id.clone()
                } else {
                    format!("REPORT-{}-{}", report_number, row.row_number)
                };
                let sku = item.sku.clone();

                self.progress_tracker.set_current_item(
                    session_id,
                    aggregate_index,
                    Some(format!("Finance {} - {}", posting_number, sku)),
                );

                // Парсим дату документа как accrual_date
                let accrual_date = chrono::NaiveDate::parse_from_str(&doc_date, "%Y-%m-%d")
                    .unwrap_or_else(|_| chrono::Utc::now().naive_utc().date());

                // Нет posting_ref, так как это финансовый отчет, не привязанный к конкретным отправлениям
                let posting_ref: Option<String> = None;

                // Обрабатываем delivery_commission (продажа)
                if let Some(ref dc) = row.delivery_commission {
                    let existing = repository::get_by_id(&posting_number, &sku, "delivery").await?;
                    let is_new = existing.is_none();

                    let quantity = dc.quantity;
                    let amount = dc.amount;
                    let commission_amount = Some(dc.commission);
                    let commission_percent = if dc.amount > 0.0 {
                        Some((dc.commission / dc.amount) * 100.0)
                    } else {
                        None
                    };

                    // Создаем запись проекции для продажи
                    let entry = repository::OzonFinanceRealizationEntry {
                        posting_number: posting_number.clone(),
                        sku: sku.clone(),
                        document_type: "OZON_Finance_Realization".to_string(),
                        registrator_ref: registrator_ref.clone(),
                        connection_mp_ref: connection.base.id.as_string(),
                        organization_ref: organization_id.clone(),
                        posting_ref: posting_ref.clone(),
                        accrual_date,
                        operation_date: None,
                        delivery_date: None,
                        delivery_schema: None,
                        delivery_region: None,
                        delivery_city: None,
                        quantity,
                        price: row.seller_price_per_instance,
                        amount,
                        commission_amount,
                        commission_percent,
                        services_amount: Some(dc.standard_fee),
                        payout_amount: Some(dc.total),
                        operation_type: "delivery".to_string(),
                        operation_type_name: Some("Доставка".to_string()),
                        is_return: false,
                        currency_code: Some(currency_code.clone()),
                        payload_version: 1,
                        extra: Some(serde_json::to_string(&row_for_json).unwrap_or_default()),
                    };

                    // Сохраняем запись
                    service::upsert_realization_row(entry).await?;

                    if is_new {
                        total_inserted += 1;
                    } else {
                        total_updated += 1;
                    }
                    total_processed += 1;
                }

                // Обрабатываем return_commission (возврат)
                if let Some(ref rc) = row.return_commission {
                    let existing = repository::get_by_id(&posting_number, &sku, "return").await?;
                    let is_new = existing.is_none();

                    // Для возвратов делаем суммы отрицательными
                    let quantity = -rc.quantity;
                    let amount = -rc.amount;
                    let commission_amount = Some(-rc.commission);
                    let commission_percent = if rc.amount > 0.0 {
                        Some((rc.commission / rc.amount) * 100.0)
                    } else {
                        None
                    };

                    // Создаем запись проекции для возврата
                    let entry = repository::OzonFinanceRealizationEntry {
                        posting_number: posting_number.clone(),
                        sku: sku.clone(),
                        document_type: "OZON_Finance_Realization".to_string(),
                        registrator_ref: registrator_ref.clone(),
                        connection_mp_ref: connection.base.id.as_string(),
                        organization_ref: organization_id.clone(),
                        posting_ref: posting_ref.clone(),
                        accrual_date,
                        operation_date: None,
                        delivery_date: None,
                        delivery_schema: None,
                        delivery_region: None,
                        delivery_city: None,
                        quantity,
                        price: row.seller_price_per_instance.map(|p| -p), // Отрицательная цена для возврата
                        amount,
                        commission_amount,
                        commission_percent,
                        services_amount: Some(-rc.standard_fee),
                        payout_amount: Some(-rc.total),
                        operation_type: "return".to_string(),
                        operation_type_name: Some("Возврат".to_string()),
                        is_return: true,
                        currency_code: Some(currency_code.clone()),
                        payload_version: 1,
                        extra: Some(serde_json::to_string(&row_for_json).unwrap_or_default()),
                    };

                    // Сохраняем запись
                    service::upsert_realization_row(entry).await?;

                    if is_new {
                        total_inserted += 1;
                    } else {
                        total_updated += 1;
                    }
                    total_processed += 1;
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
        }

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);
        tracing::info!(
            "OZON finance realization import completed: processed={}, inserted={}, updated={}",
            total_processed,
            total_inserted,
            total_updated
        );
        Ok(())
    }

    /// Импорт транзакций OZON через /v3/finance/transaction/list
    async fn import_ozon_transactions(
        &self,
        session_id: &str,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<()> {
        use crate::domain::a002_organization;
        use crate::domain::a014_ozon_transactions;
        use contracts::domain::a014_ozon_transactions::aggregate::{
            OzonTransactions, OzonTransactionsHeader, OzonTransactionsItem,
            OzonTransactionsPosting, OzonTransactionsService, OzonTransactionsSourceMeta,
        };

        let aggregate_index = "a014_ozon_transactions";
        let mut total_processed = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;

        // Получаем ID организации
        let organization_id =
            match a002_organization::repository::get_by_description(&connection.organization)
                .await?
            {
                Some(org) => org.base.id.as_string(),
                None => {
                    let error_msg = format!(
                        "Организация '{}' не найдена в справочнике",
                        connection.organization
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

        // Пагинация по страницам
        let page_size = 1000; // Максимум для OZON API
        let mut current_page = 1;

        loop {
            let resp = self
                .api_client
                .fetch_transactions_list(connection, date_from, date_to, current_page, page_size)
                .await?;

            let operations_count = resp.result.operations.len();
            if operations_count == 0 {
                break;
            }

            tracing::info!(
                "Received {} transactions from API (page {}/{})",
                operations_count,
                current_page,
                resp.result.page_count
            );

            for operation in resp.result.operations {
                let code = format!("OZON-TXN-{}", operation.operation_id);
                let description = format!(
                    "{} - {}",
                    operation.operation_type_name, operation.posting.posting_number
                );

                // Собираем header
                let header = OzonTransactionsHeader {
                    operation_id: operation.operation_id,
                    operation_type: operation.operation_type.clone(),
                    operation_date: operation.operation_date.clone(),
                    operation_type_name: operation.operation_type_name.clone(),
                    delivery_charge: operation.delivery_charge,
                    return_delivery_charge: operation.return_delivery_charge,
                    accruals_for_sale: operation.accruals_for_sale,
                    sale_commission: operation.sale_commission,
                    amount: operation.amount,
                    transaction_type: operation.transaction_type.clone(),
                    connection_id: connection.base.id.as_string(),
                    organization_id: organization_id.clone(),
                    marketplace_id: connection.marketplace_id.clone(),
                };

                // Собираем posting
                let posting = OzonTransactionsPosting {
                    delivery_schema: operation.posting.delivery_schema.clone(),
                    order_date: operation.posting.order_date.clone(),
                    posting_number: operation.posting.posting_number.clone(),
                    warehouse_id: operation.posting.warehouse_id,
                };

                // Собираем items
                let items: Vec<OzonTransactionsItem> = operation
                    .items
                    .into_iter()
                    .map(|item| OzonTransactionsItem {
                        name: item.name,
                        sku: item.sku,
                    })
                    .collect();

                // Собираем services
                let services: Vec<OzonTransactionsService> = operation
                    .services
                    .into_iter()
                    .map(|service| OzonTransactionsService {
                        name: service.name,
                        price: service.price,
                    })
                    .collect();

                // Source meta
                let source_meta = OzonTransactionsSourceMeta {
                    raw_payload_ref: format!("ozon_txn_{}", operation.operation_id),
                    fetched_at: chrono::Utc::now(),
                    document_version: 1,
                };

                // Создаем агрегат
                let aggregate = OzonTransactions::new_for_insert(
                    code,
                    description,
                    header,
                    posting,
                    items,
                    services,
                    source_meta,
                    false, // is_posted = false по умолчанию
                );

                // Upsert по operation_id
                match a014_ozon_transactions::repository::get_by_operation_id(
                    aggregate.header.operation_id,
                )
                .await?
                {
                    Some(_existing) => {
                        // Обновляем существующую транзакцию
                        a014_ozon_transactions::repository::upsert_by_operation_id(&aggregate)
                            .await?;
                        total_updated += 1;
                    }
                    None => {
                        // Вставляем новую транзакцию
                        a014_ozon_transactions::repository::upsert_by_operation_id(&aggregate)
                            .await?;
                        total_inserted += 1;
                    }
                }

                total_processed += 1;

                // Обновляем прогресс
                self.progress_tracker.update_aggregate(
                    session_id,
                    aggregate_index,
                    total_processed,
                    Some(resp.result.row_count),
                    total_inserted,
                    total_updated,
                );
            }

            // Проверяем, есть ли еще страницы
            if current_page >= resp.result.page_count {
                break;
            }

            current_page += 1;
        }

        tracing::info!(
            "OZON Transactions import completed: processed={}, inserted={}, updated={}",
            total_processed,
            total_inserted,
            total_updated
        );

        self.progress_tracker
            .complete_aggregate(session_id, aggregate_index);

        Ok(())
    }
}

/// Normalize OZON posting status
fn normalize_ozon_status(status: &str) -> String {
    match status.to_uppercase().as_str() {
        "DELIVERED" => "DELIVERED".to_string(),
        "CANCELLED" | "CANCELED" => "CANCELLED".to_string(),
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
