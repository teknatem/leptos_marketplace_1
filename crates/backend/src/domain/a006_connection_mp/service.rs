use super::repository;
use chrono::Utc;
use contracts::domain::a006_connection_mp::aggregate::{
    ConnectionMP, ConnectionMPDto, ConnectionTestResult,
};
use uuid::Uuid;

/// Создание нового подключения к маркетплейсу
pub async fn create(dto: ConnectionMPDto) -> anyhow::Result<Uuid> {
    let code = dto
        .code
        .clone()
        .unwrap_or_else(|| format!("MP-{}", Uuid::new_v4()));
    let mut aggregate = ConnectionMP::new_for_insert(
        code,
        dto.description,
        dto.marketplace_id,
        dto.organization_ref,
        dto.api_key,
        dto.comment,
    );

    // Обновляем остальные поля
    aggregate.supplier_id = dto.supplier_id;
    aggregate.application_id = dto.application_id;
    aggregate.is_used = dto.is_used;
    aggregate.business_account_id = dto.business_account_id;
    aggregate.api_key_stats = dto.api_key_stats;
    aggregate.test_mode = dto.test_mode;
    aggregate.authorization_type = dto.authorization_type;

    // Валидация
    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    // Before write
    aggregate.before_write();

    // Сохранение через repository
    repository::insert(&aggregate).await
}

/// Обновление существующего подключения
pub async fn update(dto: ConnectionMPDto) -> anyhow::Result<()> {
    let id = dto
        .id
        .as_ref()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| anyhow::anyhow!("Invalid ID"))?;

    let mut aggregate = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Not found"))?;

    aggregate.update(&dto);

    // Валидация
    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    // Before write
    aggregate.before_write();

    // Сохранение
    repository::update(&aggregate).await
}

/// Мягкое удаление подключения
pub async fn delete(id: Uuid) -> anyhow::Result<bool> {
    repository::soft_delete(id).await
}

/// Получение подключения по ID
pub async fn get_by_id(id: Uuid) -> anyhow::Result<Option<ConnectionMP>> {
    repository::get_by_id(id).await
}

/// Получение списка всех подключений
pub async fn list_all() -> anyhow::Result<Vec<ConnectionMP>> {
    repository::list_all().await
}

