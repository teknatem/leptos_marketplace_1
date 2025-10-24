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
                connection.base.id.as_string(),
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
                            let new = contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct::new_for_insert(
                                key.clone(),
                                key.clone(),
                                connection.marketplace_id.clone(),
                                connection.base.id.as_string(),
                                key.clone(),
                                None,
                                key.clone(),
                                key.clone(),
                                None,
                                None,
                                None,
                                None,
                                None,
                                Some(chrono::Utc::now()),
                                None,
                                None,
                                Some("auto-created from finance operation".to_string()),
                            );
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
            tracing::info!("Received {} returns from API (last_id={})", returns_count, last_id);

            // Сохраняем last_id для следующего запроса перед перемещением вектора
            let new_last_id = resp.returns.last().map(|r| r.id).unwrap_or(last_id);

            for return_item in resp.returns {
                let return_id_str = return_item.id.to_string();
                let return_reason = return_item.return_reason_name.as_deref().unwrap_or("Unknown");
                let return_type = return_item.return_type.as_deref().unwrap_or("Unknown");
                let order_id_str = return_item.order_id.map(|id| id.to_string()).unwrap_or_default();
                let order_number = return_item.order_number.as_deref().unwrap_or("");
                let posting_number = return_item.posting_number.as_deref().unwrap_or("");
                let clearing_id_str = return_item.clearing_id.map(|id| id.to_string());
                let return_clearing_id_str = return_item.return_clearing_id.map(|id| id.to_string());

                // Парсим дату возврата из logistic.return_date (ISO 8601 формат: "2022-01-19T19:55:35.433Z")
                let return_date = return_item.logistic
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
                let has_product = return_item.product.as_ref()
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
                    return_id_str, has_product, return_item.product
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
                        return_id_str, sku_str, product_name, price, quantity
                    );

                    let display_name = format!("{} - {} - {}", return_id_str, sku_str, product_name);
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
}

impl Clone for ImportExecutor {
    fn clone(&self) -> Self {
        Self {
            api_client: Arc::clone(&self.api_client),
            progress_tracker: Arc::clone(&self.progress_tracker),
        }
    }
}