/// Получение информации о продавце через WB API /api/v1/seller-info
pub async fn seller_info(dto: ConnectionMPDto) -> anyhow::Result<ConnectionTestResult> {
    let start = std::time::Instant::now();

    if dto.api_key.trim().is_empty() {
        return Ok(ConnectionTestResult {
            success: false,
            message: "API Key не может быть пустым".into(),
            duration_ms: 0,
            tested_at: Utc::now(),
            details: None,
        });
    }

    if dto.marketplace_id.trim().is_empty() {
        return Ok(ConnectionTestResult {
            success: false,
            message: "Маркетплейс должен быть выбран".into(),
            duration_ms: 0,
            tested_at: Utc::now(),
            details: None,
        });
    }

    let marketplace_uuid = match uuid::Uuid::parse_str(&dto.marketplace_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(ConnectionTestResult {
                success: false,
                message: "Некорректный ID маркетплейса".into(),
                duration_ms: 0,
                tested_at: Utc::now(),
                details: None,
            });
        }
    };

    let marketplace =
        match crate::domain::a005_marketplace::repository::get_by_id(marketplace_uuid).await? {
            Some(mp) => mp,
            None => {
                return Ok(ConnectionTestResult {
                    success: false,
                    message: "Маркетплейс не найден в базе данных".into(),
                    duration_ms: 0,
                    tested_at: Utc::now(),
                    details: None,
                });
            }
        };

    let mp_code = marketplace.base.code.to_lowercase();
    if !mp_code.contains("wildberries") && !mp_code.contains("wb") {
        let duration = start.elapsed();
        return Ok(ConnectionTestResult {
            success: false,
            message: format!(
                "Информация о продавце поддерживается только для Wildberries (маркетплейс: {})",
                marketplace.base.description
            ),
            duration_ms: duration.as_millis() as u64,
            tested_at: Utc::now(),
            details: None,
        });
    }

    let api_key = dto.api_key.trim().replace(['\n', '\r', '\t'], "");

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return Ok(ConnectionTestResult {
                success: false,
                message: "Ошибка создания HTTP клиента".into(),
                duration_ms: 0,
                tested_at: Utc::now(),
                details: Some(format!("{}", e)),
            });
        }
    };

    let url = "https://common-api.wildberries.ru/api/v1/seller-info";

    let response = match client
        .get(url)
        .header("Authorization", api_key.as_str())
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            let duration = start.elapsed();
            return Ok(ConnectionTestResult {
                success: false,
                message: format!("Ошибка запроса к WB seller-info: {}", e),
                duration_ms: duration.as_millis() as u64,
                tested_at: Utc::now(),
                details: None,
            });
        }
    };

    let duration = start.elapsed();
    let status = response.status();

    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Ok(ConnectionTestResult {
            success: false,
            message: format!("WB API вернул ошибку (HTTP {})", status.as_u16()),
            duration_ms: duration.as_millis() as u64,
            tested_at: Utc::now(),
            details: Some(error_text),
        });
    }

    match response.text().await {
        Ok(text) => {
            let pretty = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| serde_json::to_string_pretty(&v).ok())
                .unwrap_or(text);

            Ok(ConnectionTestResult {
                success: true,
                message: "Информация о продавце получена".into(),
                duration_ms: duration.as_millis() as u64,
                tested_at: Utc::now(),
                details: Some(pretty),
            })
        }
        Err(e) => Ok(ConnectionTestResult {
            success: false,
            message: "Ошибка чтения ответа от WB API".into(),
            duration_ms: duration.as_millis() as u64,
            tested_at: Utc::now(),
            details: Some(format!("{}", e)),
        }),
    }
}

/// Тестирование подключения к маркетплейсу
pub async fn test_connection(dto: ConnectionMPDto) -> anyhow::Result<ConnectionTestResult> {
    let start = std::time::Instant::now();

    // Валидация базовых данных
    if dto.api_key.trim().is_empty() {
        return Ok(ConnectionTestResult {
            success: false,
            message: "API Key не может быть пустым".into(),
            duration_ms: 0,
            tested_at: Utc::now(),
            details: None,
        });
    }

    if dto.marketplace_id.trim().is_empty() {
        return Ok(ConnectionTestResult {
            success: false,
            message: "Маркетплейс должен быть выбран".into(),
            duration_ms: 0,
            tested_at: Utc::now(),
            details: None,
        });
    }

    // Получаем информацию о маркетплейсе из БД
    let marketplace_uuid = match Uuid::parse_str(&dto.marketplace_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(ConnectionTestResult {
                success: false,
                message: "Некорректный ID маркетплейса".into(),
                duration_ms: 0,
                tested_at: Utc::now(),
                details: None,
            });
        }
    };

    let marketplace =
        match crate::domain::a005_marketplace::repository::get_by_id(marketplace_uuid).await? {
            Some(mp) => mp,
            None => {
                return Ok(ConnectionTestResult {
                    success: false,
                    message: "Маркетплейс не найден в базе данных".into(),
                    duration_ms: 0,
                    tested_at: Utc::now(),
                    details: None,
                });
            }
        };

    // Тестируем подключение через соответствующий клиент
    let test_result =
        crate::shared::marketplaces::test_marketplace_connection(&marketplace.base.code, &dto)
            .await;

    let duration = start.elapsed();

    Ok(ConnectionTestResult {
        success: test_result.success,
        message: test_result.message,
        duration_ms: duration.as_millis() as u64,
        tested_at: Utc::now(),
        details: test_result.details,
    })
}
